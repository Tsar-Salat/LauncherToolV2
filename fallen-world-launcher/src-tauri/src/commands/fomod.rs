use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use crate::services::{FomodService, FomodInstaller, Mo2Integration, PathDiscoveryService};
use crate::services::fomod_service::InstallStep;
use crate::models::OperationResult;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectionItem {
    pub step_index: usize,
    pub group_index: usize,
    pub plugin_indices: Vec<usize>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FomodConfig {
    pub steps: Vec<InstallStep>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InstallFomodRequest {
    pub source_folder: String,
    pub output_folder: String,
    pub selections: Vec<SelectionItem>,
    pub confirmed_resolution: Option<(u32, u32)>,
    pub confirmed_gpu: Option<String>,
    pub upscaler_mode: Option<String>,
}

/// Load and parse FOMOD configuration
#[tauri::command]
pub fn load_fomod_config(config_path: String) -> FomodConfig {
    let path = PathBuf::from(&config_path);
    match FomodService::parse_config(&path) {
        Ok(steps) => {
            let wizard_steps: Vec<_> = FomodService::get_wizard_steps(&steps)
                .into_iter()
                .map(|(_, step)| step.clone())
                .collect();
            FomodConfig { steps: wizard_steps }
        },
        Err(_) => FomodConfig { steps: Vec::new() },
    }
}

/// Install selected FOMOD options
#[tauri::command]
pub fn install_fomod_options(req: InstallFomodRequest) -> OperationResult {
    use crate::services::fomod_service::Selection;

    let source_root = PathBuf::from(&req.source_folder);
    let output_root = PathBuf::from(&req.output_folder);

    // Parse and filter to exactly the same wizard steps the frontend shows.
    // Using get_wizard_steps keeps step indices in sync — the frontend sends
    // 0-based indices into this filtered array.
    let config_path = source_root.join("fomod/ModuleConfig.xml");
    let all_steps = match FomodService::parse_config(&config_path) {
        Ok(s) => s,
        Err(e) => return OperationResult {
            success: false,
            message: format!("Failed to parse FOMOD config: {}", e),
        },
    };
    let steps: Vec<InstallStep> = FomodService::get_wizard_steps(&all_steps)
        .into_iter()
        .map(|(_, step)| step.clone())
        .collect();

    // Convert SelectionItem to Selection
    let selections: Vec<Selection> = req.selections.into_iter().map(|s| Selection {
        step_index: s.step_index,
        group_index: s.group_index,
        plugin_indices: s.plugin_indices,
    }).collect();

    // Validate selections
    if let Err(e) = FomodService::validate_selections(&steps, &selections) {
        return OperationResult {
            success: false,
            message: format!("Invalid selections: {}", e),
        };
    }

    // Install selected files
    if let Err(e) = FomodInstaller::install_selected(
        &source_root,
        &output_root,
        &steps,
        &selections,
        req.confirmed_resolution,
        req.upscaler_mode.as_deref(),
    ) {
        return OperationResult {
            success: false,
            message: format!("Installation failed: {}", e),
        };
    }

    // Enable plugins in MO2 — discover the modlist root via PathDiscoveryService
    // so we honour the user-configured Fallen World path and per-modlist MO2
    // instances instead of guessing at "$HOME/Fallout 4 Modlist".
    let _plugins_enabled = PathDiscoveryService::discover()
        .ok()
        .and_then(|p| p.mo2_root)
        .and_then(|root| Mo2Integration::enable_plugins_in_mo2(&output_root, &PathBuf::from(root)).ok());

    if let Some(mod_name) = output_root.file_name().and_then(|n| n.to_str()) {
        let _ = crate::services::ModsManager::set_enabled(mod_name, true);
    }

    OperationResult {
        success: true,
        message: "FOMOD options installed successfully".to_string(),
    }
}

/// Get available FOMOD resources
#[tauri::command]
pub fn get_fomod_resources(mods_folder: String) -> FomodConfig {
    let fomod_resources = PathBuf::from(&mods_folder)
        .join("Fallen World FOMOD Resources")
        .join("fomod/ModuleConfig.xml");

    load_fomod_config(fomod_resources.to_string_lossy().to_string())
}

/// Read a FOMOD plugin preview image and return it as a base64 data URL.
/// `image_path` is the ModuleConfig-relative path (e.g. "fomod/images/X.png").
#[tauri::command]
pub fn get_fomod_image(mods_folder: String, image_path: String) -> Result<String, String> {
    // Normalise separators and prevent path escape.
    let rel = image_path.replace('\\', "/");
    if rel.contains("..") {
        return Err("Invalid image path".to_string());
    }
    let mut full = PathBuf::from(&mods_folder).join("Fallen World FOMOD Resources");
    for part in rel.split('/').filter(|p| !p.is_empty()) {
        full.push(part);
    }
    let bytes = std::fs::read(&full).map_err(|e| format!("Cannot read image: {}", e))?;
    let mime = match full.extension().and_then(|e| e.to_str()).map(|e| e.to_lowercase()).as_deref() {
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("webp") => "image/webp",
        Some("bmp") => "image/bmp",
        Some("gif") => "image/gif",
        _ => "image/png",
    };
    Ok(format!("data:{};base64,{}", mime, base64_encode(&bytes)))
}

/// Minimal base64 encoder (avoids pulling in an extra crate).
fn base64_encode(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity((data.len() + 2) / 3 * 4);
    for chunk in data.chunks(3) {
        let b = [chunk[0], *chunk.get(1).unwrap_or(&0), *chunk.get(2).unwrap_or(&0)];
        let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
        out.push(T[((n >> 18) & 63) as usize] as char);
        out.push(T[((n >> 12) & 63) as usize] as char);
        out.push(if chunk.len() > 1 { T[((n >> 6) & 63) as usize] as char } else { '=' });
        out.push(if chunk.len() > 2 { T[(n & 63) as usize] as char } else { '=' });
    }
    out
}

/// Check if FOMOD resources are available
#[tauri::command]
pub fn check_fomod_available(mods_folder: String) -> bool {
    let fomod_path = PathBuf::from(&mods_folder)
        .join("Fallen World FOMOD Resources")
        .join("fomod/ModuleConfig.xml");

    fomod_path.exists()
}
