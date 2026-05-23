use std::fs;
use std::path::PathBuf;
use tauri::AppHandle;
use crate::types::{WorkshopInstallRequest, InstalledWorkshopPackage, InstalledPackageEntry};
use crate::config;
use crate::util;
#[tauri::command]
pub async fn workshop_install(app: AppHandle, request: WorkshopInstallRequest) -> Result<(), String> {
    let instance_dir = util::get_instance_working_dir(&app, &request.instance_id);
    if !instance_dir.exists() {
        return Err("Instance not installed".into());
    }
    let root = util::get_app_dir(&app);
    let media_dir = instance_dir.join("Windows64Media");
    let dlc_dir   = media_dir.join("DLC");
    let game_hdd  = instance_dir.join("Windows64").join("GameHDD");
    let mob_dir   = instance_dir.join("Common").join("res").join("mob");
    let wf_path   = instance_dir.join("workshop_files.json");
    let wp_path   = instance_dir.join("workshop_packages.json");
    let mut workshop_files: Vec<String> = fs::read_to_string(&wf_path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();

    let mut workshop_packages: Vec<InstalledWorkshopPackage> = fs::read_to_string(&wp_path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();

    workshop_packages.retain(|p| p.id != request.package_id);
    let raw_base = format!("https://raw.githubusercontent.com/LCE-Hub/LCE-Workshop/refs/heads/main/{}", request.package_id);
    let tmp_dir  = root.join(format!("workshop_tmp_{}", request.package_id));
    fs::create_dir_all(&tmp_dir).map_err(|e| e.to_string())?;
    let mut pkg_dirs: Vec<String> = Vec::new();
    for (zip_name, placeholder) in &request.zips {
        let zip_url = format!("{}/{}", raw_base, zip_name);
        let zip_tmp = tmp_dir.join(zip_name);
        let response = reqwest::get(&zip_url).await.map_err(|e| e.to_string())?;
        if !response.status().is_success() {
            let _ = fs::remove_dir_all(&tmp_dir);
            return Err(format!("Failed to download {}: HTTP {}", zip_name, response.status()));
        }
        let bytes = response.bytes().await.map_err(|e| e.to_string())?;
        fs::write(&zip_tmp, &bytes).map_err(|e| e.to_string())?;

        let dest_dir = if placeholder.is_empty() {
            instance_dir.clone()
        } else {
            let resolved = instance_dir.clone().join(placeholder
                .replace("{MediaDir}", media_dir.to_str().unwrap_or(""))
                .replace("{DLCDir}",   dlc_dir.to_str().unwrap_or(""))
                .replace("{GameHDD}",  game_hdd.to_str().unwrap_or(""))
                .replace("{MobDir}",   mob_dir.to_str().unwrap_or("")));
            PathBuf::from(resolved)
        };

        fs::create_dir_all(&dest_dir).map_err(|e| e.to_string())?;
        #[cfg(target_os = "linux")]
        {
            let status = std::process::Command::new("bsdtar")
                .args(["-xf", zip_tmp.to_str().unwrap(), "-C", dest_dir.to_str().unwrap()])
                .status()
                .map_err(|e| e.to_string())?;
            if !status.success() {
                let _ = fs::remove_dir_all(&tmp_dir);
                return Err(format!("Extraction failed for {}", zip_name));
            }
        }
        #[cfg(not(target_os = "linux"))]
        {
            let status = std::process::Command::new("tar")
                .args(["-xf", zip_tmp.to_str().unwrap(), "-C", dest_dir.to_str().unwrap()])
                .status()
                .map_err(|e| e.to_string())?;
            if !status.success() {
                let _ = fs::remove_dir_all(&tmp_dir);
                return Err(format!("Extraction failed for {}", zip_name));
            }
        }

        let dest_str = dest_dir.to_string_lossy().to_string();
        if !workshop_files.contains(&dest_str) {
            workshop_files.push(dest_str.clone());
        }
        if !pkg_dirs.contains(&dest_str) {
            pkg_dirs.push(dest_str);
        }
    }

    let _ = fs::remove_dir_all(&tmp_dir);

    if let Ok(json) = serde_json::to_string(&workshop_files) {
        let _ = fs::write(&wf_path, json);
    }

    workshop_packages.push(InstalledWorkshopPackage {
        id: request.package_id.clone(),
        version: request.version.clone(),
        dirs: pkg_dirs,
    });
    if let Ok(json) = serde_json::to_string(&workshop_packages) {
        let _ = fs::write(&wp_path, json);
    }

    Ok(())
}

#[tauri::command]
pub async fn workshop_uninstall(app: AppHandle, instance_id: String, package_id: String) -> Result<(), String> {
    let instance_dir = util::get_instance_working_dir(&app, &instance_id);
    let wp_path = instance_dir.join("workshop_packages.json");
    let wf_path = instance_dir.join("workshop_files.json");
    let mut packages: Vec<InstalledWorkshopPackage> = fs::read_to_string(&wp_path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();

    if let Some(pkg) = packages.iter().find(|p| p.id == package_id) {
        for dir in &pkg.dirs {
            let path = PathBuf::from(dir);
            if path.is_dir() {
                let _ = fs::remove_dir_all(&path);
            } else if path.is_file() {
                let _ = fs::remove_file(&path);
            }
        }
    }

    let removed_dirs: std::collections::HashSet<String> = packages
        .iter()
        .find(|p| p.id == package_id)
        .map(|p| p.dirs.iter().cloned().collect())
        .unwrap_or_default();

    packages.retain(|p| p.id != package_id);
    if let Ok(json) = serde_json::to_string(&packages) {
        let _ = fs::write(&wp_path, json);
    }

    let mut workshop_files: Vec<String> = fs::read_to_string(&wf_path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    workshop_files.retain(|f| !removed_dirs.contains(f));
    if let Ok(json) = serde_json::to_string(&workshop_files) {
        let _ = fs::write(&wf_path, json);
    }

    Ok(())
}

#[tauri::command]
pub fn workshop_list_installed(app: AppHandle) -> Vec<InstalledPackageEntry> {
    let root = util::get_app_dir(&app);
    let mut result = Vec::new();
    let mut instance_dirs = vec![root.join("instances")];
    let config_val = config::load_config_raw(app.clone());
    if let Some(editions) = config_val.custom_editions {
        for ed in editions {
            if let Some(path) = ed.path {
                instance_dirs.push(PathBuf::from(path));
            }
        }
    }

    for base_dir in instance_dirs {
        if base_dir.ends_with("instances") {
            if let Ok(entries) = fs::read_dir(&base_dir) {
                for entry in entries.flatten() {
                    if !entry.path().is_dir() { continue; }
                    let instance_id = entry.file_name().to_string_lossy().to_string();
                    let wp_path = entry.path().join("workshop_packages.json");
                    let packages: Vec<InstalledWorkshopPackage> = fs::read_to_string(&wp_path)
                        .ok()
                        .and_then(|s| serde_json::from_str(&s).ok())
                        .unwrap_or_default();
                    for pkg in packages {
                        result.push(InstalledPackageEntry {
                            instance_id: instance_id.clone(),
                            package_id: pkg.id,
                            version: pkg.version,
                        });
                    }
                }
            }
        } else {
            let instance_id = base_dir.file_name().and_then(|n| n.to_str()).unwrap_or("custom").to_string();
            let config_val = config::load_config_raw(app.clone());
            let final_id = config_val.custom_editions.as_ref()
                .and_then(|eds| eds.iter().find(|e| e.path.as_deref() == base_dir.to_str()).map(|e| e.id.clone()))
                .unwrap_or(instance_id);

            let wp_path = base_dir.join("workshop_packages.json");
            let packages: Vec<InstalledWorkshopPackage> = fs::read_to_string(&wp_path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default();
            for pkg in packages {
                result.push(InstalledPackageEntry {
                    instance_id: final_id.clone(),
                    package_id: pkg.id,
                    version: pkg.version,
                });
            }
        }
    }
    result
}
