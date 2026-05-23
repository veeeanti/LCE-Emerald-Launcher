use std::fs;
use std::process::Command;
use tauri::AppHandle;
use crate::types::Runner;
use crate::util;
#[tauri::command]
pub fn get_available_runners(app: AppHandle) -> Vec<Runner> {
    let mut runners = Vec::new();
    let mut seen_paths: std::collections::HashSet<String> = std::collections::HashSet::new();
    #[cfg(target_os = "linux")]
    {
        if let Ok(output) = Command::new("which").arg("wine").output() {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !seen_paths.contains(&path) {
                    seen_paths.insert(path.clone());
                    runners.push(Runner {
                        id: "wine".to_string(),
                        name: "System Wine".to_string(),
                        path,
                        r#type: "wine".to_string(),
                    });
                }
            }
        }

        if let Ok(output) = Command::new("ls")
            .arg("/usr/share/emerald-legacy-launcher/wine/bin/wine")
            .output()
        {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !seen_paths.contains(&path) {
                    seen_paths.insert(path.clone());
                    runners.push(Runner {
                        id: "flatpaksucks".to_string(),
                        name: "Default for Flatpak".to_string(),
                        path,
                        r#type: "wine".to_string(),
                    });
                }
            }
        }

        let home = std::env::var("HOME").unwrap_or_default();
        let steam_paths = [
            format!("{}/.local/share/Steam/compatibilitytools.d", home),
            format!("{}/.local/share/Steam/steamapps/common", home),
            "/usr/share/Steam/compatibilitytools.d".to_string()
        ];

        for base_path in steam_paths {
            if let Ok(entries) = fs::read_dir(base_path) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        let name = entry.file_name().to_string_lossy().to_string();
                        if name.to_lowercase().contains("proton") {
                            let path_str = path.to_string_lossy().to_string();
                            if !seen_paths.contains(&path_str) {
                                seen_paths.insert(path_str.clone());
                                runners.push(Runner {
                                    id: format!("proton_{}", name),
                                    name: name,
                                    path: path_str,
                                    r#type: "proton".to_string(),
                                });
                            }
                        }
                    }
                }
            }
        }

        let runners_dir = util::get_app_dir(&app).join("runners");
        let _ = fs::create_dir_all(&runners_dir);
        if let Ok(entries) = fs::read_dir(&runners_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let dir_name = entry.file_name().to_string_lossy().to_string();
                    let wine_bin = path.join("bin").join("wine");
                    let proton_bin = path.join("proton");
                    if proton_bin.exists() {
                        let path_str = path.to_string_lossy().to_string();
                        if !seen_paths.contains(&path_str) {
                            seen_paths.insert(path_str.clone());
                            runners.push(Runner {
                                id: format!("downloaded_{}", dir_name),
                                name: format!("{} (downloaded)", dir_name),
                                path: path_str,
                                r#type: "proton".to_string(),
                            });
                        }
                    } else if wine_bin.exists() {
                        let path_str = wine_bin.to_string_lossy().to_string();
                        if !seen_paths.contains(&path_str) {
                            seen_paths.insert(path_str.clone());
                            runners.push(Runner {
                                id: format!("downloaded_{}", dir_name),
                                name: format!("{} (downloaded)", dir_name),
                                path: path_str,
                                r#type: "wine".to_string(),
                            });
                        }
                    }
                }
            }
        }
    }

    runners
}
