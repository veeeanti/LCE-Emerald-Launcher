use crate::lce_bridge::lce_packets::*;
use crate::lce_bridge::util::*;
use crate::lce_bridge::bridge::LceBridgeSession;
use crate::lce_bridge::config::BridgeConfig;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;

pub fn decode_lce_packet(id: u8, data: &[u8]) -> std::io::Result<LcePacket> {
    let mut r = LceReader::new(data);
    Ok(match id {
        0 => LcePacket::KeepAlive(KeepAlivePacket {
            keep_alive_id: r.read_i32()?,
        }),
        1 => LcePacket::Login(LoginPacket {
            protocol_version: r.read_i32()?,
            username: r.read_string_utf16()?,
            map_seed: r.read_u64()?,
            game_type: r.read_i32()?,
            world_name: r.read_string_utf16()?,
            dimension: r.read_i32()?,
            difficulty: r.read_i32()?,
            max_players: r.read_u8()?,
            world_width: r.read_i32()?,
            world_length: r.read_i32()?,
        }),
        2 => {
            let net_version = r.read_i16()?;
            let player_name = r.read_string_utf16()?;
            let mut offline_xuid = r.read_u8()? as i64;
            r.read_i32()?;
            let player_count = r.read_u8()?;
            let mut online_xuid = 0i64;
            for i in 0..player_count {
                let off = r.read_i64()?;
                let on = r.read_i64()?;
                if i == 0 {
                    offline_xuid = off;
                    online_xuid = on;
                }
            }
            for _ in 0..14 { r.read_u8()?; }
            r.read_i32()?;
            r.read_u8()?;
            r.read_i32()?;
            LcePacket::PreLogin(PreLoginPacket { net_version, player_name, offline_xuid, online_xuid })
        }
        3 => LcePacket::Chat(ChatPacket {
            message_type: r.read_u8()?,
            string_args: {
                let count = r.read_u16()? as usize;
                let mut v = Vec::with_capacity(count);
                for _ in 0..count { v.push(r.read_string_utf16()?); }
                v
            },
            int_args: {
                let count = r.read_u16()? as usize;
                let mut v = Vec::with_capacity(count);
                for _ in 0..count { v.push(r.read_i32()?); }
                v
            },
        }),
        7 => LcePacket::Interact(InteractPacket {
            source: r.read_u8()?,
            target: r.read_i32()?,
            action: r.read_u8()?,
        }),
        9 => LcePacket::RespawnRequest(RespawnRequestPacket),
        10 | 11 | 12 | 13 => LcePacket::MovePlayer(MovePlayerPacket {
            id,
            x: r.read_f64()?,
            y: r.read_f64()?,
            y_view: r.read_f64()?,
            z: r.read_f64()?,
            yaw: r.read_f32()?,
            pitch: r.read_f32()?,
            flags: if id == 13 { r.read_u8()? } else { 0 },
        }),
        14 => LcePacket::PlayerAction(PlayerActionPacket {
            action: r.read_u8()?,
            x: r.read_i32()?,
            y: r.read_i32()?,
            z: r.read_i32()?,
            face: r.read_u8()?,
        }),
        15 => LcePacket::UseItem(UseItemPacket {
            x: r.read_i32()?,
            y: r.read_i32()?,
            z: r.read_i32()?,
            face: r.read_u8()?,
            item: read_lce_item(&mut r)?,
            click_x: r.read_f32()?,
            click_y: r.read_f32()?,
            click_z: r.read_f32()?,
        }),
        16 => LcePacket::SetCarriedItem(SetCarriedItemPacket {
            slot: r.read_u8()?,
        }),
        18 => LcePacket::Animate(AnimatePacket {
            entity_id: r.read_i32()?,
            action: r.read_u8()?,
        }),
        19 => LcePacket::PlayerCommand(PlayerCommandPacket {
            entity_id: r.read_i32()?,
            action: r.read_u8()?,
            data: r.read_i32()?,
        }),
        20 => LcePacket::AddPlayer(AddPlayerPacket {
            entity_id: r.read_i32()?,
            name: r.read_string_utf16()?,
            x: r.read_i32()?,
            y: r.read_i32()?,
            z: r.read_i32()?,
            yaw: r.read_u8()?,
            pitch: r.read_u8()?,
            head_yaw: r.read_u8()?,
            carried_item: r.read_u16()?,
            offline_xuid: r.read_i64()?,
            online_xuid: r.read_i64()?,
            player_index: r.read_i32()?,
            skin_id: r.read_string_utf16()?,
            cape_id: r.read_string_utf16()?,
            game_privileges: r.read_u32()?,
            metadata: read_entity_metadata(&mut r)?,
        }),
        23 => LcePacket::AddEntity(AddEntityPacket {
            entity_id: r.read_i32()?,
            entity_type: r.read_u8()?,
            x: r.read_i32()?,
            y: r.read_i32()?,
            z: r.read_i32()?,
            yaw: r.read_u8()?,
            pitch: r.read_u8()?,
            data: r.read_i32()?,
            motion_x: r.read_i16()?,
            motion_y: r.read_i16()?,
            motion_z: r.read_i16()?,
        }),
        24 => LcePacket::AddMob(AddMobPacket {
            entity_id: r.read_i32()?,
            entity_type: r.read_u8()?,
            x: r.read_i32()?,
            y: r.read_i32()?,
            z: r.read_i32()?,
            yaw: r.read_u8()?,
            pitch: r.read_u8()?,
            head_yaw: r.read_u8()?,
            motion_x: r.read_i16()?,
            motion_y: r.read_i16()?,
            motion_z: r.read_i16()?,
            metadata: read_entity_metadata(&mut r)?,
        }),
        28 => LcePacket::SetEntityMotion(SetEntityMotionPacket {
            entity_id: r.read_i32()?,
            xa: r.read_i16()?,
            ya: r.read_i16()?,
            za: r.read_i16()?,
        }),
        29 => {
            let count = r.read_u16()? as usize;
            let mut entity_ids = Vec::with_capacity(count);
            for _ in 0..count { entity_ids.push(r.read_i32()?); }
            LcePacket::RemoveEntities(RemoveEntitiesPacket { entity_ids })
        }
        34 => LcePacket::TeleportEntity(TeleportEntityPacket {
            entity_id: r.read_i32()?,
            x: r.read_i32()?,
            y: r.read_i32()?,
            z: r.read_i32()?,
            yaw: r.read_u8()?,
            pitch: r.read_u8()?,
        }),
        35 => LcePacket::RotateHead(RotateHeadPacket {
            entity_id: r.read_i32()?,
            y_head_rot: r.read_u8()?,
        }),
        38 => LcePacket::EntityEvent(EntityEventPacket {
            entity_id: r.read_i32()?,
            event_id: r.read_u8()?,
        }),
        40 => {
            let entity_id = r.read_i32()?;
            let values = read_entity_metadata(&mut r)?;
            LcePacket::SetEntityData(SetEntityDataPacket { entity_id, values })
        }
        50 => LcePacket::ChunkVisibility(ChunkVisibilityPacket {
            chunk_x: r.read_i32()?,
            chunk_z: r.read_i32()?,
            visible: r.read_bool()?,
        }),
        51 => LcePacket::BlockRegionUpdate(BlockRegionUpdatePacket {
            x: r.read_i32()?,
            y: r.read_i32()?,
            z: r.read_i32()?,
            xs: r.read_u8()?,
            ys: r.read_u8()?,
            zs: r.read_u8()?,
            level_idx: r.read_u8()?,
            is_full_chunk: r.read_bool()?,
            compressed_data: {
                let len = r.read_u32()? as usize;
                r.read_bytes(len)?
            },
        }),
        53 => LcePacket::TileUpdate(TileUpdatePacket {
            x: r.read_i32()?,
            y: r.read_i32()?,
            z: r.read_i32()?,
            block: r.read_u16()?,
            data: r.read_u16()?,
            level_idx: r.read_u8()?,
        }),
        70 => LcePacket::GameEvent(GameEventPacket {
            reason: r.read_u8()?,
            param: r.read_f32()?,
        }),
        100 => LcePacket::ContainerOpen(ContainerOpenPacket {
            container_id: r.read_u8()?,
            container_type: r.read_u8()?,
            size: r.read_u8()?,
            custom_name: r.read_string_utf16()?,
            title: r.read_string_utf16()?,
            entity_id: r.read_i32()?,
        }),
        101 => LcePacket::ContainerClose(ContainerClosePacket {
            container_id: r.read_u8()?,
        }),
        102 => LcePacket::ContainerClick(ContainerClickPacket {
            container_id: r.read_u8()?,
            slot_num: r.read_i16()?,
            button_num: r.read_u8()?,
            uid: r.read_i16()?,
            click_type: r.read_u8()?,
            item: read_lce_item(&mut r)?,
        }),
        103 => LcePacket::ContainerSetSlot(ContainerSetSlotPacket {
            container_id: r.read_u8()?,
            slot: r.read_i16()?,
            item: read_lce_item(&mut r)?,
        }),
        104 => LcePacket::ContainerSetContent(ContainerSetContentPacket {
            container_id: r.read_u8()?,
            items: {
                let count = r.read_u16()? as usize;
                let mut v = Vec::with_capacity(count);
                for _ in 0..count { v.push(read_lce_item(&mut r)?); }
                v
            },
        }),
        105 => LcePacket::ContainerSetData(ContainerSetDataPacket {
            container_id: r.read_u8()?,
            id: r.read_i16()?,
            value: r.read_i16()?,
        }),
        106 => LcePacket::ContainerAck(ContainerAckPacket {
            container_id: r.read_u8()?,
            uid: r.read_i16()?,
            accepted: r.read_bool()?,
        }),
        108 => LcePacket::ContainerButtonClick(ContainerButtonClickPacket {
            container_id: r.read_u8()?,
            button_id: r.read_u8()?,
        }),
        150 => LcePacket::CraftItem(CraftItemPacket {
            uid: r.read_i16()?,
            recipe: r.read_u8()?,
        }),
        152 => LcePacket::DebugOptions(DebugOptionsPacket {
            options_mask: r.read_u32()?,
        }),
        155 => LcePacket::ChunkVisibilityArea(ChunkVisibilityAreaPacket {
            min_cx: r.read_i32()?,
            max_cx: r.read_i32()?,
            min_cz: r.read_i32()?,
            max_cz: r.read_i32()?,
        }),
        202 => LcePacket::PlayerAbilities(PlayerAbilitiesPacket {
            flags: r.read_u8()?,
            fly_speed: r.read_f32()?,
            walk_speed: r.read_f32()?,
        }),
        205 => LcePacket::ClientCommand(ClientCommandPacket {
            action: r.read_u8()?,
        }),
        255 => LcePacket::Disconnect(DisconnectPacket {
            reason: r.read_u8()?,
        }),
        4 => LcePacket::SetTime(SetTimePacket {
            game_time: r.read_i64()?,
            day_time: r.read_i64()?,
        }),
        6 => LcePacket::SetSpawnPosition(SetSpawnPositionPacket {
            x: r.read_i32()?,
            y: r.read_i32()?,
            z: r.read_i32()?,
        }),
        8 => LcePacket::SetHealth(SetHealthPacket {
            health: r.read_f32()?,
            food: r.read_i32()?,
            saturation: r.read_f32()?,
            damage_source: r.read_u8()?,
        }),
        _ => LcePacket::Raw { id, data: data.to_vec() },
    })
}

pub fn encode_lce_packet(packet: &LcePacket) -> Vec<u8> {
    let id = packet.id();
    let mut w = LceWriter::new();
    match packet {
        LcePacket::KeepAlive(p) => { w.write_i32(p.keep_alive_id); }
        LcePacket::Login(p) => {
            w.write_i32(p.protocol_version);
            w.write_utf16(&p.username);
            w.write_u64(p.map_seed);
            w.write_i32(p.game_type);
            w.write_utf16(&p.world_name);
            w.write_i32(p.dimension);
            w.write_i32(p.difficulty);
            w.write_u8(p.max_players);
            w.write_i32(p.world_width);
            w.write_i32(p.world_length);
        }
        LcePacket::PreLogin(p) => {
            w.write_i16(p.net_version);
            w.write_utf16(&p.player_name);
            w.write_i64(p.offline_xuid);
            w.write_i64(p.online_xuid);
        }
        LcePacket::Chat(p) => {
            w.write_u8(p.message_type);
            w.write_u16(p.string_args.len() as u16);
            for s in &p.string_args { w.write_utf16(s); }
            w.write_u16(p.int_args.len() as u16);
            for v in &p.int_args { w.write_i32(*v); }
        }
        LcePacket::Disconnect(p) => { w.write_u8(p.reason); }
        LcePacket::Animate(p) => { w.write_i32(p.entity_id); w.write_u8(p.action); }
        LcePacket::MovePlayer(p) => {
            w.write_f64(p.x); w.write_f64(p.y); w.write_f64(p.y_view); w.write_f64(p.z);
            w.write_f32(p.yaw); w.write_f32(p.pitch);
            if p.id == 13 { w.write_u8(p.flags); }
        }
        LcePacket::PlayerAction(p) => {
            w.write_u8(p.action); w.write_i32(p.x); w.write_i32(p.y); w.write_i32(p.z); w.write_u8(p.face);
        }
        LcePacket::Interact(p) => { w.write_u8(p.source); w.write_i32(p.target); w.write_u8(p.action); }
        LcePacket::UseItem(p) => {
            w.write_i32(p.x); w.write_i32(p.y); w.write_i32(p.z); w.write_u8(p.face);
            write_lce_item(&mut w, &p.item);
            w.write_f32(p.click_x); w.write_f32(p.click_y); w.write_f32(p.click_z);
        }
        LcePacket::PlayerCommand(p) => { w.write_i32(p.entity_id); w.write_u8(p.action); w.write_i32(p.data); }
        LcePacket::SetCarriedItem(p) => { w.write_u8(p.slot); }
        LcePacket::PlayerAbilities(p) => { w.write_u8(p.flags); w.write_f32(p.fly_speed); w.write_f32(p.walk_speed); }
        LcePacket::DebugOptions(p) => { w.write_u32(p.options_mask); }
        LcePacket::ClientCommand(p) => { w.write_u8(p.action); }
        LcePacket::RespawnRequest(_) => {}
        LcePacket::ContainerClose(p) => { w.write_u8(p.container_id); }
        LcePacket::ContainerClick(p) => {
            w.write_u8(p.container_id); w.write_i16(p.slot_num); w.write_u8(p.button_num);
            w.write_i16(p.uid); w.write_u8(p.click_type);
            write_lce_item(&mut w, &p.item);
        }
        LcePacket::ContainerButtonClick(p) => { w.write_u8(p.container_id); w.write_u8(p.button_id); }
        LcePacket::CraftItem(p) => { w.write_i16(p.uid); w.write_u8(p.recipe); }
        LcePacket::SetTime(p) => { w.write_i64(p.game_time); w.write_i64(p.day_time); }
        LcePacket::SetSpawnPosition(p) => { w.write_i32(p.x); w.write_i32(p.y); w.write_i32(p.z); }
        LcePacket::SetHealth(p) => { w.write_f32(p.health); w.write_i32(p.food); w.write_f32(p.saturation); w.write_u8(p.damage_source); }
        LcePacket::GameEvent(p) => { w.write_u8(p.reason); w.write_f32(p.param); }
        LcePacket::EntityEvent(p) => { w.write_i32(p.entity_id); w.write_u8(p.event_id); }
        LcePacket::ContainerOpen(p) => {
            w.write_u8(p.container_id); w.write_u8(p.container_type); w.write_u8(p.size);
            w.write_utf16(&p.custom_name); w.write_utf16(&p.title); w.write_i32(p.entity_id);
        }
        LcePacket::ContainerSetSlot(p) => { w.write_u8(p.container_id); w.write_i16(p.slot); write_lce_item(&mut w, &p.item); }
        LcePacket::ContainerSetContent(p) => {
            w.write_u8(p.container_id);
            w.write_u16(p.items.len() as u16);
            for item in &p.items { write_lce_item(&mut w, item); }
        }
        LcePacket::ContainerSetData(p) => { w.write_u8(p.container_id); w.write_i16(p.id); w.write_i16(p.value); }
        LcePacket::ContainerAck(p) => { w.write_u8(p.container_id); w.write_i16(p.uid); w.write_bool(p.accepted); }
        LcePacket::ChunkVisibility(p) => { w.write_i32(p.chunk_x); w.write_i32(p.chunk_z); w.write_bool(p.visible); }
        LcePacket::ChunkVisibilityArea(p) => { w.write_i32(p.min_cx); w.write_i32(p.max_cx); w.write_i32(p.min_cz); w.write_i32(p.max_cz); }
        LcePacket::BlockRegionUpdate(p) => {
            w.write_i32(p.x); w.write_i32(p.y); w.write_i32(p.z);
            w.write_u8(p.xs); w.write_u8(p.ys); w.write_u8(p.zs);
            w.write_u8(p.level_idx); w.write_bool(p.is_full_chunk);
            w.write_u32(p.compressed_data.len() as u32);
            w.write_bytes(&p.compressed_data);
        }
        LcePacket::TileUpdate(p) => { w.write_i32(p.x); w.write_i32(p.y); w.write_i32(p.z); w.write_u16(p.block); w.write_u16(p.data); w.write_u8(p.level_idx); }
        LcePacket::AddPlayer(p) => {
            w.write_i32(p.entity_id); w.write_utf16(&p.name);
            w.write_i32(p.x); w.write_i32(p.y); w.write_i32(p.z);
            w.write_u8(p.yaw); w.write_u8(p.pitch); w.write_u8(p.head_yaw);
            w.write_u16(p.carried_item);
            w.write_i64(p.offline_xuid); w.write_i64(p.online_xuid);
            w.write_i32(p.player_index);
            w.write_utf16(&p.skin_id); w.write_utf16(&p.cape_id);
            w.write_u32(p.game_privileges);
            write_entity_metadata(&mut w, &p.metadata);
        }
        LcePacket::AddEntity(p) => {
            w.write_i32(p.entity_id); w.write_u8(p.entity_type);
            w.write_i32(p.x); w.write_i32(p.y); w.write_i32(p.z);
            w.write_u8(p.yaw); w.write_u8(p.pitch); w.write_i32(p.data);
            w.write_i16(p.motion_x); w.write_i16(p.motion_y); w.write_i16(p.motion_z);
        }
        LcePacket::AddMob(p) => {
            w.write_i32(p.entity_id); w.write_u8(p.entity_type);
            w.write_i32(p.x); w.write_i32(p.y); w.write_i32(p.z);
            w.write_u8(p.yaw); w.write_u8(p.pitch); w.write_u8(p.head_yaw);
            w.write_i16(p.motion_x); w.write_i16(p.motion_y); w.write_i16(p.motion_z);
            write_entity_metadata(&mut w, &p.metadata);
        }
        LcePacket::TeleportEntity(p) => { w.write_i32(p.entity_id); w.write_i32(p.x); w.write_i32(p.y); w.write_i32(p.z); w.write_u8(p.yaw); w.write_u8(p.pitch); }
        LcePacket::RotateHead(p) => { w.write_i32(p.entity_id); w.write_u8(p.y_head_rot); }
        LcePacket::SetEntityMotion(p) => { w.write_i32(p.entity_id); w.write_i16(p.xa); w.write_i16(p.ya); w.write_i16(p.za); }
        LcePacket::RemoveEntities(p) => {
            w.write_u16(p.entity_ids.len() as u16);
            for id in &p.entity_ids { w.write_i32(*id); }
        }
        LcePacket::SetEntityData(p) => {
            w.write_i32(p.entity_id);
            write_entity_metadata(&mut w, &p.values);
        }
        LcePacket::Raw { data, .. } => { w.write_bytes(data); }
    }
    let payload = w.into_bytes();
    let mut frame = Vec::with_capacity(4 + 1 + payload.len());
    frame.extend_from_slice(&(payload.len() as u32 + 1).to_be_bytes());
    frame.push(id);
    frame.extend_from_slice(&payload);
    frame
}

fn read_lce_item(r: &mut LceReader) -> std::io::Result<LceItemStack> {
    let id = r.read_i16()?;
    let count = r.read_u8()?;
    let damage = r.read_i16()?;
    let _nbt_size = r.read_i16()?;
    if _nbt_size > 0 { r.skip(_nbt_size as usize)?; }
    Ok(LceItemStack { id, count, damage })
}

fn write_lce_item(w: &mut LceWriter, item: &LceItemStack) {
    w.write_i16(item.id);
    w.write_u8(item.count);
    w.write_i16(item.damage);
    w.write_i16(0);
}

fn read_entity_metadata(r: &mut LceReader) -> std::io::Result<Vec<EntityDataValue>> {
    let mut values = Vec::new();
    loop {
        let id = r.read_u8()?;
        if id == 0x7F { break; }
        let data_type = r.read_u8()?;
        let value = match data_type {
            0 => EntityDataValueType::Byte(r.read_u8()?),
            1 => EntityDataValueType::Short(r.read_i16()?),
            2 => EntityDataValueType::Int(r.read_i32()?),
            3 => EntityDataValueType::Float(r.read_f32()?),
            4 => EntityDataValueType::String(r.read_string_utf16()?),
            5 => EntityDataValueType::ItemStack(read_lce_item(r)?),
            _ => { r.skip(1)?; EntityDataValueType::Byte(0) }
        };
        values.push(EntityDataValue { id, data_type, value });
    }
    Ok(values)
}

fn write_entity_metadata(w: &mut LceWriter, values: &[EntityDataValue]) {
    for v in values {
        w.write_u8(v.id);
        w.write_u8(v.data_type);
        match &v.value {
            EntityDataValueType::Byte(b) => w.write_u8(*b),
            EntityDataValueType::Short(s) => w.write_i16(*s),
            EntityDataValueType::Int(i) => w.write_i32(*i),
            EntityDataValueType::Float(f) => w.write_f32(*f),
            EntityDataValueType::String(s) => w.write_utf16(s),
            EntityDataValueType::ItemStack(item) => write_lce_item(w, item),
        }
    }
    w.write_u8(0x7F);
}

pub async fn run_lce_server(config: BridgeConfig) -> std::io::Result<()> {
    let addr = format!("{}:{}", config.lce.listen_address, config.lce.port);
    let listener = TcpListener::bind(&addr).await?;
    eprintln!("[LCEBridge] Listening on {}", addr);
    loop {
        let (stream, addr) = listener.accept().await?;
        eprintln!("[LCEBridge] Connection from {}", addr);
        let config = config.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_connection(stream, config).await {
                eprintln!("[LCEBridge] Connection error: {}", e);
            }
        });
    }
}

async fn handle_connection(stream: TcpStream, config: BridgeConfig) -> std::io::Result<()> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let (mut reader, mut writer) = stream.into_split();
    let small_id: u8 = 1;
    writer.write_all(&small_id.to_be_bytes()).await?;
    eprintln!("[LCEBridge] small_id sent");
    let session = Arc::new(Mutex::new(LceBridgeSession::new(config)));
    {
        let mut s = session.lock().await;
        s.on_connected();
    }
    let session_clone = session.clone();
    let read_handle = tokio::spawn(async move {
        let mut header = [0u8; 4];
        loop {
            match reader.read_exact(&mut header).await {
                Ok(_) => {}
                Err(_) => {
                    eprintln!("[LCEBridge] LCE read error (connection closed?)");
                    break;
                }
            }
            let payload_len = u32::from_be_bytes(header) as usize;
            eprintln!("[LCEBridge] received frame len={}", payload_len);
            if payload_len > 2_097_152 {
                break;
            }
            let mut payload = vec![0u8; payload_len];
            if reader.read_exact(&mut payload).await.is_err() {
                break;
            }
            let hex: String = payload.iter().map(|b| format!("{:02x}", b)).collect::<Vec<_>>().join(" ");
            eprintln!("[LCEBridge] frame hex=[{}]", hex);
            let mut offset = 0;
            while offset < payload_len {
                if offset >= payload_len { break; }
                let packet_id = payload[offset];
                offset += 1;
                let remaining = payload_len - offset;
                let packet_data = &payload[offset..offset + remaining];
                match decode_lce_packet(packet_id, packet_data) {
                    Ok(packet) => {
                        let mut s = session_clone.lock().await;
                        s.handle_lce_packet(packet).await;
                    }
                    Err(e) => {
                        eprintln!("[LCEBridge] decode failed for packet_id={} len={}: {}", packet_id, remaining, e);
                    }
                }
                offset += remaining;
            }
        }
    });
    let write_clone = session.clone();
    let _write_handle = tokio::spawn(async move {
        let mut out_buf: Vec<u8> = Vec::with_capacity(4096);
        loop {
            out_buf.clear();
            let mut s = write_clone.lock().await;
            while let Some(packet) = s.lce_tx.pop_front() {
                let frame = encode_lce_packet(&packet);
                out_buf.extend_from_slice(&frame);
            }
            drop(s);
            if !out_buf.is_empty() {
                if writer.write_all(&out_buf).await.is_err() { break; }
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }
    });
    let java_clone = session.clone();
    tokio::spawn(async move {
        use tokio::sync::mpsc::error::TryRecvError;
        loop {
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            let packet = {
                let mut s = java_clone.lock().await;
                let js = match s.java_session.as_mut() {
                    Some(js) => js,
                    None => continue,
                };
                match js.rx.try_recv() {
                    Ok(p) => p,
                    Err(TryRecvError::Empty) => continue,
                    Err(TryRecvError::Disconnected) => return,
                }
            };
            let mut s = java_clone.lock().await;
            s.handle_java_packet(packet).await;
        }
    });
    let _ = read_handle.await;
    Ok(())
}
