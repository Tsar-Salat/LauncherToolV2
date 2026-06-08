use crate::services::{Clf3, Clf3Status, Fluorine, LinuxBootstrap, ModlistMetadata};
use serde_json::Value;

// ── CLF3 (Wabbajack installer bridge) ─────────────────────────────────────

#[tauri::command]
pub fn clf3_status() -> Clf3Status {
    Clf3::status()
}

#[tauri::command]
pub fn clf3_has_api_key() -> bool {
    Clf3::has_saved_api_key()
}

#[tauri::command]
pub fn clf3_set_api_key(key: String) -> Result<(), String> {
    Clf3::set_nexus_api_key(&key)
}

#[tauri::command]
pub fn clf3_list_gpus() -> Result<Vec<Value>, String> {
    Clf3::list_gpus()
}

#[tauri::command]
pub fn clf3_check_updates(name: Option<String>) -> Result<Value, String> {
    Clf3::check_modlist_updates(name.as_deref())
}

/// Blocking; can take hours. The frontend should invoke this off the UI path
/// and show a spinner / status.
#[tauri::command]
pub async fn clf3_install_modlist(
    wabbajack_url: String,
    downloads: String,
    output: String,
    nexus_key: String,
    auto_fluorine: bool,
) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || {
        Clf3::install_modlist(&wabbajack_url, &downloads, &output, &nexus_key, auto_fluorine)
    })
    .await
    .map_err(|e| format!("install task panicked: {}", e))?
}

// ── Linux first-run bootstrap ──────────────────────────────────────────────

#[tauri::command]
pub fn fetch_modlist_metadata(url: Option<String>) -> Result<ModlistMetadata, String> {
    LinuxBootstrap::fetch_modlist_metadata(url.as_deref())
}

#[tauri::command]
pub async fn bootstrap_install(
    nexus_key: String,
    downloads: String,
    output: String,
    pinned_wabbajack_url: Option<String>,
    modlist_json: Option<String>,
) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || {
        LinuxBootstrap::install(
            &nexus_key,
            &downloads,
            &output,
            pinned_wabbajack_url.as_deref(),
            modlist_json.as_deref(),
        )
    })
    .await
    .map_err(|e| format!("bootstrap task panicked: {}", e))?
}

// ── Fluorine Manager ───────────────────────────────────────────────────────

#[tauri::command]
pub fn fluorine_open(install_dir: String) -> Result<u32, String> {
    Fluorine::open_instance(&install_dir, None)
}
