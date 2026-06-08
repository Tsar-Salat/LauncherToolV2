use crate::services::LoadOrderValidator;
use crate::models::LoadOrder;

#[tauri::command]
pub async fn validate_load_order(load_order: LoadOrder) -> (bool, Vec<String>) {
    LoadOrderValidator::validate_load_order(&load_order)
}

#[tauri::command]
pub async fn check_conflicts(load_order: LoadOrder) -> Vec<String> {
    LoadOrderValidator::check_conflicts(&load_order)
}

#[tauri::command]
pub async fn validate_file_integrity(
    load_order: LoadOrder,
    mod_folder: String,
) -> (bool, Vec<String>) {
    LoadOrderValidator::validate_file_integrity(&load_order, &mod_folder)
}
