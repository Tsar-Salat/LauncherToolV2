use crate::services::UpdateManager;
use crate::models::{ModUpdate, OperationResult};

#[tauri::command]
pub async fn check_updates() -> OperationResult {
    match UpdateManager::check_updates().await {
        Ok(updates) => OperationResult {
            success: true,
            message: serde_json::to_string(&updates).unwrap_or_default(),
        },
        Err(e) => OperationResult {
            success: false,
            message: e,
        },
    }
}

#[tauri::command]
pub async fn update_mod(mod_id: String) -> OperationResult {
    match UpdateManager::update_mod(&mod_id).await {
        Ok(()) => OperationResult {
            success: true,
            message: format!("Updated mod: {}", mod_id),
        },
        Err(e) => OperationResult {
            success: false,
            message: e,
        },
    }
}

#[tauri::command]
pub async fn get_changelog(mod_id: String) -> OperationResult {
    match UpdateManager::get_changelog(&mod_id).await {
        Ok(Some(changelog)) => OperationResult {
            success: true,
            message: changelog,
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

#[tauri::command]
pub async fn check_launcher_update() -> OperationResult {
    match UpdateManager::check_launcher_update().await {
        Ok(Some(version)) => OperationResult {
            success: true,
            message: version,
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
