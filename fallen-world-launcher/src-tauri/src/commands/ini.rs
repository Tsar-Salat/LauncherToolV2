use crate::services::{IniManager, IniFileInfo, IniChange, PathDiscoveryService};
use crate::models::{IniConfig, OperationResult};
use regex::Regex;
use std::path::Path;

#[tauri::command]
pub async fn get_ini_config() -> Result<IniConfig, String> {
    let paths = PathDiscoveryService::discover()?;
    IniManager::read_ini(&paths.ini_folder)
}

/// List the INI files present in the active profile folder, with sizes.
#[tauri::command]
pub async fn list_ini_files() -> Result<Vec<IniFileInfo>, String> {
    let paths = PathDiscoveryService::discover()?;
    Ok(IniManager::list_files(&paths.ini_folder))
}

/// Read a single INI file (for the per-file structured editor).
#[tauri::command]
pub async fn read_ini_file(file: String) -> Result<IniConfig, String> {
    let paths = PathDiscoveryService::discover()?;
    IniManager::read_file(&paths.ini_folder, &file)
}

/// Apply a chosen resolution to the profile INIs (writes iSize W/H to Fallout4Prefs.ini
/// and Resolution=WxH to HighFPSPhysicsFix.ini in the Optional Mods folder).
/// `upscaler` — "DLSS" or "DLAA" — also disables TAA in the physics INI.
#[tauri::command]
pub async fn apply_resolution(width: u32, height: u32, upscaler: Option<String>) -> OperationResult {
    match PathDiscoveryService::discover() {
        Ok(paths) => {
            let changes = vec![
                IniChange { section: "Display".into(), key: "iSize W".into(), value: width.to_string() },
                IniChange { section: "Display".into(), key: "iSize H".into(), value: height.to_string() },
            ];
            if let Err(e) = IniManager::save_changes(&paths.ini_folder, "Fallout4Prefs.ini", &changes) {
                return OperationResult { success: false, message: e };
            }

            // Also write Resolution=WxH to HighFPSPhysicsFix.ini in Optional Mods.
            if let Some(mo2_root) = &paths.mo2_root {
                let physics_ini = Path::new(mo2_root)
                    .join("mods")
                    .join("Fallen World Optional Mods")
                    .join("F4SE")
                    .join("Plugins")
                    .join("HighFPSPhysicsFix.ini");
                if physics_ini.exists() {
                    if let Err(e) = update_physics_ini(&physics_ini, width, height, upscaler.as_deref()) {
                        return OperationResult {
                            success: true,
                            message: format!("Resolution set to {}x{} (HighFPS INI warning: {})", width, height, e),
                        };
                    }
                }
            }

            OperationResult { success: true, message: format!("Resolution set to {}x{}", width, height) }
        }
        Err(e) => OperationResult { success: false, message: e },
    }
}

fn update_physics_ini(ini_path: &Path, width: u32, height: u32, upscaler: Option<&str>) -> Result<(), String> {
    let content = std::fs::read_to_string(ini_path)
        .map_err(|e| format!("Read failed: {}", e))?;
    let mut updated = content;

    let res_pat = Regex::new(r"(?mi)^Resolution\s*=.*").unwrap();
    if res_pat.is_match(&updated) {
        updated = res_pat
            .replace(&updated, format!("Resolution={}x{}", width, height))
            .to_string();
    }

    if upscaler == Some("DLSS") || upscaler == Some("DLAA") {
        let taa_pat = Regex::new(r"(?mi)^bUseTAA\s*=.*").unwrap();
        if taa_pat.is_match(&updated) {
            updated = taa_pat.replace(&updated, "bUseTAA=0").to_string();
        }
    }

    std::fs::write(ini_path, updated)
        .map_err(|e| format!("Write failed: {}", e))
}

/// Apply a batch of edits to one INI file (in-place, preserves formatting).
#[tauri::command]
pub async fn save_ini_changes(file: String, changes: Vec<IniChange>) -> OperationResult {
    match PathDiscoveryService::discover() {
        Ok(paths) => match IniManager::save_changes(&paths.ini_folder, &file, &changes) {
            Ok(_) => OperationResult { success: true, message: format!("Saved {} ({} change(s))", file, changes.len()) },
            Err(e) => OperationResult { success: false, message: e },
        },
        Err(e) => OperationResult { success: false, message: e },
    }
}

#[tauri::command]
pub async fn update_ini_value(section: String, key: String, value: String) -> OperationResult {
    match PathDiscoveryService::discover() {
        Ok(paths) => {
            match IniManager::read_ini(&paths.ini_folder) {
                Ok(mut config) => {
                    IniManager::set_value(&mut config, &section, &key, &value);
                    match IniManager::write_ini(&paths.ini_folder, &config) {
                        Ok(_) => OperationResult {
                            success: true,
                            message: format!("Set {}.{}={}", section, key, value),
                        },
                        Err(e) => OperationResult {
                            success: false,
                            message: e,
                        },
                    }
                }
                Err(e) => OperationResult {
                    success: false,
                    message: format!("Cannot read INI: {}", e),
                },
            }
        }
        Err(e) => OperationResult {
            success: false,
            message: e,
        },
    }
}

#[tauri::command]
pub async fn backup_ini() -> OperationResult {
    match PathDiscoveryService::discover() {
        Ok(paths) => {
            match IniManager::backup_ini(&paths.ini_folder) {
                Ok(path) => OperationResult {
                    success: true,
                    message: format!("Backed up to {}", path),
                },
                Err(e) => OperationResult {
                    success: false,
                    message: e,
                },
            }
        }
        Err(e) => OperationResult {
            success: false,
            message: e,
        },
    }
}

#[tauri::command]
pub async fn restore_ini() -> OperationResult {
    match PathDiscoveryService::discover() {
        Ok(paths) => {
            match IniManager::restore_ini(&paths.ini_folder) {
                Ok(_) => OperationResult {
                    success: true,
                    message: "Restored from backup".to_string(),
                },
                Err(e) => OperationResult {
                    success: false,
                    message: e,
                },
            }
        }
        Err(e) => OperationResult {
            success: false,
            message: e,
        },
    }
}

#[tauri::command]
pub async fn apply_preset(preset: String) -> OperationResult {
    match PathDiscoveryService::discover() {
        Ok(paths) => {
            match IniManager::read_ini(&paths.ini_folder) {
                Ok(mut config) => {
                    match IniManager::apply_preset(&mut config, &preset) {
                        Ok(_) => {
                            match IniManager::write_ini(&paths.ini_folder, &config) {
                                Ok(_) => OperationResult {
                                    success: true,
                                    message: format!("Applied {} preset", preset),
                                },
                                Err(e) => OperationResult {
                                    success: false,
                                    message: e,
                                },
                            }
                        }
                        Err(e) => OperationResult {
                            success: false,
                            message: e,
                        },
                    }
                }
                Err(e) => OperationResult {
                    success: false,
                    message: format!("Cannot read INI: {}", e),
                },
            }
        }
        Err(e) => OperationResult {
            success: false,
            message: e,
        },
    }
}
