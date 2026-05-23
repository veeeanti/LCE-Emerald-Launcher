use std::fs;
use crate::util;
pub fn kill_process_tree(app: &tauri::AppHandle, instance_id: &str) {
    let root = util::get_app_dir(app);
    let instance_dir = root.join("instances").join(instance_id);
    let target = util::unix_path_to_wine_z_path(&instance_dir.join("Minecraft.Client.exe"));
    let Ok(entries) = fs::read_dir("/proc") else { return };
    for entry in entries.flatten() {
        let Ok(pid) = entry.file_name().to_string_lossy().parse::<u32>() else { continue };
        let cmdline = fs::read_to_string(format!("/proc/{}/cmdline", pid))
            .unwrap_or_default();
        if cmdline.contains(&*target) {
            unsafe { libc::kill(pid as i32, libc::SIGKILL); }
        }
    }
}
