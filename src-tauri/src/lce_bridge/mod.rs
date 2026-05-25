pub mod config;
pub mod util;
pub mod lce_packets;
pub mod lce_codec;
pub mod java_protocol;
pub mod java_session;
pub mod registry;
pub mod chunk;
pub mod crafting;
pub mod bridge;
pub mod msa_auth;

use crate::lce_bridge::config::BridgeConfig;
use crate::lce_bridge::lce_codec::run_lce_server;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct BridgeHandle {
    cancel: tokio::sync::watch::Sender<bool>,
}

static BRIDGE_INSTANCE: std::sync::OnceLock<Arc<Mutex<Option<BridgeHandle>>>> = std::sync::OnceLock::new();

fn bridge_instance() -> &'static Arc<Mutex<Option<BridgeHandle>>> {
    BRIDGE_INSTANCE.get_or_init(|| Arc::new(Mutex::new(None)))
}

pub async fn start_bridge(config: BridgeConfig) -> Result<String, String> {
    let (cancel_tx, cancel_rx) = tokio::sync::watch::channel(false);
    let handle = BridgeHandle { cancel: cancel_tx.clone() };
    *bridge_instance().lock().await = Some(handle);
    let config_clone = config.clone();
    tokio::spawn(async move {
        let mut cancel_rx = cancel_rx;
        let server_fut = run_lce_server(config_clone);
        tokio::select! {
            _ = server_fut => {}
            _ = cancel_rx.changed() => {}
        }
    });
    Ok(format!("LCEBridge started on {}:{}", config.lce.listen_address, config.lce.port))
}

pub async fn stop_bridge() -> Result<String, String> {
    let mut guard = bridge_instance().lock().await;
    if let Some(handle) = guard.take() {
        let _ = handle.cancel.send(true);
        Ok("LCEBridge stopped".to_string())
    } else {
        Err("LCEBridge is not running".to_string())
    }
}

pub async fn bridge_status() -> Result<String, String> {
    let guard = bridge_instance().lock().await;
    if guard.is_some() {
        Ok("running".to_string())
    } else {
        Ok("stopped".to_string())
    }
}
