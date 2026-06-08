use crate::services::{ProfileManager, PathDiscoveryService};
use crate::models::{GameProfile, OperationResult};
use std::path::PathBuf;

#[tauri::command]
pub async fn list_profiles() -> OperationResult {
    match ProfileManager::list_profiles() {
        Ok(profiles) => OperationResult {
            success: true,
            message: serde_json::to_string(&profiles).unwrap_or_default(),
        },
        Err(e) => OperationResult {
            success: false,
            message: e,
        },
    }
}

#[tauri::command]
pub async fn save_profile(profile: GameProfile) -> OperationResult {
    let profile_name = profile.name.clone();
    match ProfileManager::save_profile(profile) {
        Ok(()) => OperationResult {
            success: true,
            message: format!("Saved profile: {}", profile_name),
        },
        Err(e) => OperationResult {
            success: false,
            message: e,
        },
    }
}

#[tauri::command]
pub async fn load_profile(name: String) -> OperationResult {
    match ProfileManager::load_profile(&name) {
        Ok(profile) => OperationResult {
            success: true,
            message: serde_json::to_string(&profile).unwrap_or_default(),
        },
        Err(e) => OperationResult {
            success: false,
            message: e,
        },
    }
}

#[tauri::command]
pub async fn delete_profile(name: String) -> OperationResult {
    match ProfileManager::delete_profile(&name) {
        Ok(()) => OperationResult {
            success: true,
            message: format!("Deleted profile: {}", name),
        },
        Err(e) => OperationResult {
            success: false,
            message: e,
        },
    }
}

#[tauri::command]
pub async fn profile_exists(name: String) -> OperationResult {
    match ProfileManager::profile_exists(&name) {
        Ok(exists) => OperationResult {
            success: true,
            message: exists.to_string(),
        },
        Err(e) => OperationResult {
            success: false,
            message: e,
        },
    }
}

#[tauri::command]
pub async fn activate_profile(name: String) -> OperationResult {
    match ProfileManager::activate_profile(&name) {
        Ok(()) => OperationResult {
            success: true,
            message: format!("Activated profile: {}", name),
        },
        Err(e) => OperationResult { success: false, message: e },
    }
}

#[tauri::command]
pub async fn open_profiles_folder() -> OperationResult {
    match ProfileManager::open_profiles_folder() {
        Ok(()) => OperationResult { success: true, message: "Opened profiles folder".to_string() },
        Err(e) => OperationResult { success: false, message: e },
    }
}

#[tauri::command]
pub async fn rename_profile(old_name: String, new_name: String) -> OperationResult {
    match ProfileManager::rename_profile(&old_name, &new_name) {
        Ok(()) => OperationResult {
            success: true,
            message: format!("Renamed profile from {} to {}", old_name, new_name),
        },
        Err(e) => OperationResult {
            success: false,
            message: e,
        },
    }
}

#[tauri::command]
pub async fn get_profile_metadata(name: String) -> OperationResult {
    match ProfileManager::get_profile_metadata(&name) {
        Ok((created, modified, profile_name)) => {
            let metadata = serde_json::json!({
                "created_date": created,
                "last_modified": modified,
                "name": profile_name,
            });
            OperationResult {
                success: true,
                message: serde_json::to_string(&metadata).unwrap_or_default(),
            }
        },
        Err(e) => OperationResult {
            success: false,
            message: e,
        },
    }
}

/// Move saves from the MO2 profiles/Saves directory to a user-chosen folder.
/// Uses a PowerShell FolderBrowserDialog so the user picks the destination in Explorer.
#[tauri::command]
pub fn backup_saves() -> OperationResult {
    let paths = match PathDiscoveryService::discover() {
        Ok(p) => p,
        Err(e) => return OperationResult { success: false, message: e },
    };
    let mo2_root = match paths.mo2_root {
        Some(r) => r,
        None => return OperationResult { success: false, message: "MO2 root not found".to_string() },
    };

    // Look for saves in the active profile's saves folder, then the shared one
    let active_name = paths.mo2_profile.unwrap_or_default();
    let candidates = [
        PathBuf::from(&mo2_root).join("profiles").join(&active_name).join("saves"),
        PathBuf::from(&mo2_root).join("profiles").join("Saves"),
    ];
    let saves_dir = candidates.iter().find(|p| p.exists()).cloned();

    let saves_dir = match saves_dir {
        Some(p) => p,
        None => return OperationResult {
            success: false,
            message: format!(
                "No saves folder found. Checked:\n  {}\n  {}",
                candidates[0].display(), candidates[1].display()
            ),
        },
    };

    // Use PowerShell FolderBrowserDialog to pick destination
    let ps_script = r#"
Add-Type -AssemblyName System.Windows.Forms
$dialog = New-Object System.Windows.Forms.FolderBrowserDialog
$dialog.Description = 'Choose where to move your Fallout 4 saves'
$dialog.ShowNewFolderButton = $true
$result = $dialog.ShowDialog()
if ($result -eq [System.Windows.Forms.DialogResult]::OK) {
    Write-Output $dialog.SelectedPath
}
"#;
    let output = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", ps_script])
        .output();

    let dest_str = match output {
        Ok(out) if out.status.success() => {
            String::from_utf8_lossy(&out.stdout).trim().to_string()
        }
        _ => return OperationResult { success: false, message: "Could not open folder picker".to_string() },
    };

    if dest_str.is_empty() {
        return OperationResult { success: false, message: "No destination selected".to_string() };
    }

    let dest = PathBuf::from(&dest_str).join("Fallout4_Saves_FallenWorld");

    match ProfileManager::copy_dir(&saves_dir, &dest) {
        Ok(()) => {
            match std::fs::remove_dir_all(&saves_dir) {
                Ok(()) => OperationResult {
                    success: true,
                    message: format!("Saves moved to:\n{}", dest.display()),
                },
                Err(e) => OperationResult {
                    success: false,
                    message: format!("Saves copied but original could not be removed: {}", e),
                },
            }
        }
        Err(e) => OperationResult { success: false, message: e },
    }
}
