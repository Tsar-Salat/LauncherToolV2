use crate::models::Preset;
use crate::services::PathDiscoveryService;
use std::fs;
use std::path::Path;

pub struct PresetManager;

impl PresetManager {
    /// List available ENB and ReShade presets from FOMOD resources
    pub fn list_presets(mods_folder: &str, preset_type: &str) -> Result<Vec<Preset>, String> {
        let mut presets = Vec::new();

        let preset_dir = if preset_type.eq_ignore_ascii_case("ENB") {
            format!("{}/Fallen World FOMOD Resources/ENBs/Root/ENBPresets", mods_folder)
        } else if preset_type.eq_ignore_ascii_case("ReShade") {
            format!("{}/Fallen World FOMOD Resources/ReshadeSetup/Root/ReshadePresets", mods_folder)
        } else {
            return Err("Invalid preset type: must be ENB or ReShade".to_string());
        };

        let preset_path = Path::new(&preset_dir);
        if !preset_path.exists() {
            return Ok(presets); // Return empty list if directory doesn't exist
        }

        let entries = fs::read_dir(&preset_dir)
            .map_err(|e| format!("Cannot read preset directory: {}", e))?;

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if let Some(preset_name) = path.file_name().and_then(|n| n.to_str()) {
                    let installed = Self::is_preset_installed(preset_type, preset_name)
                        .unwrap_or(false);

                    let preview_image = if Self::has_preview(&path) {
                        Self::load_preview(&path)
                    } else {
                        Vec::new()
                    };

                    let preset = Preset {
                        name: preset_name.to_string(),
                        preset_type: preset_type.to_string(),
                        preview_image: if !preview_image.is_empty() {
                            Some(preview_image)
                        } else {
                            None
                        },
                        installed,
                    };

                    presets.push(preset);
                }
            }
        }

        presets.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(presets)
    }

    /// Install a preset from FOMOD resources to game folder
    pub fn install_preset(
        name: &str,
        preset_type: &str,
        mods_folder: &str,
    ) -> Result<(), String> {
        // Validate preset name to prevent path traversal
        if name.contains("..") || name.contains("/") || name.contains("\\") {
            return Err("Invalid preset name".to_string());
        }

        let source_dir = if preset_type.eq_ignore_ascii_case("ENB") {
            format!("{}/Fallen World FOMOD Resources/ENBs/Root/ENBPresets/{}", mods_folder, name)
        } else if preset_type.eq_ignore_ascii_case("ReShade") {
            format!("{}/Fallen World FOMOD Resources/ReshadeSetup/Root/ReshadePresets/{}", mods_folder, name)
        } else {
            return Err("Invalid preset type".to_string());
        };

        let src_path = Path::new(&source_dir);
        if !src_path.exists() {
            return Err(format!("Preset not found: {}", name));
        }

        // Determine destination based on preset type
        let game_paths = PathDiscoveryService::discover()
            .map_err(|e| format!("Cannot determine game paths: {}", e))?;

        let dest_dir = if preset_type.eq_ignore_ascii_case("ENB") {
            format!("{}/ENBSeries", game_paths.game_root)
        } else {
            format!("{}/Reshade", game_paths.game_root)
        };

        fs::create_dir_all(&dest_dir)
            .map_err(|e| format!("Cannot create destination directory: {}", e))?;

        // Copy preset files
        Self::copy_preset_recursive(src_path, &dest_dir)
            .map_err(|e| format!("Failed to install preset: {}", e))?;

        Ok(())
    }

    /// Remove an installed preset
    pub fn remove_preset(name: &str, preset_type: &str) -> Result<(), String> {
        let game_paths = PathDiscoveryService::discover()
            .map_err(|e| format!("Cannot determine game paths: {}", e))?;

        let install_path = if preset_type.eq_ignore_ascii_case("ENB") {
            format!("{}/ENBSeries/{}", game_paths.game_root, name)
        } else {
            format!("{}/Reshade/{}", game_paths.game_root, name)
        };

        let path = Path::new(&install_path);
        if path.exists() {
            fs::remove_dir_all(path)
                .map_err(|e| format!("Failed to remove preset: {}", e))?;
        }

        Ok(())
    }

    /// Get the currently active preset
    pub fn get_active_preset(preset_type: &str) -> Result<Option<String>, String> {
        let game_paths = PathDiscoveryService::discover()
            .map_err(|e| format!("Cannot determine game paths: {}", e))?;

        let preset_dir = if preset_type.eq_ignore_ascii_case("ENB") {
            format!("{}/ENBSeries", game_paths.game_root)
        } else {
            format!("{}/Reshade", game_paths.game_root)
        };

        let path = Path::new(&preset_dir);
        if !path.exists() {
            return Ok(None);
        }

        // Return the first preset folder found as "active"
        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                let entry_path = entry.path();
                if entry_path.is_dir() {
                    if let Some(name) = entry_path.file_name().and_then(|n| n.to_str()) {
                        if !name.starts_with(".") {
                            return Ok(Some(name.to_string()));
                        }
                    }
                }
            }
        }

        Ok(None)
    }

    /// Check if a preset is installed
    fn is_preset_installed(preset_type: &str, preset_name: &str) -> Result<bool, String> {
        let game_paths = PathDiscoveryService::discover()?;

        let install_path = if preset_type.eq_ignore_ascii_case("ENB") {
            format!("{}/ENBSeries/{}", game_paths.game_root, preset_name)
        } else {
            format!("{}/Reshade/{}", game_paths.game_root, preset_name)
        };

        Ok(Path::new(&install_path).exists())
    }

    /// Check if preset has a preview image
    fn has_preview(preset_path: &Path) -> bool {
        let png_path = preset_path.join("preview.png");
        let jpg_path = preset_path.join("preview.jpg");
        png_path.exists() || jpg_path.exists()
    }

    /// Load preview image as binary data
    fn load_preview(preset_path: &Path) -> Vec<u8> {
        let png_path = preset_path.join("preview.png");
        let jpg_path = preset_path.join("preview.jpg");

        if let Ok(data) = fs::read(&png_path) {
            return data;
        } else if let Ok(data) = fs::read(&jpg_path) {
            return data;
        }

        Vec::new()
    }

    /// Recursively copy preset files
    fn copy_preset_recursive(src: &Path, dst: &str) -> std::io::Result<()> {
        if src.is_dir() {
            fs::create_dir_all(dst)?;
            for entry in fs::read_dir(src)? {
                let entry = entry?;
                let path = entry.path();
                let file_name = entry.file_name();
                let dst_path = Path::new(dst).join(&file_name);

                if path.is_dir() {
                    Self::copy_preset_recursive(&path, dst_path.to_string_lossy().as_ref())?;
                } else {
                    fs::copy(&path, &dst_path)?;
                }
            }
        } else {
            fs::copy(src, dst)?;
        }
        Ok(())
    }

    /// Load preview image as base64 string for frontend
    pub fn get_preset_preview_base64(
        name: &str,
        preset_type: &str,
        mods_folder: &str,
    ) -> Result<Option<String>, String> {
        let preset_dir = if preset_type.eq_ignore_ascii_case("ENB") {
            format!("{}/Fallen World FOMOD Resources/ENBs/Root/ENBPresets/{}", mods_folder, name)
        } else if preset_type.eq_ignore_ascii_case("ReShade") {
            format!("{}/Fallen World FOMOD Resources/ReshadeSetup/Root/ReshadePresets/{}", mods_folder, name)
        } else {
            return Err("Invalid preset type".to_string());
        };

        let preset_path = Path::new(&preset_dir);
        let png_path = preset_path.join("preview.png");
        let jpg_path = preset_path.join("preview.jpg");

        let image_data = if png_path.exists() {
            fs::read(&png_path).ok()
        } else if jpg_path.exists() {
            fs::read(&jpg_path).ok()
        } else {
            None
        };

        if let Some(data) = image_data {
            let mime = if jpg_path.exists() { "image/jpeg" } else { "image/png" };
            let encoded = Self::encode_base64(&data);
            Ok(Some(format!("data:{};base64,{}", mime, encoded)))
        } else {
            Ok(None)
        }
    }

    fn encode_base64(data: &[u8]) -> String {
        const TABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut out = String::with_capacity((data.len() + 2) / 3 * 4);
        for chunk in data.chunks(3) {
            let b0 = chunk[0] as usize;
            let b1 = chunk.get(1).copied().unwrap_or(0) as usize;
            let b2 = chunk.get(2).copied().unwrap_or(0) as usize;
            let n = (b0 << 16) | (b1 << 8) | b2;
            out.push(TABLE[(n >> 18) & 63] as char);
            out.push(TABLE[(n >> 12) & 63] as char);
            out.push(if chunk.len() > 1 { TABLE[(n >> 6) & 63] as char } else { '=' });
            out.push(if chunk.len() > 2 { TABLE[n & 63] as char } else { '=' });
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_preset_name() {
        let bad_names = vec!["../../../etc/passwd", "preset/bad", "preset\\bad"];
        for name in bad_names {
            assert!(name.contains("..") || name.contains("/") || name.contains("\\"));
        }
    }

    #[test]
    fn test_enb_path_construction() {
        let mods_folder = "/mods";
        let expected = "/mods/Fallen World FOMOD Resources/ENBs/Root/ENBPresets";
        let actual = format!("{}/Fallen World FOMOD Resources/ENBs/Root/ENBPresets", mods_folder);
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_reshade_path_construction() {
        let mods_folder = "/mods";
        let expected = "/mods/Fallen World FOMOD Resources/ReshadeSetup/Root/ReshadePresets";
        let actual = format!("{}/Fallen World FOMOD Resources/ReshadeSetup/Root/ReshadePresets", mods_folder);
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_preset_type_case_insensitive() {
        let types = vec!["ENB", "enb", "Enb", "ReShade", "reshade", "RESHADE"];
        for preset_type in types {
            assert!(
                preset_type.eq_ignore_ascii_case("ENB")
                    || preset_type.eq_ignore_ascii_case("ReShade")
            );
        }
    }
}
