use crate::models::OperationResult;
use crate::services::enb_manager::{EnbConfig, EnbIniChange, EnbManager, EnbStatus};

#[tauri::command]
pub async fn get_enb_status() -> Result<EnbStatus, String> {
    EnbManager::get_status()
}

#[tauri::command]
pub async fn get_enb_config(target: String) -> EnbConfig {
    EnbManager::config(&target)
}

#[tauri::command]
pub async fn save_enb_config(changes: Vec<EnbIniChange>) -> OperationResult {
    EnbManager::save_config(&changes)
}

#[tauri::command]
pub async fn apply_default_enb() -> OperationResult {
    EnbManager::apply_default()
}

#[tauri::command]
pub async fn install_custom_enb(source_dir: String, name: String, showcase_path: Option<String>) -> OperationResult {
    EnbManager::install_custom(&source_dir, &name, showcase_path.as_deref())
}

#[tauri::command]
pub async fn remove_custom_enb() -> OperationResult {
    EnbManager::remove_custom()
}

#[tauri::command]
pub async fn disable_enb() -> OperationResult {
    EnbManager::disable_enb()
}

#[tauri::command]
pub async fn get_enb_showcase(which: String) -> String {
    EnbManager::showcase(&which)
}
