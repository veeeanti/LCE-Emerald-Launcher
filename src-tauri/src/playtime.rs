use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::AppHandle;
use serde::{Serialize, Deserialize};
use crate::util;
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PlaytimeSession {
    pub start: u64,
    pub end: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PlaytimeData {
    pub sessions: HashMap<String, Vec<PlaytimeSession>>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PlaytimeResponse {
    pub total_seconds: u64,
    pub week_seconds: u64,
    pub day_seconds: u64,
}

fn playtime_path(app: &AppHandle) -> PathBuf {
    util::get_app_dir(app).join("playtime.json")
}

pub fn load(app: &AppHandle) -> PlaytimeData {
    let path = playtime_path(app);
    if path.exists() {
        let content = std::fs::read_to_string(&path).unwrap_or_default();
        serde_json::from_str(&content).unwrap_or(PlaytimeData { sessions: HashMap::new() })
    } else {
        PlaytimeData { sessions: HashMap::new() }
    }
}

pub fn save(app: &AppHandle, data: &PlaytimeData) {
    let path = playtime_path(app);
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(content) = serde_json::to_string_pretty(data) {
        let _ = std::fs::write(&path, content);
    }
}

pub fn record_session(app: &AppHandle, instance_id: &str, start: u64, end: u64) {
    let mut data = load(app);
    let sessions = data.sessions.entry(instance_id.to_string()).or_default();
    sessions.push(PlaytimeSession { start, end });
    save(app, &data);
}

pub fn get_playtime(app: &AppHandle, instance_id: &str) -> PlaytimeResponse {
    let data = load(app);
    let sessions = data.sessions.get(instance_id);
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let week_ago = now - 7 * 24 * 60 * 60;
    let day_ago = now - 24 * 60 * 60;
    let total_seconds: u64 = sessions.map_or(0, |s| s.iter().map(|s| s.end - s.start).sum());
    let week_seconds: u64 = sessions.map_or(0, |s| s.iter()
        .filter(|s| s.start >= week_ago)
        .map(|s| s.end - s.start)
        .sum());
    let day_seconds: u64 = sessions.map_or(0, |s| s.iter()
        .filter(|s| s.start >= day_ago)
        .map(|s| s.end - s.start)
        .sum());

    PlaytimeResponse { total_seconds, week_seconds, day_seconds }
}
