use crate::services::{ModEntry, ModsManager};
use crate::models::OperationResult;

/// List mods straight from the active MO2 profile's modlist.txt (order +
/// enabled state + user-mod flag).
#[tauri::command]
pub async fn list_mods() -> Result<Vec<ModEntry>, String> {
    ModsManager::list()
}

/// Enable/disable a mod by flipping its prefix in modlist.txt (writes to MO2).
#[tauri::command]
pub async fn toggle_mod(name: String, enabled: bool) -> OperationResult {
    match ModsManager::set_enabled(&name, enabled) {
        Ok(_) => OperationResult {
            success: true,
            message: format!("{} {}", name, if enabled { "enabled" } else { "disabled" }),
        },
        Err(e) => OperationResult { success: false, message: e },
    }
}

/// Add a user mod from a folder or .zip archive; installs into MO2's mods dir
/// and enables it at the top (highest priority).
#[tauri::command]
pub async fn add_user_mod(source: String) -> OperationResult {
    match ModsManager::add_user_mod(&source) {
        Ok(name) => OperationResult { success: true, message: format!("Added {}", name) },
        Err(e) => OperationResult { success: false, message: e },
    }
}

/// Move a mod up (higher priority) or down in the load order. Only usable in
/// the UI's Override mode for base mods; user mods can always be reordered.
#[tauri::command]
pub async fn move_mod(name: String, up: bool) -> OperationResult {
    match ModsManager::move_mod(&name, up) {
        Ok(_) => OperationResult { success: true, message: format!("Moved {}", name) },
        Err(e) => OperationResult { success: false, message: e },
    }
}
