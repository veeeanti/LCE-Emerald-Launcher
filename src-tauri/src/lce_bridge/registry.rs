use std::collections::HashMap;
use std::sync::OnceLock;

static MAPPINGS: OnceLock<MappingRegistry> = OnceLock::new();

pub fn registry() -> &'static MappingRegistry {
    MAPPINGS.get_or_init(|| {
        let mut r = MappingRegistry {
            block_map: HashMap::new(),
            biome_map: HashMap::new(),
            fallback_block: u16::MAX,
        };
        r.load_default();
        r
    })
}

pub struct MappingRegistry {
    block_map: HashMap<i32, u16>,
    biome_map: HashMap<String, u8>,
    fallback_block: u16,
}

impl MappingRegistry {
    fn load_default(&mut self) {
        for (java_id, lce_id) in DEFAULT_BLOCK_MAP {
            self.block_map.insert(*java_id, *lce_id);
        }
        for (name, id) in DEFAULT_BIOME_MAP {
            self.biome_map.insert(name.to_string(), *id);
        }
        self.fallback_block = (1 << 8) | 0;
    }

    pub fn is_air(&self, java_state_id: i32) -> bool {
        java_state_id == 0 || java_state_id == 1024
    }

    pub fn get_lce(&self, java_state_id: i32) -> u16 {
        self.block_map.get(&java_state_id).copied().unwrap_or(self.fallback_block)
    }

    pub fn get_lce_id(&self, java_state_id: i32) -> u8 {
        (self.get_lce(java_state_id) >> 8) as u8
    }

    pub fn get_lce_data(&self, java_state_id: i32) -> u8 {
        (self.get_lce(java_state_id) & 0xFF) as u8
    }

    pub fn get_biome(&self, java_name: &str) -> u8 {
        self.biome_map.get(java_name).copied().unwrap_or(1)
    }

    pub fn register_block(&mut self, java_state_id: i32, packed_lce: u16) {
        self.block_map.insert(java_state_id, packed_lce);
    }
}

pub fn get_lce_block_id(java_state_id: i32) -> u8 {
    registry().get_lce_id(java_state_id)
}

pub fn get_lce_block_data(java_state_id: i32) -> u8 {
    registry().get_lce_data(java_state_id)
}

pub fn get_lce_biome(java_name: &str) -> u8 {
    registry().get_biome(java_name)
}

static DEFAULT_BLOCK_MAP: &[(i32, u16)] = &[
    (0, 0), (1, 1<<8|0), (2, 2<<8|0), (3, 3<<8|0), (4, 4<<8|0),
    (5, 5<<8|0), (5, 5<<8|1), (5, 5<<8|2), (5, 5<<8|3), (5, 5<<8|4), (5, 5<<8|5),
    (7, 7<<8|0), (8, 8<<8|0), (9, 9<<8|0), (10, 10<<8|0), (11, 11<<8|0),
    (12, 12<<8|0), (13, 13<<8|0), (14, 14<<8|0), (15, 15<<8|0),
    (16, 16<<8|0), (16, 16<<8|1), (16, 16<<8|2), (16, 16<<8|3), (16, 16<<8|4), (16, 16<<8|5),
    (17, 17<<8|0), (17, 17<<8|1), (17, 17<<8|2), (17, 17<<8|3),
    (18, 18<<8|0), (18, 18<<8|1), (18, 18<<8|2), (18, 18<<8|3),
    (19, 19<<8|0), (19, 19<<8|1), (19, 19<<8|2),
    (20, 20<<8|0), (21, 21<<8|0),
    (22, 22<<8|0), (24, 24<<8|0), (24, 24<<8|1), (24, 24<<8|2),
    (25, 25<<8|0), (35, 35<<8|0), (35, 35<<8|1), (35, 35<<8|2), (35, 35<<8|3),
    (35, 35<<8|4), (35, 35<<8|5), (35, 35<<8|6), (35, 35<<8|7),
    (35, 35<<8|8), (35, 35<<8|9), (35, 35<<8|10), (35, 35<<8|11),
    (35, 35<<8|12), (35, 35<<8|13), (35, 35<<8|14), (35, 35<<8|15),
    (41, 41<<8|0), (42, 42<<8|0), (43, 43<<8|0), (43, 43<<8|8),
    (44, 44<<8|0), (44, 44<<8|1), (44, 44<<8|2), (44, 44<<8|3),
    (44, 44<<8|4), (44, 44<<8|5), (44, 44<<8|6), (44, 44<<8|7),
    (45, 45<<8|0), (46, 46<<8|0), (47, 47<<8|0), (48, 48<<8|0),
    (49, 49<<8|0), (50, 50<<8|0), (50, 50<<8|1), (50, 50<<8|2), (50, 50<<8|3), (50, 50<<8|4), (50, 50<<8|5),
    (52, 52<<8|0), (53, 53<<8|0), (53, 53<<8|1), (53, 53<<8|2), (53, 53<<8|3),
    (56, 56<<8|0), (57, 57<<8|0), (58, 58<<8|0), (60, 60<<8|0),
    (61, 61<<8|0), (62, 62<<8|0), (73, 73<<8|0), (74, 74<<8|0),
    (78, 78<<8|0), (79, 79<<8|0), (80, 80<<8|0), (81, 81<<8|0),
    (82, 82<<8|0), (83, 83<<8|0), (84, 84<<8|0), (85, 85<<8|0),
    (86, 86<<8|0), (87, 87<<8|0), (88, 88<<8|0), (89, 89<<8|0),
    (91, 91<<8|0), (95, 95<<8|0), (97, 97<<8|0), (98, 98<<8|0),
    (98, 98<<8|1), (98, 98<<8|2), (98, 98<<8|3),
    (99, 99<<8|0), (100, 100<<8|0), (101, 101<<8|0), (102, 102<<8|0),
    (103, 103<<8|0), (107, 107<<8|0), (108, 108<<8|0), (109, 109<<8|0),
    (110, 110<<8|0), (111, 111<<8|0), (112, 112<<8|0), (113, 113<<8|0),
    (114, 114<<8|0), (121, 121<<8|0), (123, 123<<8|0), (124, 124<<8|0),
    (125, 125<<8|0), (126, 126<<8|0), (128, 128<<8|0), (129, 129<<8|0),
    (130, 130<<8|0), (133, 133<<8|0), (134, 134<<8|0), (135, 135<<8|0),
    (136, 136<<8|0), (138, 138<<8|0), (139, 139<<8|0), (140, 140<<8|0),
    (141, 141<<8|0), (142, 142<<8|0), (144, 144<<8|0), (145, 145<<8|0),
    (147, 147<<8|0), (148, 148<<8|0), (151, 151<<8|0), (152, 152<<8|0),
    (153, 153<<8|0), (154, 154<<8|0), (155, 155<<8|0), (156, 156<<8|0),
    (158, 158<<8|0), (159, 159<<8|0), (159, 159<<8|1), (159, 159<<8|2), (159, 159<<8|3),
    (159, 159<<8|4), (159, 159<<8|5), (159, 159<<8|6), (159, 159<<8|7),
    (159, 159<<8|8), (159, 159<<8|9), (159, 159<<8|10), (159, 159<<8|11),
    (159, 159<<8|12), (159, 159<<8|13), (159, 159<<8|14), (159, 159<<8|15),
    (161, 161<<8|0), (161, 161<<8|1), (162, 162<<8|0), (162, 162<<8|1),
    (163, 163<<8|0), (164, 164<<8|0), (165, 165<<8|0), (167, 167<<8|0),
    (168, 168<<8|0), (168, 168<<8|1), (168, 168<<8|2),
    (169, 169<<8|0), (170, 170<<8|0), (171, 171<<8|0), (171, 171<<8|1),
    (171, 171<<8|2), (171, 171<<8|3), (171, 171<<8|4), (171, 171<<8|5),
    (171, 171<<8|6), (171, 171<<8|7), (171, 171<<8|8), (171, 171<<8|9),
    (171, 171<<8|10), (171, 171<<8|11), (171, 171<<8|12), (171, 171<<8|13),
    (171, 171<<8|14), (171, 171<<8|15),
    (172, 172<<8|0), (173, 173<<8|0),
];

static DEFAULT_BIOME_MAP: &[(&str, u8)] = &[
    ("minecraft:plains", 1), ("minecraft:desert", 2), ("minecraft:forest", 4),
    ("minecraft:taiga", 5), ("minecraft:swamp", 6), ("minecraft:river", 7),
    ("minecraft:frozen_river", 7), ("minecraft:ocean", 0),
    ("minecraft:cold_ocean", 0), ("minecraft:deep_ocean", 0),
    ("minecraft:frozen_ocean", 10), ("minecraft:deep_frozen_ocean", 10),
    ("minecraft:beach", 16), ("minecraft:stone_beach", 16),
    ("minecraft:snowy_beach", 16), ("minecraft:birch_forest", 27),
    ("minecraft:dark_forest", 29), ("minecraft:flower_forest", 28),
    ("minecraft:ice_spikes", 12), ("minecraft:jungle", 21),
    ("minecraft:mushroom_fields", 14), ("minecraft:nether_wastes", 8),
    ("minecraft:savanna", 36), ("minecraft:snowy_taiga", 30),
    ("minecraft:snowy_plains", 11), ("minecraft:sunflower_plains", 1),
    ("minecraft:the_end", 9), ("minecraft:the_void", 9),
    ("minecraft:warm_ocean", 0), ("minecraft:lukewarm_ocean", 0),
    ("minecraft:badlands", 39), ("minecraft:wooded_badlands", 39),
    ("minecraft:eroded_badlands", 39), ("minecraft:meadow", 1),
    ("minecraft:windswept_hills", 15), ("minecraft:windswept_forest", 15),
    ("minecraft:windswept_gravelly_hills", 15), ("minecraft:windswept_savanna", 36),
    ("minecraft:grove", 5), ("minecraft:snowy_slopes", 12),
    ("minecraft:jagged_peaks", 24), ("minecraft:frozen_peaks", 24),
    ("minecraft:stony_peaks", 15), ("minecraft:cherry_grove", 28),
    ("minecraft:mangrove_swamp", 6), ("minecraft:deep_dark", 2),
    ("minecraft:dripstone_caves", 2), ("minecraft:lush_caves", 2),
];
