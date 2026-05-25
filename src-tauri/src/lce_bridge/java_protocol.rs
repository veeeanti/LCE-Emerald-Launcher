use crate::lce_bridge::util::*;

use std::io::Read;

pub struct JavaConnection {
    stream: tokio::net::TcpStream,
    compression_threshold: i32,
    pub state: JavaProtocolState,
}

#[derive(Debug)]
pub enum JavaProtocolState {
    Handshaking,
    Login,
    Config,
    Play,
}

impl JavaConnection {
    pub async fn connect(host: &str, port: u16) -> std::io::Result<Self> {
        let stream = tokio::net::TcpStream::connect((host, port)).await?;
        Ok(Self {
            stream,
            compression_threshold: -1,
            state: JavaProtocolState::Handshaking,
        })
    }

    pub async fn send_packet(&mut self, packet: &JavaPacket) -> std::io::Result<()> {
        use tokio::io::AsyncWriteExt;
        let mut payload = Vec::new();
        packet.encode(&mut payload)?;
        let mut frame = Vec::new();
        frame.extend_from_slice(&VarInt::write(packet.id()));
        if self.compression_threshold >= 0 {
            if payload.len() as i32 > self.compression_threshold {
                use flate2::write::ZlibEncoder;
                use flate2::Compression;
                use std::io::Write;
                let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
                encoder.write_all(&payload)?;
                let compressed = encoder.finish()?;
                let mut compressed_frame = Vec::new();
                compressed_frame.extend_from_slice(&VarInt::write(payload.len() as i32));
                compressed_frame.extend_from_slice(&compressed);
                payload = compressed_frame;
            } else {
                let mut uncompressed_frame = Vec::new();
                uncompressed_frame.extend_from_slice(&VarInt::write(0));
                uncompressed_frame.extend_from_slice(&payload);
                payload = uncompressed_frame;
            }
        }
        frame.extend_from_slice(&payload);
        let mut len_prefix = VarInt::write(frame.len() as i32);
        len_prefix.extend_from_slice(&frame);
        self.stream.write_all(&len_prefix).await?;
        Ok(())
    }

    pub async fn read_packet(&mut self) -> std::io::Result<JavaPacket> {
        use tokio::io::AsyncReadExt;
        let len = read_varint_stream(&mut self.stream).await?;
        let mut packet_data = vec![0u8; len as usize];
        self.stream.read_exact(&mut packet_data).await?;
        let decompressed = if self.compression_threshold >= 0 {
            let mut data = &packet_data[..];
            let uncompressed_len = VarInt::read(&mut data)?;
            if uncompressed_len > 0 {
                use flate2::read::ZlibDecoder;
                use std::io::Read;
                let mut decoder = ZlibDecoder::new(data);
                let mut buf = Vec::with_capacity(uncompressed_len as usize);
                decoder.read_to_end(&mut buf)?;
                buf
            } else {
                data.to_vec()
            }
        } else {
            packet_data.clone()
        };
        let mut data = &decompressed[..];
        let packet_id = VarInt::read(&mut data)?;
        JavaPacket::decode(packet_id, data, &self.state)
    }
}

async fn read_varint_stream(stream: &mut tokio::net::TcpStream) -> std::io::Result<i32> {
    use tokio::io::AsyncReadExt;
    let mut result = 0i32;
    let mut shift = 0;
    loop {
        let byte = stream.read_u8().await?;
        result |= ((byte & 0x7F) as i32) << shift;
        shift += 7;
        if byte & 0x80 == 0 {
            return Ok(result);
        }
        if shift >= 35 {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "varint too long"));
        }
    }
}

#[derive(Debug, Clone)]
pub enum JavaPacket {
    Handshake(HandshakePacket),
    LoginStart(LoginStartPacket),
    LoginSuccess(LoginSuccessPacket),
    LoginAcknowledged,
    SetCompression(SetCompressionPacket),
    FinishConfig,
    KeepAliveC2S(KeepAliveC2SPacket),
    KeepAliveS2C(KeepAliveS2CPacket),
    ChunkData(ChunkDataPacket),
    ChunkBatchFinished,
    RegistryData(RegistryDataPacket),
    PlayerPosition(PlayerPositionPacket),
    AcceptTeleportation(AcceptTeleportationPacket),
    ClientInformation(ClientInformationPacket),
    SynchronizePlayerPosition(SynchronizePlayerPositionPacket),
    SetContainerContent(SetContainerContentPacket),
    SetContainerSlot(SetContainerSlotPacket),
    OpenScreen(OpenScreenPacket),
    SystemChat(SystemChatPacket),
    PlayerChat(PlayerChatPacket),
    ChatCommand(ChatCommandPacket),
    SetHealth(SetHealthS2CPacket),
    SetTime(SetTimeS2CPacket),
    AddEntity(AddEntityS2CPacket),
    AddMob(AddMobS2CPacket),
    AddPlayer(AddPlayerS2CPacket),
    RemoveEntities(RemoveEntitiesS2CPacket),
    SetEntityMotion(SetEntityMotionS2CPacket),
    TeleportEntity(TeleportEntityS2CPacket),
    SetHeadRotation(SetHeadRotationS2CPacket),
    SetEntityData(SetEntityDataS2CPacket),
    LoginPlay(LoginPlayPacket),
    SoundEffect(SoundEffectS2CPacket),
    SetDefaultSpawnPosition(SetDefaultSpawnPositionS2CPacket),
    GameEventS2C(GameEventS2CPacket),
    ContainerCloseS2C(ContainerCloseS2CPacket),
    ContainerSetDataS2C(ContainerSetDataS2CPacket),
    ContainerAckS2C(ContainerAckS2CPacket),
    PingPong(PingPongPacket),
    EncryptionRequest(EncryptionRequestPacket),
    EncryptionResponse(EncryptionResponsePacket),
    Unknown { id: i32, data: Vec<u8> },
}

impl JavaPacket {
    pub fn id(&self) -> i32 {
        use JavaPacket::*;
        match self {
            Handshake(_) => 0x00,
            LoginStart(_) => 0x00,
            LoginSuccess(_) => 0x03,
            LoginAcknowledged => 0x03,
            SetCompression(_) => 0x03,
            FinishConfig => 0x03,
            KeepAliveC2S(_) => 0x18,
            KeepAliveS2C(_) => 0x26,
            ChunkData(_) => 0x24,
            ChunkBatchFinished => 0x71,
            RegistryData(_) => 0x7C,
            PlayerPosition(_) => 0x40,
            AcceptTeleportation(_) => 0x00,
            ClientInformation(_) => 0x09,
            SynchronizePlayerPosition(_) => 0x3E,
            SetContainerContent(_) => 0x13,
            SetContainerSlot(_) => 0x14,
            OpenScreen(_) => 0x32,
            SystemChat(_) => 0x6F,
            PlayerChat(_) => 0x37,
            ChatCommand(_) => 0x04,
            SetHealth(_) => 0x5A,
            SetTime(_) => 0x5C,
            AddEntity(_) => 0x01,
            AddMob(_) => 0x03,
            AddPlayer(_) => 0x04,
            RemoveEntities(_) => 0x41,
            SetEntityMotion(_) => 0x2E,
            TeleportEntity(_) => 0x6E,
            SetHeadRotation(_) => 0x4A,
            SetEntityData(_) => 0x4C,
            LoginPlay(_) => 0x2C,
            SoundEffect(_) => 0x64,
            SetDefaultSpawnPosition(_) => 0x5B,
            GameEventS2C(_) => 0x1F,
            ContainerCloseS2C(_) => 0x12,
            ContainerSetDataS2C(_) => 0x15,
            ContainerAckS2C(_) => 0x07,
            EncryptionRequest(_) => 0x01,
            EncryptionResponse(_) => 0x01,
            PingPong(_) => 0x26,
            Unknown { id, .. } => *id,
        }
    }

    pub fn encode(&self, buf: &mut Vec<u8>) -> std::io::Result<()> {
        use JavaPacket::*;
        match self {
            Handshake(p) => {
                VarInt::write_to(p.protocol_version, buf);
                write_utf8_string(buf, &p.server_address);
                buf.extend_from_slice(&p.server_port.to_be_bytes());
                VarInt::write_to(p.next_state, buf);
            }
            LoginStart(p) => {
                write_utf8_string(buf, &p.username);
                if p.protocol_version >= 761 && p.protocol_version <= 765 {
                    buf.push(0);
                }
                if p.protocol_version >= 761 {
                    buf.extend_from_slice(p.profile_id.as_bytes());
                }
            }
            LoginSuccess(p) => {
                write_uuid(buf, &p.uuid);
                write_utf8_string(buf, &p.username);
                VarInt::write_to(0, buf);
            }
            LoginAcknowledged => {}
            SetCompression(p) => { VarInt::write_to(p.threshold, buf); }
            FinishConfig => {}
            KeepAliveC2S(p) => { buf.extend_from_slice(&p.id.to_be_bytes()); }
            KeepAliveS2C(p) => { buf.extend_from_slice(&p.id.to_be_bytes()); }
            ClientInformation(p) => {
                write_utf8_string(buf, &p.locale);
                buf.push(p.view_distance);
                VarInt::write_to(p.chat_mode, buf);
                buf.push(p.chat_colors as u8);
                buf.push(p.displayed_skin_parts);
                VarInt::write_to(p.main_hand, buf);
                buf.push(p.enable_text_filtering as u8);
                buf.push(p.allow_listing as u8);
                VarInt::write_to(p.particle_status, buf);
            }
            AcceptTeleportation(p) => { VarInt::write_to(p.teleport_id, buf); }
            PingPong(p) => { buf.extend_from_slice(&p.id.to_be_bytes()); }
            EncryptionRequest(p) => {
                write_utf8_string(buf, &p.server_id);
                VarInt::write_to(p.public_key.len() as i32, buf);
                buf.extend_from_slice(&p.public_key);
                VarInt::write_to(p.verify_token.len() as i32, buf);
                buf.extend_from_slice(&p.verify_token);
            }
            EncryptionResponse(p) => {
                VarInt::write_to(p.shared_secret.len() as i32, buf);
                buf.extend_from_slice(&p.shared_secret);
                VarInt::write_to(p.verify_token.len() as i32, buf);
                buf.extend_from_slice(&p.verify_token);
            }
            _ => {
                if let Unknown { data, .. } = self {
                    buf.extend_from_slice(data);
                }
            }
        }
        Ok(())
    }

    pub fn decode(id: i32, data: &[u8], state: &JavaProtocolState) -> std::io::Result<Self> {
        let mut r = data;
        Ok(match (state, id) {
            (JavaProtocolState::Login, 0x00) => {
                let reason = read_utf8_string(&mut r)?;
                eprintln!("[LCEBridge] Java server disconnected: {}", reason);
                JavaPacket::Unknown { id, data: data.to_vec() }
            }
            (JavaProtocolState::Login, 0x01) => {
                let server_id = read_utf8_string(&mut r)?;
                let key_len = VarInt::read(&mut r)? as usize;
                let mut public_key = vec![0u8; key_len];
                r.read_exact(&mut public_key)?;
                let token_len = VarInt::read(&mut r)? as usize;
                let mut verify_token = vec![0u8; token_len];
                r.read_exact(&mut verify_token)?;
                JavaPacket::EncryptionRequest(EncryptionRequestPacket {
                    server_id, public_key, verify_token,
                })
            }
            (JavaProtocolState::Login, 0x02) => {
                let saved = data;
                let mut r2 = saved;
                if let Ok(uuid) = read_uuid(&mut r2) {
                    if let Ok(username) = read_utf8_string(&mut r2) {
                        return Ok(JavaPacket::LoginSuccess(LoginSuccessPacket { uuid, username }));
                    }
                }
                let uuid_str = read_utf8_string(&mut r)?;
                let uuid = uuid::Uuid::try_parse(&uuid_str).unwrap_or(uuid::Uuid::nil());
                let username = read_utf8_string(&mut r)?;
                JavaPacket::LoginSuccess(LoginSuccessPacket { uuid, username })
            }
            (JavaProtocolState::Login, 0x03) => {
                let peek = r.first().copied().unwrap_or(0);
                if (peek & 0x80) != 0 || peek >= 4 {
                    let threshold = VarInt::read(&mut r)?;
                    JavaPacket::SetCompression(SetCompressionPacket { threshold })
                } else {
                    let uuid = read_uuid(&mut r)?;
                    let username = read_utf8_string(&mut r)?;
                    JavaPacket::LoginSuccess(LoginSuccessPacket { uuid, username })
                }
            }
            (JavaProtocolState::Config, 0x03) => {
                JavaPacket::FinishConfig
            }
            (JavaProtocolState::Play, 0x26) => {
                let id_val = read_i64be(&mut r)?;
                JavaPacket::KeepAliveS2C(KeepAliveS2CPacket { id: id_val })
            }
            (JavaProtocolState::Play, 0x24) => {
                let chunk_x = read_i32be(&mut r)?;
                let chunk_z = read_i32be(&mut r)?;
                let _heightmaps = read_nbt(&mut r)?;
                let data_size = VarInt::read(&mut r)? as usize;
                let mut chunk_data = vec![0u8; data_size];
                r.read_exact(&mut chunk_data)?;
                let block_entities_count = VarInt::read(&mut r)?;
                for _ in 0..block_entities_count {
                    let _packed_xyz = read_u8be(&mut r)?;
                    let _block_entity_type = VarInt::read(&mut r)?;
                    let _nbt = read_nbt(&mut r)?;
                }
                let _trust_edges = read_u8be(&mut r)? != 0;
                let _sky_light_mask = read_varint_array(&mut r)?;
                let _block_light_mask = read_varint_array(&mut r)?;
                let _empty_sky_light_mask = read_varint_array(&mut r)?;
                let _empty_block_light_mask = read_varint_array(&mut r)?;
                let sky_light_arrays_count = VarInt::read(&mut r)?;
                let mut light_data = Vec::new();
                for _ in 0..sky_light_arrays_count {
                    let light_len = VarInt::read(&mut r)? as usize;
                    let mut light = vec![0u8; light_len];
                    r.read_exact(&mut light)?;
                    light_data.extend_from_slice(&light);
                }
                let block_light_arrays_count = VarInt::read(&mut r)?;
                for _ in 0..block_light_arrays_count {
                    let light_len = VarInt::read(&mut r)? as usize;
                    let mut light = vec![0u8; light_len];
                    r.read_exact(&mut light)?;
                }
                JavaPacket::ChunkData(ChunkDataPacket {
                    chunk_x, chunk_z, chunk_data, light_data,
                    trust_edges: false, block_entities: Vec::new(),
                })
            }
            (JavaProtocolState::Play, 0x71) => {
                JavaPacket::ChunkBatchFinished
            }
            (JavaProtocolState::Config, 0x07) => {
                let _registry_codec = read_nbt(&mut r)?;
                JavaPacket::RegistryData(RegistryDataPacket {})
            }
            (JavaProtocolState::Play, 0x40) => {
                let teleport_id = VarInt::read(&mut r)?;
                let x = read_f64be(&mut r)?;
                let y = read_f64be(&mut r)?;
                let z = read_f64be(&mut r)?;
                let yaw = read_f32be(&mut r)?;
                let pitch = read_f32be(&mut r)?;
                let _flags = read_u8be(&mut r)?;
                let _ = VarInt::read(&mut r)?;
                JavaPacket::PlayerPosition(PlayerPositionPacket {
                    teleport_id, x, y, z, yaw, pitch, flags: _flags,
                })
            }
            (JavaProtocolState::Play, 0x3E) => {
                let teleport_id = VarInt::read(&mut r)?;
                let x = read_f64be(&mut r)?;
                let y = read_f64be(&mut r)?;
                let z = read_f64be(&mut r)?;
                let yaw = read_f32be(&mut r)?;
                let pitch = read_f32be(&mut r)?;
                let _dismount_vehicle = read_u8be(&mut r)?;
                let _ = VarInt::read(&mut r)?;
                JavaPacket::SynchronizePlayerPosition(SynchronizePlayerPositionPacket {
                    teleport_id, x, y, z, yaw, pitch,
                })
            }
            (JavaProtocolState::Play, 0x13) => {
                let container_id = VarInt::read(&mut r)?;
                let _state_id = VarInt::read(&mut r)?;
                let slots_count = VarInt::read(&mut r)? as usize;
                let mut slots = Vec::with_capacity(slots_count);
                for _ in 0..slots_count {
                    slots.push(read_java_slot(&mut r)?);
                }
                let _carried_item = read_java_slot(&mut r)?;
                JavaPacket::SetContainerContent(SetContainerContentPacket {
                    container_id, state_id: _state_id, slots,
                })
            }
            (JavaProtocolState::Play, 0x14) => {
                let container_id = VarInt::read(&mut r)?;
                let _state_id = VarInt::read(&mut r)?;
                let slot = i16::try_from(VarInt::read(&mut r)?).unwrap_or(0);
                let item = read_java_slot(&mut r)?;
                JavaPacket::SetContainerSlot(SetContainerSlotPacket {
                    container_id, state_id: _state_id, slot, item,
                })
            }
            (JavaProtocolState::Play, 0x32) => {
                let container_id = VarInt::read(&mut r)?;
                let _screen_id = VarInt::read(&mut r)?;
                let _title = read_nbt(&mut r)?;
                JavaPacket::OpenScreen(OpenScreenPacket {
                    container_id, screen_id: _screen_id, title: _title,
                })
            }
            (JavaProtocolState::Play, 0x6F) => {
                let content = read_nbt(&mut r)?;
                let is_actionbar = read_u8be(&mut r)? != 0;
                JavaPacket::SystemChat(SystemChatPacket {
                    content_json: content, is_actionbar,
                })
            }
            (JavaProtocolState::Play, 0x37) => {
                let content = read_nbt(&mut r)?;
                let _position = read_i64be(&mut r)?;
                let mut uuid_bytes = [0u8; 16];
                r.read_exact(&mut uuid_bytes)?;
                let sender = uuid::Uuid::from_bytes(uuid_bytes);
                JavaPacket::PlayerChat(PlayerChatPacket {
                    content_json: content, sender,
                })
            }
            (JavaProtocolState::Play, 0x5A) => {
                let health = read_f32be(&mut r)?;
                let food = VarInt::read(&mut r)?;
                let saturation = read_f32be(&mut r)?;
                JavaPacket::SetHealth(SetHealthS2CPacket { health, food, saturation })
            }
            (JavaProtocolState::Play, 0x5C) => {
                let world_age = read_i64be(&mut r)?;
                let day_time = read_i64be(&mut r)?;
                JavaPacket::SetTime(SetTimeS2CPacket { world_age, day_time })
            }
            (JavaProtocolState::Play, 0x01) => {
                let entity_id = VarInt::read(&mut r)?;
                let entity_type = VarInt::read(&mut r)?;
                let x = read_f64be(&mut r)?;
                let y = read_f64be(&mut r)?;
                let z = read_f64be(&mut r)?;
                let yaw = read_u8be(&mut r)?;
                let pitch = read_u8be(&mut r)?;
                let head_yaw = read_u8be(&mut r)?;
                let data = VarInt::read(&mut r)?;
                let vel_x = read_i16be(&mut r)?;
                let vel_y = read_i16be(&mut r)?;
                let vel_z = read_i16be(&mut r)?;
                JavaPacket::AddEntity(AddEntityS2CPacket {
                    entity_id, entity_type, x, y, z, yaw, pitch, head_yaw, data,
                    vel_x, vel_y, vel_z,
                })
            }
            (JavaProtocolState::Play, 0x03) => {
                let entity_id = VarInt::read(&mut r)?;
                let _uuid = read_uuid(&mut r)?;
                let entity_type = VarInt::read(&mut r)?;
                let x = read_f64be(&mut r)?;
                let y = read_f64be(&mut r)?;
                let z = read_f64be(&mut r)?;
                let yaw = read_u8be(&mut r)?;
                let pitch = read_u8be(&mut r)?;
                let head_yaw = read_u8be(&mut r)?;
                let vel_x = read_i16be(&mut r)?;
                let vel_y = read_i16be(&mut r)?;
                let vel_z = read_i16be(&mut r)?;
                let _metadata = read_java_metadata(&mut r)?;
                JavaPacket::AddMob(AddMobS2CPacket {
                    entity_id, entity_type, x, y, z, yaw, pitch, head_yaw,
                    vel_x, vel_y, vel_z,
                })
            }
            (JavaProtocolState::Play, 0x04) => {
                let entity_id = VarInt::read(&mut r)?;
                let _uuid = read_uuid(&mut r)?;
                let x = read_f64be(&mut r)?;
                let y = read_f64be(&mut r)?;
                let z = read_f64be(&mut r)?;
                let yaw = read_u8be(&mut r)?;
                let pitch = read_u8be(&mut r)?;
                let _metadata = read_java_metadata(&mut r)?;
                JavaPacket::AddPlayer(AddPlayerS2CPacket {
                    entity_id, x, y, z, yaw, pitch,
                })
            }
            (JavaProtocolState::Play, 0x41) => {
                let count = VarInt::read(&mut r)?;
                let mut entity_ids = Vec::with_capacity(count as usize);
                for _ in 0..count {
                    entity_ids.push(VarInt::read(&mut r)?);
                }
                JavaPacket::RemoveEntities(RemoveEntitiesS2CPacket { entity_ids })
            }
            (JavaProtocolState::Play, 0x2E) => {
                let entity_id = VarInt::read(&mut r)?;
                let xa = read_i16be(&mut r)?;
                let ya = read_i16be(&mut r)?;
                let za = read_i16be(&mut r)?;
                JavaPacket::SetEntityMotion(SetEntityMotionS2CPacket { entity_id, xa, ya, za })
            }
            (JavaProtocolState::Play, 0x6E) => {
                let entity_id = VarInt::read(&mut r)?;
                let x = read_f64be(&mut r)?;
                let y = read_f64be(&mut r)?;
                let z = read_f64be(&mut r)?;
                let yaw = read_u8be(&mut r)?;
                let pitch = read_u8be(&mut r)?;
                let _on_ground = read_u8be(&mut r)? != 0;
                JavaPacket::TeleportEntity(TeleportEntityS2CPacket {
                    entity_id, x, y, z, yaw, pitch,
                })
            }
            (JavaProtocolState::Play, 0x4A) => {
                let entity_id = VarInt::read(&mut r)?;
                let head_yaw = read_u8be(&mut r)?;
                JavaPacket::SetHeadRotation(SetHeadRotationS2CPacket { entity_id, head_yaw })
            }
            (JavaProtocolState::Play, 0x4C) => {
                let entity_id = VarInt::read(&mut r)?;
                let _metadata = read_java_metadata(&mut r)?;
                JavaPacket::SetEntityData(SetEntityDataS2CPacket { entity_id })
            }
            (JavaProtocolState::Play, 0x2C) => {
                let entity_id = VarInt::read(&mut r)?;
                let _is_hardcore = read_u8be(&mut r)? != 0;
                let _dim_count = VarInt::read(&mut r)?;
                for _ in 0.._dim_count {
                    let _ = read_utf8_string(&mut r)?;
                }
                let _max_players = VarInt::read(&mut r)?;
                let view_distance = VarInt::read(&mut r)?;
                let simulation_distance = VarInt::read(&mut r)?;
                let _reduced_debug = read_u8be(&mut r)? != 0;
                let _enable_respawn_screen = read_u8be(&mut r)? != 0;
                let _do_limited_crafting = read_u8be(&mut r)? != 0;
                let _dim_type = VarInt::read(&mut r)?;
                let _dim_name = read_utf8_string(&mut r)?;
                let _hashed_seed = read_i64be(&mut r)?;
                let _ = VarInt::read(&mut r)?;
                let _is_debug = read_u8be(&mut r)? != 0;
                let _is_flat = read_u8be(&mut r)? != 0;
                JavaPacket::LoginPlay(LoginPlayPacket {
                    entity_id, is_hardcore: _is_hardcore,
                    view_distance, simulation_distance,
                    reduced_debug: _reduced_debug,
                    enable_respawn_screen: _enable_respawn_screen,
                    do_limited_crafting: _do_limited_crafting,
                    hashed_seed: _hashed_seed,
                    is_debug: _is_debug,
                    is_flat: _is_flat,
                })
            }
            (JavaProtocolState::Play, 0x64) => {
                let _sound_id = VarInt::read(&mut r)?;
                let _sound_category = VarInt::read(&mut r)?;
                let _x = read_i32be(&mut r)?;
                let _y = read_i32be(&mut r)?;
                let _z = read_i32be(&mut r)?;
                let _volume = read_f32be(&mut r)?;
                let _pitch = read_f32be(&mut r)?;
                let _seed = read_i64be(&mut r)?;
                JavaPacket::SoundEffect(SoundEffectS2CPacket {
                    sound_id: _sound_id, sound_category: _sound_category,
                    x: _x, y: _y, z: _z, volume: _volume, pitch: _pitch, seed: _seed,
                })
            }
            (JavaProtocolState::Play, 0x5B) => {
                let x = read_i32be(&mut r)?;
                let y = read_i32be(&mut r)?;
                let z = read_i32be(&mut r)?;
                let _angle = read_f32be(&mut r)?;
                JavaPacket::SetDefaultSpawnPosition(SetDefaultSpawnPositionS2CPacket { x, y, z })
            }
            (JavaProtocolState::Play, 0x1F) => {
                let reason = read_u8be(&mut r)?;
                let param = read_f32be(&mut r)?;
                JavaPacket::GameEventS2C(GameEventS2CPacket { reason, param })
            }
            (JavaProtocolState::Play, 0x12) => {
                let container_id = VarInt::read(&mut r)?;
                JavaPacket::ContainerCloseS2C(ContainerCloseS2CPacket { container_id })
            }
            (JavaProtocolState::Play, 0x15) => {
                let container_id = VarInt::read(&mut r)?;
                let key = VarInt::read(&mut r)?;
                let value = VarInt::read(&mut r)?;
                JavaPacket::ContainerSetDataS2C(ContainerSetDataS2CPacket { container_id, key, value })
            }
            (JavaProtocolState::Play, 0x07) => {
                let container_id = read_u8be(&mut r)?;
                let uid = read_i16be(&mut r)?;
                let accepted = read_u8be(&mut r)? != 0;
                JavaPacket::ContainerAckS2C(ContainerAckS2CPacket { container_id, uid, accepted })
            }
            _ => {
                JavaPacket::Unknown { id, data: data.to_vec() }
            }
        })
    }
}

#[derive(Debug, Clone)]
pub struct HandshakePacket {
    pub protocol_version: i32,
    pub server_address: String,
    pub server_port: u16,
    pub next_state: i32,
}

#[derive(Debug, Clone)]
pub struct LoginStartPacket {
    pub username: String,
    pub profile_id: uuid::Uuid,
    pub protocol_version: i32,
}

#[derive(Debug, Clone)]
pub struct LoginSuccessPacket {
    pub uuid: uuid::Uuid,
    pub username: String,
}

#[derive(Debug, Clone)]
pub struct SetCompressionPacket {
    pub threshold: i32,
}

#[derive(Debug, Clone)]
pub struct KeepAliveC2SPacket {
    pub id: i64,
}

#[derive(Debug, Clone)]
pub struct KeepAliveS2CPacket {
    pub id: i64,
}

#[derive(Debug, Clone)]
pub struct ChunkDataPacket {
    pub chunk_x: i32,
    pub chunk_z: i32,
    pub chunk_data: Vec<u8>,
    pub light_data: Vec<u8>,
    pub trust_edges: bool,
    pub block_entities: Vec<()>,
}

#[derive(Debug, Clone)]
pub struct RegistryDataPacket {}

#[derive(Debug, Clone)]
pub struct PlayerPositionPacket {
    pub teleport_id: i32,
    pub x: f64, pub y: f64, pub z: f64,
    pub yaw: f32, pub pitch: f32,
    pub flags: u8,
}

#[derive(Debug, Clone)]
pub struct AcceptTeleportationPacket {
    pub teleport_id: i32,
}

#[derive(Debug, Clone)]
pub struct ClientInformationPacket {
    pub locale: String,
    pub view_distance: u8,
    pub chat_mode: i32,
    pub chat_colors: bool,
    pub displayed_skin_parts: u8,
    pub main_hand: i32,
    pub enable_text_filtering: bool,
    pub allow_listing: bool,
    pub particle_status: i32,
}

#[derive(Debug, Clone)]
pub struct SynchronizePlayerPositionPacket {
    pub teleport_id: i32,
    pub x: f64, pub y: f64, pub z: f64,
    pub yaw: f32, pub pitch: f32,
}

#[derive(Debug, Clone)]
pub struct SetContainerContentPacket {
    pub container_id: i32,
    pub state_id: i32,
    pub slots: Vec<JavaSlot>,
}

#[derive(Debug, Clone)]
pub struct SetContainerSlotPacket {
    pub container_id: i32,
    pub state_id: i32,
    pub slot: i16,
    pub item: JavaSlot,
}

#[derive(Debug, Clone)]
pub struct OpenScreenPacket {
    pub container_id: i32,
    pub screen_id: i32,
    pub title: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct SystemChatPacket {
    pub content_json: Vec<u8>,
    pub is_actionbar: bool,
}

#[derive(Debug, Clone)]
pub struct PlayerChatPacket {
    pub content_json: Vec<u8>,
    pub sender: uuid::Uuid,
}

#[derive(Debug, Clone)]
pub struct ChatCommandPacket {
    pub command: String,
}

#[derive(Debug, Clone)]
pub struct SetHealthS2CPacket {
    pub health: f32,
    pub food: i32,
    pub saturation: f32,
}

#[derive(Debug, Clone)]
pub struct SetTimeS2CPacket {
    pub world_age: i64,
    pub day_time: i64,
}

#[derive(Debug, Clone)]
pub struct AddEntityS2CPacket {
    pub entity_id: i32,
    pub entity_type: i32,
    pub x: f64, pub y: f64, pub z: f64,
    pub yaw: u8, pub pitch: u8, pub head_yaw: u8,
    pub data: i32,
    pub vel_x: i16, pub vel_y: i16, pub vel_z: i16,
}

#[derive(Debug, Clone)]
pub struct AddMobS2CPacket {
    pub entity_id: i32,
    pub entity_type: i32,
    pub x: f64, pub y: f64, pub z: f64,
    pub yaw: u8, pub pitch: u8, pub head_yaw: u8,
    pub vel_x: i16, pub vel_y: i16, pub vel_z: i16,
}

#[derive(Debug, Clone)]
pub struct AddPlayerS2CPacket {
    pub entity_id: i32,
    pub x: f64, pub y: f64, pub z: f64,
    pub yaw: u8, pub pitch: u8,
}

#[derive(Debug, Clone)]
pub struct RemoveEntitiesS2CPacket {
    pub entity_ids: Vec<i32>,
}

#[derive(Debug, Clone)]
pub struct SetEntityMotionS2CPacket {
    pub entity_id: i32,
    pub xa: i16, pub ya: i16, pub za: i16,
}

#[derive(Debug, Clone)]
pub struct TeleportEntityS2CPacket {
    pub entity_id: i32,
    pub x: f64, pub y: f64, pub z: f64,
    pub yaw: u8, pub pitch: u8,
}

#[derive(Debug, Clone)]
pub struct SetHeadRotationS2CPacket {
    pub entity_id: i32,
    pub head_yaw: u8,
}

#[derive(Debug, Clone)]
pub struct SetEntityDataS2CPacket {
    pub entity_id: i32,
}

#[derive(Debug, Clone)]
pub struct LoginPlayPacket {
    pub entity_id: i32,
    pub is_hardcore: bool,
    pub view_distance: i32,
    pub simulation_distance: i32,
    pub reduced_debug: bool,
    pub enable_respawn_screen: bool,
    pub do_limited_crafting: bool,
    pub hashed_seed: i64,
    pub is_debug: bool,
    pub is_flat: bool,
}

#[derive(Debug, Clone)]
pub struct SoundEffectS2CPacket {
    pub sound_id: i32,
    pub sound_category: i32,
    pub x: i32, pub y: i32, pub z: i32,
    pub volume: f32,
    pub pitch: f32,
    pub seed: i64,
}

#[derive(Debug, Clone)]
pub struct SetDefaultSpawnPositionS2CPacket {
    pub x: i32, pub y: i32, pub z: i32,
}

#[derive(Debug, Clone)]
pub struct GameEventS2CPacket {
    pub reason: u8,
    pub param: f32,
}

#[derive(Debug, Clone)]
pub struct ContainerCloseS2CPacket {
    pub container_id: i32,
}

#[derive(Debug, Clone)]
pub struct ContainerSetDataS2CPacket {
    pub container_id: i32,
    pub key: i32,
    pub value: i32,
}

#[derive(Debug, Clone)]
pub struct ContainerAckS2CPacket {
    pub container_id: u8,
    pub uid: i16,
    pub accepted: bool,
}

#[derive(Debug, Clone)]
pub struct PingPongPacket {
    pub id: i64,
}

#[derive(Debug, Clone)]
pub struct EncryptionRequestPacket {
    pub server_id: String,
    pub public_key: Vec<u8>,
    pub verify_token: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct EncryptionResponsePacket {
    pub shared_secret: Vec<u8>,
    pub verify_token: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct JavaSlot {
    pub present: bool,
    pub item_id: i32,
    pub count: u8,
    pub components: Vec<u8>,
}

impl JavaSlot {
    pub fn empty() -> Self {
        Self { present: false, item_id: 0, count: 0, components: Vec::new() }
    }
}

fn write_uuid(buf: &mut Vec<u8>, uuid: &uuid::Uuid) {
    buf.extend_from_slice(uuid.as_bytes());
}

pub fn write_utf8_string(buf: &mut Vec<u8>, s: &str) {
    let bytes = s.as_bytes();
    VarInt::write_to(bytes.len() as i32, buf);
    buf.extend_from_slice(bytes);
}

fn read_uuid(r: &mut &[u8]) -> std::io::Result<uuid::Uuid> {
    let mut bytes = [0u8; 16];
    r.read_exact(&mut bytes)?;
    Ok(uuid::Uuid::from_bytes(bytes))
}

fn read_u8be(r: &mut &[u8]) -> std::io::Result<u8> {
    if r.is_empty() { return Err(std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "eof")); }
    let v = r[0];
    *r = &r[1..];
    Ok(v)
}

fn read_i16be(r: &mut &[u8]) -> std::io::Result<i16> {
    let mut buf = [0u8; 2];
    r.read_exact(&mut buf)?;
    Ok(i16::from_be_bytes(buf))
}

fn read_i32be(r: &mut &[u8]) -> std::io::Result<i32> {
    let mut buf = [0u8; 4];
    r.read_exact(&mut buf)?;
    Ok(i32::from_be_bytes(buf))
}

fn read_i64be(r: &mut &[u8]) -> std::io::Result<i64> {
    let mut buf = [0u8; 8];
    r.read_exact(&mut buf)?;
    Ok(i64::from_be_bytes(buf))
}

fn read_f32be(r: &mut &[u8]) -> std::io::Result<f32> {
    let mut buf = [0u8; 4];
    r.read_exact(&mut buf)?;
    Ok(f32::from_be_bytes(buf))
}

fn read_f64be(r: &mut &[u8]) -> std::io::Result<f64> {
    let mut buf = [0u8; 8];
    r.read_exact(&mut buf)?;
    Ok(f64::from_be_bytes(buf))
}

fn read_utf8_string(r: &mut &[u8]) -> std::io::Result<String> {
    let len = VarInt::read(r)? as usize;
    if len == 0 { return Ok(String::new()); }
    let mut buf = vec![0u8; len];
    r.read_exact(&mut buf)?;
    Ok(String::from_utf8(buf)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?)
}

fn read_varint_array(r: &mut &[u8]) -> std::io::Result<Vec<i32>> {
    let count = VarInt::read(r)?;
    let mut v = Vec::with_capacity(count as usize);
    for _ in 0..count { v.push(VarInt::read(r)?); }
    Ok(v)
}

fn read_nbt(r: &mut &[u8]) -> std::io::Result<Vec<u8>> {
    if r.is_empty() { return Ok(Vec::new()); }
    let tag_type = r[0];
    if tag_type == 0 { *r = &r[1..]; return Ok(Vec::new()); }
    let start_len = r.len();
    let mut depth = 1;
    let mut i = 1;
    if tag_type == 10 {
        if r.len() < 3 { return Ok(Vec::new()); }
        let name_len = i16::from_be_bytes([r[1], r[2]]) as usize;
        i = 3 + name_len;
    }
    let skip = |pos: &mut usize, n: usize| -> bool {
        if *pos + n > r.len() { return false; }
        *pos += n;
        true
    };
    while i < r.len() && depth > 0 {
        let tag = r[i];
        i += 1;
        match tag {
            0 => { depth -= 1; }
            1 => { if !skip(&mut i, 1) { break; } }
            2 => { if !skip(&mut i, 2) { break; } }
            3 => { if !skip(&mut i, 4) { break; } }
            4 => { if !skip(&mut i, 8) { break; } }
            5 => { if !skip(&mut i, 4) { break; } }
            6 => { if !skip(&mut i, 8) { break; } }
            7 => {
                if i + 4 > r.len() { break; }
                let len = i32::from_be_bytes([r[i], r[i+1], r[i+2], r[i+3]]) as usize;
                if !skip(&mut i, 4 + len) { break; }
            }
            8 => {
                if i + 2 > r.len() { break; }
                let len = i16::from_be_bytes([r[i], r[i+1]]) as usize;
                if !skip(&mut i, 2 + len) { break; }
            }
            9 => {
                if i + 5 > r.len() { break; }
                let _inner = r[i]; i += 1;
                let len = i32::from_be_bytes([r[i], r[i+1], r[i+2], r[i+3]]) as usize;
                i += 4;
                depth += len;
            }
            10 => {
                if i + 2 > r.len() { break; }
                let name_len = i16::from_be_bytes([r[i], r[i+1]]) as usize;
                if !skip(&mut i, 2 + name_len) { break; }
                depth += 1;
            }
            11 => {
                if i + 4 > r.len() { break; }
                let len = i32::from_be_bytes([r[i], r[i+1], r[i+2], r[i+3]]) as usize;
                if !skip(&mut i, 4 + len * 4) { break; }
            }
            12 => {
                if i + 4 > r.len() { break; }
                let len = i32::from_be_bytes([r[i], r[i+1], r[i+2], r[i+3]]) as usize;
                if !skip(&mut i, 4 + len * 8) { break; }
            }
            _ => { break; }
        }
    }
    let consumed = start_len.saturating_sub(r.len() - i).min(start_len);
    let result = r[..consumed].to_vec();
    *r = &r[consumed..];
    Ok(result)
}

fn read_java_slot(r: &mut &[u8]) -> std::io::Result<JavaSlot> {
    let present = read_u8be(r)? != 0;
    if !present {
        Ok(JavaSlot::empty())
    } else {
        let item_id = VarInt::read(r)?;
        let count = read_u8be(r)?;
        let components_len = VarInt::read(r)? as usize;
        let mut components = vec![0u8; components_len];
        r.read_exact(&mut components)?;
        Ok(JavaSlot { present, item_id, count, components })
    }
}

fn read_java_metadata(r: &mut &[u8]) -> std::io::Result<()> {
    loop {
        if r.is_empty() { break; }
        let item = r[0];
        if item == 0xFF { *r = &r[1..]; break; }
        *r = &r[1..];
        let type_id = item & 0x1F;
        match type_id {
            0 => { let _ = read_u8be(r)?; }
            1 => { let _ = VarInt::read(r)?; }
            2 => { let _ = read_f32be(r)?; }
            3 => { let _ = read_utf8_string(r)?; }
            4 => { let _ = read_utf8_string(r)?; }
            5 => { let _ = read_nbt(r)?; }
            6 => { let _ = VarInt::read(r)?; }
            7 => { let _ = read_f32be(r)?; let _ = read_f32be(r)?; }
            8 => { let _ = VarInt::read(r)?; }
            9 => { let _ = VarInt::read(r)?; }
            10 => { let _ = VarInt::read(r)?; }
            11 => { let _ = read_u8be(r)?; }
            12 => { let mut u = [0u8; 16]; r.read_exact(&mut u)?; }
            13 => { let _ = VarInt::read(r)?; }
            14 => { let _ = read_nbt(r)?; }
            15 => { let _ = VarInt::read(r)?; }
            16 => { let _ = read_i64be(r)?; }
            17 => { let _ = VarInt::read(r)?; }
            _ => { break; }
        }
    }
    Ok(())
}
