use crate::services::{PresetManager, PathDiscoveryService};
use crate::models::{Preset, OperationResult};

#[tauri::command]
pub async fn list_presets(preset_type: String) -> OperationResult {
    match PathDiscoveryService::discover() {
        Ok(paths) => {
            match PresetManager::list_presets(&paths.mods_folder, &preset_type) {
                Ok(presets) => OperationResult {
                    success: true,
                    message: serde_json::to_string(&presets).unwrap_or_default(),
                },
                Err(e) => OperationResult {
                    success: false,
                    message: format!("Failed to list presets: {}", e),
                },
            }
        }
        Err(e) => OperationResult {
            success: false,
            message: format!("Cannot determine game paths: {}", e),
        },
    }
}

#[tauri::command]
pub async fn install_preset(name: String, preset_type: String) -> OperationResult {
    match PathDiscoveryService::discover() {
        Ok(paths) => {
            match PresetManager::install_preset(&name, &preset_type, &paths.mods_folder) {
                Ok(_) => OperationResult {
                    success: true,
                    message: format!("Installed {} preset: {}", preset_type, name),
                },
                Err(e) => OperationResult {
                    success: false,
                    message: e,
                },
            }
        }
        Err(e) => OperationResult {
            success: false,
            message: format!("Cannot determine game paths: {}", e),
        },
    }
}

#[tauri::command]
pub async fn remove_preset(name: String, preset_type: String) -> OperationResult {
    match PresetManager::remove_preset(&name, &preset_type) {
        Ok(_) => OperationResult {
            success: true,
            message: format!("Removed {} preset: {}", preset_type, name),
        },
        Err(e) => OperationResult {
            success: false,
            message: e,
        },
    }
}

#[tauri::command]
pub async fn get_active_preset(preset_type: String) -> OperationResult {
    match PresetManager::get_active_preset(&preset_type) {
        Ok(preset_name) => OperationResult {
            success: true,
            message: preset_name.unwrap_or_default(),
        },
        Err(e) => OperationResult {
            success: false,
            message: e,
        },
    }
}

#[tauri::command]
pub async fn get_preset_preview(name: String, preset_type: String) -> OperationResult {
    match PathDiscoveryService::discover() {
        Ok(paths) => {
            match PresetManager::get_preset_preview_base64(&name, &preset_type, &paths.mods_folder) {
                Ok(Some(base64_data)) => OperationResult {
                    success: true,
                    message: base64_data,
                },
                Ok(None) => OperationResult {
                    success: true,
                    message: String::new(),
                },
                Err(e) => OperationResult {
                    success: false,
                    message: e,
                },
            }
        }
        Err(e) => OperationResult {
            success: false,
            message: format!("Cannot determine game paths: {}", e),
        },
    }
}
