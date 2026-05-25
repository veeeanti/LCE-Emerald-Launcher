use crate::lce_bridge::lce_packets::*;
use crate::lce_bridge::java_protocol::*;
use crate::lce_bridge::java_session::JavaSession;
use crate::lce_bridge::chunk::{translate_java_chunk, CachedLceChunk};
use crate::lce_bridge::config::BridgeConfig;
use crate::lce_bridge::crafting::InventoryManager;
use std::collections::{HashMap, VecDeque};
use uuid::Uuid;

pub struct LceBridgeSession {
    pub config: BridgeConfig,
    pub lce_tx: VecDeque<LcePacket>,
    pub java_session: Option<JavaSession>,
    pub player_eid: i32,
    pub tracked_entities: HashMap<i32, TrackedEntity>,
    pub chunk_cache: HashMap<(i32, i32), CachedLceChunk>,
    pub inventory: InventoryManager,
    pub render_distance: i32,
    pub teleport_id: i32,
    pub health: f32,
    pub food: i32,
    pub saturation: f32,
    pub game_time: i64,
    pub day_time: i64,
    pub has_joined: bool,
    pub player_name: String,
    pub pos_x: f64,
    pub pos_y: f64,
    pub pos_z: f64,
    pub yaw: f32,
    pub pitch: f32,
    pub container_id_map: HashMap<i32, u8>,
    pub lce_container_id: u8,
}

pub struct TrackedEntity {
    pub entity_id: i32,
    pub kind: TrackedEntityKind,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub yaw: u8,
    pub pitch: u8,
}

pub enum TrackedEntityKind {
    Object,
    Mob(u8),
    Item,
    Player,
}

impl LceBridgeSession {
    pub fn new(config: BridgeConfig) -> Self {
        Self {
            config,
            lce_tx: VecDeque::new(),
            java_session: None,
            player_eid: 0,
            tracked_entities: HashMap::new(),
            chunk_cache: HashMap::new(),
            inventory: InventoryManager::new(),
            render_distance: 8,
            teleport_id: 0,
            health: 20.0,
            food: 20,
            saturation: 5.0,
            game_time: 0,
            day_time: 1000,
            has_joined: false,
            player_name: String::new(),
            pos_x: 0.0, pos_y: 64.0, pos_z: 0.0,
            yaw: 0.0, pitch: 0.0,
            container_id_map: HashMap::new(),
            lce_container_id: 1,
        }
    }

    pub fn send_lce(&mut self, packet: LcePacket) {
        self.lce_tx.push_back(packet);
    }

    pub fn on_connected(&mut self) {
        eprintln!("[LCEBridge] on_connected");
        let prelogin = LcePacket::PreLogin(PreLoginPacket {
            net_version: 560,
            player_name: "Player".to_string(),
            offline_xuid: 0,
            online_xuid: 0,
        });
        self.send_lce(prelogin);
    }

    pub async fn handle_lce_packet(&mut self, packet: LcePacket) {
        match packet {
            LcePacket::PreLogin(p) => {
                self.on_lce_prelogin(p).await;
            }
            LcePacket::Login(p) => {
                self.on_lce_login(p).await;
            }
            LcePacket::MovePlayer(p) => {
                self.pos_x = p.x;
                self.pos_y = p.y;
                self.pos_z = p.z;
                self.yaw = p.yaw;
                self.pitch = p.pitch;
                if let Some(ref mut js) = self.java_session {
                    let _ = js.send(JavaPacket::AcceptTeleportation(AcceptTeleportationPacket {
                        teleport_id: self.teleport_id,
                    })).await;
                }
            }
            LcePacket::Chat(p) => {
                if let Some(ref mut js) = self.java_session {
                    let msg = p.string_args.first().cloned().unwrap_or_default();
                    let _ = js.send(JavaPacket::ChatCommand(ChatCommandPacket { command: msg }));
                }
            }
            LcePacket::PlayerAction(p) => {
                if let Some(ref mut js) = self.java_session {
                    if p.action == 1 {
                        let _x = p.x as f64;
                        let _y = p.y as f64;
                        let _z = p.z as f64;
                        let _face = p.face;
                        let _ = js.send(JavaPacket::Unknown { id: 0x36, data: VarInt::write(0) });
                    }
                }
            }
            LcePacket::Interact(p) => {
                if let Some(ref mut js) = self.java_session {
                    let _ = js.send(JavaPacket::Unknown {
                        id: 0x11,
                        data: pack_varint(p.target),
                    }).await;
                }
            }
            LcePacket::ContainerClick(p) => {
                if let Some(ref mut js) = self.java_session {
                    let mut data = Vec::new();
                    VarInt::write_to(p.container_id as i32, &mut data);
                    VarInt::write_to(0, &mut data);
                    VarInt::write_to(p.slot_num as i32, &mut data);
                    data.push(p.click_type);
                    data.push(p.button_num);
                    VarInt::write_to(0, &mut data);
                    let _ = js.send(JavaPacket::Unknown { id: 0x0E, data }).await;
                }
            }
            LcePacket::ContainerClose(p) => {
                if let Some(ref mut js) = self.java_session {
                    let _ = js.send(JavaPacket::ContainerCloseS2C(
                        ContainerCloseS2CPacket { container_id: p.container_id as i32 }
                    )).await;
                }
            }
            LcePacket::SetCarriedItem(p) => {
                if let Some(ref mut js) = self.java_session {
                    let _ = js.send(JavaPacket::Unknown {
                        id: 0x28,
                        data: vec![p.slot],
                    }).await;
                }
            }
            LcePacket::ClientCommand(_p) => {
                //neo: do NOT think of sending here. it causes premature FinishConfiguration
            }
            LcePacket::RespawnRequest(_) => {}
            _ => {}
        }
    }

    async fn on_lce_prelogin(&mut self, p: PreLoginPacket) {
        eprintln!("[LCEBridge] PreLogin from client: player={}, net_ver={}, offline_xuid={}, online_xuid={}", p.player_name, p.net_version, p.offline_xuid, p.online_xuid);
        self.player_name = p.player_name.clone();
        eprintln!("[LCEBridge] Connecting to Java server at {}:{}", self.config.remote.host, self.config.remote.port);
        match JavaSession::connect(&self.config.remote).await {
            Ok(mut js) => {
                eprintln!("[LCEBridge] Connected to Java server, sending Handshake+LoginStart");
                let handshake = JavaPacket::Handshake(HandshakePacket {
                    protocol_version: self.config.remote.protocol_version,
                    server_address: self.config.remote.host.clone(),
                    server_port: self.config.remote.port,
                    next_state: 2,
                });
                let _ = js.send(handshake).await;
                let login_start = JavaPacket::LoginStart(LoginStartPacket {
                    username: p.player_name.clone(),
                    profile_id: Uuid::nil(),
                    protocol_version: self.config.remote.protocol_version,
                });
                let _ = js.send(login_start).await;
                eprintln!("[LCEBridge] Handshake+LoginStart sent to Java server");
                self.java_session = Some(js);
                self.has_joined = true;
            }
            Err(e) => {
                eprintln!("[LCEBridge] Failed to connect to Java server: {}", e);
                self.send_lce(LcePacket::Disconnect(DisconnectPacket { reason: 1 }));
            }
        }
    }

    async fn on_lce_login(&mut self, p: LoginPacket) {
        if self.java_session.is_none() {
            eprintln!("[LCEBridge] Client sent Login without PreLogin, connecting to Java server now");
            match JavaSession::connect(&self.config.remote).await {
                Ok(mut js) => {
                    let handshake = JavaPacket::Handshake(HandshakePacket {
                        protocol_version: self.config.remote.protocol_version,
                        server_address: self.config.remote.host.clone(),
                        server_port: self.config.remote.port,
                        next_state: 2,
                    });
                    let _ = js.send(handshake).await;
                    let login_start = JavaPacket::LoginStart(LoginStartPacket {
                        username: p.username.clone(),
                        profile_id: Uuid::nil(),
                        protocol_version: self.config.remote.protocol_version,
                    });
                    let _ = js.send(login_start).await;
                    self.java_session = Some(js);
                    self.has_joined = true;
                }
                Err(e) => {
                    eprintln!("[LCEBridge] Failed to connect to Java server: {}", e);
                }
            }
        }
        self.send_lce_login_response(&p.username).await;
        self.player_name.clear();
    }

    async fn send_lce_login_response(&mut self, username: &str) {
        self.send_lce(LcePacket::Login(LoginPacket {
            protocol_version: 560,
            username: username.to_string(),
            map_seed: 0,
            game_type: 0,
            world_name: "bridge".to_string(),
            dimension: 0,
            difficulty: 2,
            max_players: 1,
            world_width: 0,
            world_length: 0,
        }));
        self.send_lce(LcePacket::SetSpawnPosition(SetSpawnPositionPacket {
            x: 0, y: 64, z: 0,
        }));
        self.send_lce(LcePacket::GameEvent(GameEventPacket {
            reason: 3, param: 0.0,
        }));
        self.send_lce(LcePacket::SetTime(SetTimePacket {
            game_time: 0, day_time: 1000,
        }));
        self.send_lce(LcePacket::PlayerAbilities(PlayerAbilitiesPacket {
            flags: 0x04, fly_speed: 0.05, walk_speed: 0.1,
        }));
        self.send_lce(LcePacket::SetHealth(SetHealthPacket {
            health: 20.0, food: 20, saturation: 5.0, damage_source: 0,
        }));
        let _ = self.spawn_player().await;
    }

    async fn spawn_player(&mut self) {
        let (cx, cz) = (0i32, 0i32);
        self.send_lce(LcePacket::ChunkVisibilityArea(ChunkVisibilityAreaPacket {
            min_cx: cx - self.render_distance,
            max_cx: cx + self.render_distance,
            min_cz: cz - self.render_distance,
            max_cz: cz + self.render_distance,
        }));
        for dx in -self.render_distance..=self.render_distance {
            for dz in -self.render_distance..=self.render_distance {
                self.send_lce(LcePacket::ChunkVisibility(ChunkVisibilityPacket {
                    chunk_x: cx + dx,
                    chunk_z: cz + dz,
                    visible: true,
                }));
            }
        }
        let builder = crate::lce_bridge::chunk::LceChunkBuilder::new();
        let raw = builder.build_raw_data();
        let compressed = crate::lce_bridge::chunk::compress_rle_zlib(&raw);
        self.send_lce(LcePacket::BlockRegionUpdate(BlockRegionUpdatePacket {
            x: 0, y: -64, z: 0, xs: 16, ys: 24, zs: 16,
            level_idx: 0, is_full_chunk: true, compressed_data: compressed,
        }));
    }

    pub async fn handle_java_packet(&mut self, packet: JavaPacket) {
        match packet {
            JavaPacket::KeepAliveS2C(p) => {
                if let Some(ref mut js) = self.java_session {
                    let _ = js.send(JavaPacket::KeepAliveC2S(KeepAliveC2SPacket { id: p.id })).await;
                }
            }
            JavaPacket::LoginPlay(p) => {
                self.player_eid = p.entity_id;
                self.render_distance = p.view_distance.min(self.render_distance);
                if !self.player_name.is_empty() {
                    let name = self.player_name.clone();
                    self.send_lce_login_response(&name).await;
                    self.player_name.clear();
                }
            }
            JavaPacket::SynchronizePlayerPosition(p) => {
                self.teleport_id = p.teleport_id;
                self.pos_x = p.x;
                self.pos_y = p.y;
                self.pos_z = p.z;
                self.yaw = p.yaw;
                self.pitch = p.pitch;
                if let Some(ref mut js) = self.java_session {
                    let _ = js.send(JavaPacket::ClientInformation(ClientInformationPacket {
                        locale: "en_GB".to_string(),
                        view_distance: self.render_distance as u8,
                        chat_mode: 0,
                        chat_colors: true,
                        displayed_skin_parts: 0x7F,
                        main_hand: 1,
                        enable_text_filtering: false,
                        allow_listing: false,
                        particle_status: 0,
                    })).await;
                    let _ = js.send(JavaPacket::AcceptTeleportation(AcceptTeleportationPacket {
                        teleport_id: p.teleport_id,
                    })).await;
                }
            }
            JavaPacket::PlayerPosition(p) => {
                self.teleport_id = p.teleport_id;
                if let Some(ref mut js) = self.java_session {
                    let _ = js.send(JavaPacket::ClientInformation(ClientInformationPacket {
                        locale: "en_GB".to_string(),
                        view_distance: self.render_distance as u8,
                        chat_mode: 0,
                        chat_colors: true,
                        displayed_skin_parts: 0x7F,
                        main_hand: 1,
                        enable_text_filtering: false,
                        allow_listing: false,
                        particle_status: 0,
                    })).await;
                    let _ = js.send(JavaPacket::AcceptTeleportation(AcceptTeleportationPacket {
                        teleport_id: p.teleport_id,
                    })).await;
                }
                self.send_lce(LcePacket::SetSpawnPosition(SetSpawnPositionPacket {
                    x: p.x as i32, y: p.y as i32, z: p.z as i32,
                }));
            }
            JavaPacket::ChunkData(p) => {
                let (_builder, compressed) = translate_java_chunk(
                    p.chunk_x, p.chunk_z, &p.chunk_data,
                );
                self.chunk_cache.insert(
                    (p.chunk_x, p.chunk_z),
                    CachedLceChunk::new(p.chunk_x, p.chunk_z),
                );
                self.send_lce(LcePacket::ChunkVisibility(ChunkVisibilityPacket {
                    chunk_x: p.chunk_x,
                    chunk_z: p.chunk_z,
                    visible: true,
                }));
                self.send_lce(LcePacket::BlockRegionUpdate(BlockRegionUpdatePacket {
                    x: p.chunk_x * 16, y: -64, z: p.chunk_z * 16,
                    xs: 16, ys: 24, zs: 16,
                    level_idx: 0, is_full_chunk: true,
                    compressed_data: compressed,
                }));
            }
            JavaPacket::ChunkBatchFinished => {}
            JavaPacket::SetContainerContent(p) => {
                let mut items = Vec::new();
                for slot in &p.slots {
                    if slot.present {
                        items.push(LceItemStack {
                            id: slot.item_id as i16,
                            count: slot.count,
                            damage: 0,
                        });
                    } else {
                        items.push(LceItemStack::empty());
                    }
                }
                let cid = *self.container_id_map.get(&p.container_id).unwrap_or(&1);
                self.send_lce(LcePacket::ContainerSetContent(ContainerSetContentPacket {
                    container_id: cid,
                    items,
                }));
            }
            JavaPacket::SetContainerSlot(p) => {
                let item = if p.item.present {
                    LceItemStack {
                        id: p.item.item_id as i16,
                        count: p.item.count,
                        damage: 0,
                    }
                } else {
                    LceItemStack::empty()
                };
                let cid = *self.container_id_map.get(&p.container_id).unwrap_or(&1);
                self.send_lce(LcePacket::ContainerSetSlot(ContainerSetSlotPacket {
                    container_id: cid,
                    slot: p.slot,
                    item,
                }));
            }
            JavaPacket::OpenScreen(p) => {
                let lce_type = match p.screen_id {
                    2 | 3 | 4 | 5 => 16,
                    7 => 17,
                    8 => 15,
                    9 | 10 | 11 | 12 | 13 | 14 => 14,
                    _ => 14,
                };
                let cid = self.lce_container_id;
                self.lce_container_id += 1;
                self.container_id_map.insert(p.container_id, cid);
                self.send_lce(LcePacket::ContainerOpen(ContainerOpenPacket {
                    container_id: cid,
                    container_type: lce_type,
                    size: 0,
                    custom_name: String::new(),
                    title: String::new(),
                    entity_id: -1,
                }));
            }
            JavaPacket::SystemChat(p) => {
                let text = extract_chat_text(&p.content_json);
                self.send_lce(LcePacket::Chat(ChatPacket::set_message(&text)));
            }
            JavaPacket::PlayerChat(p) => {
                let text = extract_chat_text(&p.content_json);
                self.send_lce(LcePacket::Chat(ChatPacket::set_message(&text)));
            }
            JavaPacket::SetHealth(p) => {
                self.health = p.health;
                self.food = p.food;
                self.saturation = p.saturation;
                self.send_lce(LcePacket::SetHealth(SetHealthPacket {
                    health: p.health, food: p.food, saturation: p.saturation, damage_source: 0,
                }));
            }
            JavaPacket::SetTime(p) => {
                self.game_time = p.world_age;
                self.day_time = p.day_time;
                self.send_lce(LcePacket::SetTime(SetTimePacket {
                    game_time: p.world_age, day_time: p.day_time,
                }));
            }
            JavaPacket::AddEntity(p) => {
                let lce_type = map_java_object_type(p.entity_type);
                let fx = (p.x * 32.0) as i32;
                let fy = (p.y * 32.0) as i32;
                let fz = (p.z * 32.0) as i32;
                self.tracked_entities.insert(p.entity_id, TrackedEntity {
                    entity_id: p.entity_id,
                    kind: TrackedEntityKind::Object,
                    x: p.x, y: p.y, z: p.z,
                    yaw: p.yaw, pitch: p.pitch,
                });
                self.send_lce(LcePacket::AddEntity(AddEntityPacket {
                    entity_id: p.entity_id,
                    entity_type: lce_type,
                    x: fx, y: fy, z: fz,
                    yaw: p.yaw, pitch: p.pitch,
                    data: p.data,
                    motion_x: p.vel_x, motion_y: p.vel_y, motion_z: p.vel_z,
                }));
            }
            JavaPacket::AddMob(p) => {
                let lce_type = map_java_mob_type(p.entity_type);
                let fx = (p.x * 32.0) as i32;
                let fy = (p.y * 32.0) as i32;
                let fz = (p.z * 32.0) as i32;
                self.tracked_entities.insert(p.entity_id, TrackedEntity {
                    entity_id: p.entity_id,
                    kind: TrackedEntityKind::Mob(lce_type),
                    x: p.x, y: p.y, z: p.z,
                    yaw: p.yaw, pitch: p.pitch,
                });
                self.send_lce(LcePacket::AddMob(AddMobPacket {
                    entity_id: p.entity_id,
                    entity_type: lce_type,
                    x: fx, y: fy, z: fz,
                    yaw: p.yaw, pitch: p.pitch, head_yaw: p.head_yaw,
                    motion_x: p.vel_x, motion_y: p.vel_y, motion_z: p.vel_z,
                    metadata: Vec::new(),
                }));
            }
            JavaPacket::AddPlayer(p) => {
                let fx = (p.x * 32.0) as i32;
                let fy = (p.y * 32.0) as i32;
                let fz = (p.z * 32.0) as i32;
                self.send_lce(LcePacket::AddPlayer(AddPlayerPacket {
                    entity_id: p.entity_id,
                    name: String::new(),
                    x: fx, y: fy, z: fz,
                    yaw: p.yaw, pitch: p.pitch, head_yaw: p.yaw,
                    carried_item: 0,
                    offline_xuid: p.entity_id as i64,
                    online_xuid: 0,
                    player_index: 0,
                    skin_id: String::new(),
                    cape_id: String::new(),
                    game_privileges: 0xFFFFFFFF,
                    metadata: Vec::new(),
                }));
            }
            JavaPacket::RemoveEntities(p) => {
                for id in &p.entity_ids {
                    self.tracked_entities.remove(id);
                }
                self.send_lce(LcePacket::RemoveEntities(RemoveEntitiesPacket {
                    entity_ids: p.entity_ids,
                }));
            }
            JavaPacket::SetEntityMotion(p) => {
                self.send_lce(LcePacket::SetEntityMotion(SetEntityMotionPacket {
                    entity_id: p.entity_id,
                    xa: p.xa, ya: p.ya, za: p.za,
                }));
            }
            JavaPacket::TeleportEntity(p) => {
                let fx = (p.x * 32.0) as i32;
                let fy = (p.y * 32.0) as i32;
                let fz = (p.z * 32.0) as i32;
                if let Some(e) = self.tracked_entities.get_mut(&p.entity_id) {
                    e.x = p.x; e.y = p.y; e.z = p.z;
                }
                self.send_lce(LcePacket::TeleportEntity(TeleportEntityPacket {
                    entity_id: p.entity_id,
                    x: fx, y: fy, z: fz,
                    yaw: p.yaw, pitch: p.pitch,
                }));
            }
            JavaPacket::SetHeadRotation(p) => {
                self.send_lce(LcePacket::RotateHead(RotateHeadPacket {
                    entity_id: p.entity_id,
                    y_head_rot: p.head_yaw,
                }));
            }
            JavaPacket::SetEntityData(_) => {}
            JavaPacket::GameEventS2C(p) => {
                self.send_lce(LcePacket::GameEvent(GameEventPacket {
                    reason: p.reason, param: p.param,
                }));
            }
            JavaPacket::SetDefaultSpawnPosition(p) => {
                self.send_lce(LcePacket::SetSpawnPosition(SetSpawnPositionPacket {
                    x: p.x, y: p.y, z: p.z,
                }));
            }
            JavaPacket::ContainerCloseS2C(p) => {
                self.send_lce(LcePacket::ContainerClose(ContainerClosePacket {
                    container_id: p.container_id as u8,
                }));
            }
            JavaPacket::ContainerSetDataS2C(p) => {
                self.send_lce(LcePacket::ContainerSetData(ContainerSetDataPacket {
                    container_id: p.container_id as u8,
                    id: p.key as i16,
                    value: p.value as i16,
                }));
            }
            JavaPacket::ContainerAckS2C(p) => {
                self.send_lce(LcePacket::ContainerAck(ContainerAckPacket {
                    container_id: p.container_id,
                    uid: p.uid,
                    accepted: p.accepted,
                }));
            }
            JavaPacket::SoundEffect(_) => {}
            JavaPacket::RegistryData(_) => {}
            _ => {}
        }
    }
}

fn map_java_mob_type(java_type: i32) -> u8 {
    match java_type {
        0 => 60, 1 => 61, 2 => 62, 3 => 63, 4 => 64, 5 => 65, 6 => 66,
        7 => 67, 8 => 68, 10 => 69, 11 => 70, 12 => 71, 13 => 72,
        14 => 73, 15 => 74, 16 => 75, 17 => 76, 18 => 77, 19 => 78,
        20 => 79, 21 => 80, 22 => 81, 23 => 82, 24 => 83, 25 => 84,
        26 => 85, 27 => 86, 28 => 87, 29 => 88, 30 => 89, 31 => 90,
        32 => 91, 33 => 92, 34 => 93, 35 => 94, 36 => 95, 37 => 96,
        38 => 97, 39 => 98, 40 => 99,
        _ => 60,
    }
}

fn map_java_object_type(java_type: i32) -> u8 {
    match java_type {
        0 => 50, 1 => 51, 2 => 52, 3 => 53, 4 => 54, 5 => 55,
        6 => 56, 7 => 57, 8 => 58, 9 => 59, 10 => 60, 11 => 61,
        _ => 50,
    }
}

fn extract_chat_text(nbt_data: &[u8]) -> String {
    if nbt_data.is_empty() || nbt_data[0] == 0 {
        return String::new();
    }
    let text = String::from_utf8_lossy(nbt_data);
    if let Some(start) = text.find("text\":\"") {
        let rest = &text[start + 7..];
        if let Some(end) = rest.find('"') {
            return rest[..end].to_string();
        }
    }
    String::new()
}

fn pack_varint(val: i32) -> Vec<u8> {
    VarInt::write(val)
}

use crate::lce_bridge::util::VarInt;
