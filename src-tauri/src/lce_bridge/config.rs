use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeConfig {
    pub lce: LceConfig,
    pub remote: RemoteConfig,
    pub world: WorldConfig,
    pub performance: PerformanceConfig,
    pub logging: LoggingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LceConfig {
    pub listen_address: String,
    pub port: u16,
    pub motd: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteConfig {
    pub host: String,
    pub port: u16,
    pub auth_type: String,
    pub protocol_version: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldConfig {
    pub render_distance: i32,
    pub simulation_distance: i32,
    pub force_gamemode: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    pub chunk_threads: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
}

impl Default for BridgeConfig {
    fn default() -> Self {
        Self {
            lce: LceConfig {
                listen_address: "0.0.0.0".to_string(),
                port: 25656,
                motd: "LCEBridge".to_string(),
            },
            remote: RemoteConfig {
                host: "127.0.0.1".to_string(),
                port: 25565,
                auth_type: "offline".to_string(),
                protocol_version: 47,
            },
            world: WorldConfig {
                render_distance: 8,
                simulation_distance: 8,
                force_gamemode: None,
            },
            performance: PerformanceConfig {
                chunk_threads: 2,
            },
            logging: LoggingConfig {
                level: "info".to_string(),
            },
        }
    }
}
