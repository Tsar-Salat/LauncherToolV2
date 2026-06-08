use crate::services::{GameLauncher, PathDiscoveryService};
use crate::models::OperationResult;

#[tauri::command]
pub async fn launch_game() -> OperationResult {
    match PathDiscoveryService::discover() {
        Ok(paths) => GameLauncher::launch_game(&paths.game_root).await,
        Err(e) => OperationResult {
            success: false,
            message: format!("Cannot determine game path: {}", e),
        },
    }
}

#[tauri::command]
pub async fn get_game_info() -> Option<String> {
    PathDiscoveryService::discover()
        .ok()
        .map(|p| format!("Fallout 4 at {}", p.game_root))
}

#[tauri::command]
pub fn launch_mo2() -> OperationResult {
    GameLauncher::launch_mo2()
}

/// Open a URL or file path with the OS default handler (Nexus, Discord, etc.).
#[tauri::command]
pub fn open_external(target: String) -> Result<(), String> {
    crate::services::Platform::open_external(&target)
}

/// Quit the launcher. Used by the "close launcher when the game launches" option.
/// The game / MO2 run as independent processes, so exiting here doesn't affect them.
#[tauri::command]
pub fn quit_app() {
    std::process::exit(0);
}
