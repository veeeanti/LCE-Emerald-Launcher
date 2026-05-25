#[derive(Debug, Clone)]
pub enum LcePacket {
    KeepAlive(KeepAlivePacket),
    Login(LoginPacket),
    PreLogin(PreLoginPacket),
    Chat(ChatPacket),
    Disconnect(DisconnectPacket),
    Animate(AnimatePacket),
    MovePlayer(MovePlayerPacket),
    PlayerAction(PlayerActionPacket),
    Interact(InteractPacket),
    UseItem(UseItemPacket),
    PlayerCommand(PlayerCommandPacket),
    SetCarriedItem(SetCarriedItemPacket),
    PlayerAbilities(PlayerAbilitiesPacket),
    DebugOptions(DebugOptionsPacket),
    ClientCommand(ClientCommandPacket),
    RespawnRequest(RespawnRequestPacket),
    ContainerClose(ContainerClosePacket),
    ContainerClick(ContainerClickPacket),
    ContainerButtonClick(ContainerButtonClickPacket),
    CraftItem(CraftItemPacket),
    SetTime(SetTimePacket),
    SetSpawnPosition(SetSpawnPositionPacket),
    SetHealth(SetHealthPacket),
    GameEvent(GameEventPacket),
    EntityEvent(EntityEventPacket),
    ContainerOpen(ContainerOpenPacket),
    ContainerSetSlot(ContainerSetSlotPacket),
    ContainerSetContent(ContainerSetContentPacket),
    ContainerSetData(ContainerSetDataPacket),
    ContainerAck(ContainerAckPacket),
    ChunkVisibility(ChunkVisibilityPacket),
    ChunkVisibilityArea(ChunkVisibilityAreaPacket),
    BlockRegionUpdate(BlockRegionUpdatePacket),
    TileUpdate(TileUpdatePacket),
    AddPlayer(AddPlayerPacket),
    AddEntity(AddEntityPacket),
    AddMob(AddMobPacket),
    TeleportEntity(TeleportEntityPacket),
    RotateHead(RotateHeadPacket),
    SetEntityMotion(SetEntityMotionPacket),
    RemoveEntities(RemoveEntitiesPacket),
    SetEntityData(SetEntityDataPacket),
    Raw { id: u8, data: Vec<u8> },
}

impl LcePacket {
    pub fn id(&self) -> u8 {
        match self {
            LcePacket::KeepAlive(_) => 0,
            LcePacket::Login(_) => 1,
            LcePacket::PreLogin(_) => 2,
            LcePacket::Chat(_) => 3,
            LcePacket::Disconnect(_) => 255,
            LcePacket::Animate(_) => 18,
            LcePacket::MovePlayer(_) => 10,
            LcePacket::PlayerAction(_) => 14,
            LcePacket::Interact(_) => 7,
            LcePacket::UseItem(_) => 15,
            LcePacket::PlayerCommand(_) => 19,
            LcePacket::SetCarriedItem(_) => 16,
            LcePacket::PlayerAbilities(_) => 202,
            LcePacket::DebugOptions(_) => 152,
            LcePacket::ClientCommand(_) => 205,
            LcePacket::RespawnRequest(_) => 9,
            LcePacket::ContainerClose(_) => 101,
            LcePacket::ContainerClick(_) => 102,
            LcePacket::ContainerButtonClick(_) => 108,
            LcePacket::CraftItem(_) => 150,
            LcePacket::SetTime(_) => 4,
            LcePacket::SetSpawnPosition(_) => 6,
            LcePacket::SetHealth(_) => 8,
            LcePacket::GameEvent(_) => 70,
            LcePacket::EntityEvent(_) => 38,
            LcePacket::ContainerOpen(_) => 100,
            LcePacket::ContainerSetSlot(_) => 103,
            LcePacket::ContainerSetContent(_) => 104,
            LcePacket::ContainerSetData(_) => 105,
            LcePacket::ContainerAck(_) => 106,
            LcePacket::ChunkVisibility(_) => 50,
            LcePacket::ChunkVisibilityArea(_) => 155,
            LcePacket::BlockRegionUpdate(_) => 51,
            LcePacket::TileUpdate(_) => 53,
            LcePacket::AddPlayer(_) => 20,
            LcePacket::AddEntity(_) => 23,
            LcePacket::AddMob(_) => 24,
            LcePacket::TeleportEntity(_) => 34,
            LcePacket::RotateHead(_) => 35,
            LcePacket::SetEntityMotion(_) => 28,
            LcePacket::RemoveEntities(_) => 29,
            LcePacket::SetEntityData(_) => 40,
            LcePacket::Raw { id, .. } => *id,
        }
    }
}

#[derive(Debug, Clone)]
pub struct KeepAlivePacket {
    pub keep_alive_id: i32,
}

#[derive(Debug, Clone)]
pub struct LoginPacket {
    pub protocol_version: i32,
    pub username: String,
    pub map_seed: u64,
    pub game_type: i32,
    pub world_name: String,
    pub dimension: i32,
    pub difficulty: i32,
    pub max_players: u8,
    pub world_width: i32,
    pub world_length: i32,
}

#[derive(Debug, Clone)]
pub struct PreLoginPacket {
    pub net_version: i16,
    pub player_name: String,
    pub offline_xuid: i64,
    pub online_xuid: i64,
}

#[derive(Debug, Clone)]
pub struct ChatPacket {
    pub message_type: u8,
    pub string_args: Vec<String>,
    pub int_args: Vec<i32>,
}

impl ChatPacket {
    pub fn set_message(msg: &str) -> Self {
        Self {
            message_type: 1,
            string_args: vec![msg.to_string()],
            int_args: vec![],
        }
    }
}

#[derive(Debug, Clone)]
pub struct DisconnectPacket {
    pub reason: u8,
}

#[derive(Debug, Clone)]
pub struct AnimatePacket {
    pub entity_id: i32,
    pub action: u8,
}

#[derive(Debug, Clone)]
pub struct MovePlayerPacket {
    pub id: u8,
    pub x: f64,
    pub y: f64,
    pub y_view: f64,
    pub z: f64,
    pub yaw: f32,
    pub pitch: f32,
    pub flags: u8,
}

#[derive(Debug, Clone)]
pub struct PlayerActionPacket {
    pub action: u8,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub face: u8,
}

#[derive(Debug, Clone)]
pub struct PlayerCommandPacket {
    pub entity_id: i32,
    pub action: u8,
    pub data: i32,
}

#[derive(Debug, Clone)]
pub struct InteractPacket {
    pub source: u8,
    pub target: i32,
    pub action: u8,
}

#[derive(Debug, Clone)]
pub struct UseItemPacket {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub face: u8,
    pub item: LceItemStack,
    pub click_x: f32,
    pub click_y: f32,
    pub click_z: f32,
}

#[derive(Debug, Clone)]
pub struct SetCarriedItemPacket {
    pub slot: u8,
}

#[derive(Debug, Clone)]
pub struct PlayerAbilitiesPacket {
    pub flags: u8,
    pub fly_speed: f32,
    pub walk_speed: f32,
}

#[derive(Debug, Clone)]
pub struct DebugOptionsPacket {
    pub options_mask: u32,
}

#[derive(Debug, Clone)]
pub struct ClientCommandPacket {
    pub action: u8,
}

#[derive(Debug, Clone)]
pub struct RespawnRequestPacket;

#[derive(Debug, Clone)]
pub struct ContainerClosePacket {
    pub container_id: u8,
}

#[derive(Debug, Clone)]
pub struct ContainerClickPacket {
    pub container_id: u8,
    pub slot_num: i16,
    pub button_num: u8,
    pub uid: i16,
    pub click_type: u8,
    pub item: LceItemStack,
}

#[derive(Debug, Clone)]
pub struct ContainerButtonClickPacket {
    pub container_id: u8,
    pub button_id: u8,
}

#[derive(Debug, Clone)]
pub struct CraftItemPacket {
    pub uid: i16,
    pub recipe: u8,
}

#[derive(Debug, Clone)]
pub struct SetTimePacket {
    pub game_time: i64,
    pub day_time: i64,
}

#[derive(Debug, Clone)]
pub struct SetSpawnPositionPacket {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

#[derive(Debug, Clone)]
pub struct SetHealthPacket {
    pub health: f32,
    pub food: i32,
    pub saturation: f32,
    pub damage_source: u8,
}

#[derive(Debug, Clone)]
pub struct GameEventPacket {
    pub reason: u8,
    pub param: f32,
}

#[derive(Debug, Clone)]
pub struct EntityEventPacket {
    pub entity_id: i32,
    pub event_id: u8,
}

#[derive(Debug, Clone)]
pub struct ContainerOpenPacket {
    pub container_id: u8,
    pub container_type: u8,
    pub size: u8,
    pub custom_name: String,
    pub title: String,
    pub entity_id: i32,
}

#[derive(Debug, Clone)]
pub struct ContainerSetSlotPacket {
    pub container_id: u8,
    pub slot: i16,
    pub item: LceItemStack,
}

#[derive(Debug, Clone)]
pub struct ContainerSetContentPacket {
    pub container_id: u8,
    pub items: Vec<LceItemStack>,
}

#[derive(Debug, Clone)]
pub struct ContainerSetDataPacket {
    pub container_id: u8,
    pub id: i16,
    pub value: i16,
}

#[derive(Debug, Clone)]
pub struct ContainerAckPacket {
    pub container_id: u8,
    pub uid: i16,
    pub accepted: bool,
}

#[derive(Debug, Clone)]
pub struct ChunkVisibilityPacket {
    pub chunk_x: i32,
    pub chunk_z: i32,
    pub visible: bool,
}

#[derive(Debug, Clone)]
pub struct ChunkVisibilityAreaPacket {
    pub min_cx: i32,
    pub max_cx: i32,
    pub min_cz: i32,
    pub max_cz: i32,
}

#[derive(Debug, Clone)]
pub struct BlockRegionUpdatePacket {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub xs: u8,
    pub ys: u8,
    pub zs: u8,
    pub level_idx: u8,
    pub is_full_chunk: bool,
    pub compressed_data: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct TileUpdatePacket {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub block: u16,
    pub data: u16,
    pub level_idx: u8,
}

#[derive(Debug, Clone)]
pub struct AddPlayerPacket {
    pub entity_id: i32,
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub yaw: u8,
    pub pitch: u8,
    pub head_yaw: u8,
    pub carried_item: u16,
    pub offline_xuid: i64,
    pub online_xuid: i64,
    pub player_index: i32,
    pub skin_id: String,
    pub cape_id: String,
    pub game_privileges: u32,
    pub metadata: Vec<EntityDataValue>,
}

#[derive(Debug, Clone)]
pub struct AddEntityPacket {
    pub entity_id: i32,
    pub entity_type: u8,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub yaw: u8,
    pub pitch: u8,
    pub data: i32,
    pub motion_x: i16,
    pub motion_y: i16,
    pub motion_z: i16,
}

#[derive(Debug, Clone)]
pub struct AddMobPacket {
    pub entity_id: i32,
    pub entity_type: u8,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub yaw: u8,
    pub pitch: u8,
    pub head_yaw: u8,
    pub motion_x: i16,
    pub motion_y: i16,
    pub motion_z: i16,
    pub metadata: Vec<EntityDataValue>,
}

#[derive(Debug, Clone)]
pub struct TeleportEntityPacket {
    pub entity_id: i32,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub yaw: u8,
    pub pitch: u8,
}

#[derive(Debug, Clone)]
pub struct RotateHeadPacket {
    pub entity_id: i32,
    pub y_head_rot: u8,
}

#[derive(Debug, Clone)]
pub struct SetEntityMotionPacket {
    pub entity_id: i32,
    pub xa: i16,
    pub ya: i16,
    pub za: i16,
}

#[derive(Debug, Clone)]
pub struct RemoveEntitiesPacket {
    pub entity_ids: Vec<i32>,
}

#[derive(Debug, Clone)]
pub struct SetEntityDataPacket {
    pub entity_id: i32,
    pub values: Vec<EntityDataValue>,
}

#[derive(Debug, Clone)]
pub struct EntityDataValue {
    pub id: u8,
    pub data_type: u8,
    pub value: EntityDataValueType,
}

#[derive(Debug, Clone)]
pub enum EntityDataValueType {
    Byte(u8),
    Short(i16),
    Int(i32),
    Float(f32),
    String(String),
    ItemStack(LceItemStack),
}

#[derive(Debug, Clone, Default)]
pub struct LceItemStack {
    pub id: i16,
    pub count: u8,
    pub damage: i16,
}

impl LceItemStack {
    pub fn is_empty(&self) -> bool {
        self.id <= 0 || self.count == 0
    }

    pub fn empty() -> Self {
        Self { id: 0, count: 0, damage: 0 }
    }
}
