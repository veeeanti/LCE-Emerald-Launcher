use tauri::{AppHandle, Emitter};
use crate::lce_bridge::{start_bridge, stop_bridge, bridge_status as lce_bridge_status, config::BridgeConfig, msa_auth};
use std::sync::OnceLock;

static AUTH_PROFILE: OnceLock<msa_auth::MinecraftProfile> = OnceLock::new();

#[allow(dead_code)]
pub fn get_auth_profile() -> Option<&'static msa_auth::MinecraftProfile> {
    AUTH_PROFILE.get()
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn bridge_start(
    app: AppHandle,
    listen_address: String,
    port: u16,
    remote_host: String,
    remote_port: u16,
    auth_type: String,
    remote_protocol_version: i32,
) -> Result<String, String> {
    if auth_type == "online" {
        let mut rx = msa_auth::subscribe_device_code();
        let app2 = app.clone();
        tokio::spawn(async move {
            while let Ok(msg) = rx.recv().await {
                let _ = app2.emit("lcebridge-msa-code", &msg);
            }
        });
        let profile = msa_auth::authenticate().await
            .map_err(|e| format!("MSA auth failed: {e}"))?;
        let _ = AUTH_PROFILE.set(profile);
    }

    let config = BridgeConfig {
        lce: crate::lce_bridge::config::LceConfig {
            listen_address,
            port,
            motd: "LCEBridge".to_string(),
        },
        remote: crate::lce_bridge::config::RemoteConfig {
            host: remote_host,
            port: remote_port,
            auth_type,
            protocol_version: remote_protocol_version,
        },
        ..Default::default()
    };
    start_bridge(config).await
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn bridge_stop() -> Result<String, String> {
    stop_bridge().await
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn bridge_status() -> Result<String, String> {
    lce_bridge_status().await
}
