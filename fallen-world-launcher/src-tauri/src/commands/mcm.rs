use crate::services::{McmManager, PathDiscoveryService};
use crate::models::OperationResult;

#[tauri::command]
pub async fn list_mcm_presets() -> OperationResult {
    match PathDiscoveryService::discover() {
        Ok(paths) => match McmManager::list_presets(&paths.game_root) {
            Ok(presets) => OperationResult {
                success: true,
                message: serde_json::to_string(&presets).unwrap_or_default(),
            },
            Err(e) => OperationResult {
                success: false,
                message: e,
            },
        },
        Err(e) => OperationResult {
            success: false,
            message: format!("Cannot determine game paths: {}", e),
        },
    }
}

#[tauri::command]
pub async fn load_mcm_preset(name: String) -> OperationResult {
    match PathDiscoveryService::discover() {
        Ok(paths) => match McmManager::load_preset(&name, &paths.game_root) {
            Ok(preset) => OperationResult {
                success: true,
                message: serde_json::to_string(&preset).unwrap_or_default(),
            },
            Err(e) => OperationResult {
                success: false,
                message: e,
            },
        },
        Err(e) => OperationResult {
            success: false,
            message: format!("Cannot determine game paths: {}", e),
        },
    }
}

#[tauri::command]
pub async fn save_mcm_preset(
    name: String,
    settings: std::collections::HashMap<String, std::collections::HashMap<String, String>>,
) -> OperationResult {
    match PathDiscoveryService::discover() {
        Ok(paths) => match McmManager::save_preset(&name, &paths.game_root, settings) {
            Ok(()) => OperationResult {
                success: true,
                message: format!("Saved MCM preset: {}", name),
            },
            Err(e) => OperationResult {
                success: false,
                message: e,
            },
        },
        Err(e) => OperationResult {
            success: false,
            message: format!("Cannot determine game paths: {}", e),
        },
    }
}

#[tauri::command]
pub async fn apply_mcm_preset(name: String) -> OperationResult {
    match PathDiscoveryService::discover() {
        Ok(paths) => match McmManager::apply_preset(&name, &paths.game_root, &paths.ini_folder) {
            Ok(()) => OperationResult {
                success: true,
                message: format!("Applied MCM preset: {}", name),
            },
            Err(e) => OperationResult {
                success: false,
                message: e,
            },
        },
        Err(e) => OperationResult {
            success: false,
            message: format!("Cannot determine game paths: {}", e),
        },
    }
}

#[tauri::command]
pub async fn delete_mcm_preset(name: String) -> OperationResult {
    match PathDiscoveryService::discover() {
        Ok(paths) => match McmManager::delete_preset(&name, &paths.game_root) {
            Ok(()) => OperationResult {
                success: true,
                message: format!("Deleted MCM preset: {}", name),
            },
            Err(e) => OperationResult {
                success: false,
                message: e,
            },
        },
        Err(e) => OperationResult {
            success: false,
            message: format!("Cannot determine game paths: {}", e),
        },
    }
}

#[tauri::command]
pub async fn get_current_mcm_settings() -> OperationResult {
    match PathDiscoveryService::discover() {
        Ok(paths) => match McmManager::get_current_settings(&paths.ini_folder) {
            Ok(settings) => OperationResult {
                success: true,
                message: serde_json::to_string(&settings).unwrap_or_default(),
            },
            Err(e) => OperationResult {
                success: false,
                message: e,
            },
        },
        Err(e) => OperationResult {
            success: false,
            message: format!("Cannot determine game paths: {}", e),
        },
    }
}

#[tauri::command]
pub async fn mcm_preset_exists(name: String) -> OperationResult {
    match PathDiscoveryService::discover() {
        Ok(paths) => match McmManager::preset_exists(&name, &paths.game_root) {
            Ok(exists) => OperationResult {
                success: true,
                message: exists.to_string(),
            },
            Err(e) => OperationResult {
                success: false,
                message: e,
            },
        },
        Err(e) => OperationResult {
            success: false,
            message: format!("Cannot determine game paths: {}", e),
        },
    }
}
