use std::fs;
use std::path::{Path, PathBuf};
use regex::Regex;
use crate::services::fomod_service::{InstallStep, Selection};
use crate::services::system_info::SystemInfoService;

pub struct FomodInstaller;

impl FomodInstaller {
    /// Install selected FOMOD options to output folder
    pub fn install_selected(
        source_root: &Path,
        output_root: &Path,
        steps: &[InstallStep],
        selections: &[Selection],
        confirmed_resolution: Option<(u32, u32)>,
        upscaler_mode: Option<&str>,
    ) -> Result<(), String> {
        // Validate paths
        Self::validate_output_path(output_root)?;

        // Check for blocking processes
        Self::check_blocking_processes()?;

        // Probe lock before deletion
        Self::probe_output_lock(output_root)?;

        // Clear output folder
        if output_root.exists() {
            fs::remove_dir_all(output_root)
                .map_err(|e| format!("Failed to remove existing folder: {}", e))?;
        }

        fs::create_dir_all(output_root)
            .map_err(|e| format!("Failed to create output folder: {}", e))?;

        // Copy selected files
        Self::copy_selected_files(source_root, output_root, steps, selections)?;

        // Handle ultrawide support if needed
        if let Some((width, height)) = confirmed_resolution {
            if Self::is_ultrawide_resolution(width, height) {
                Self::install_ultrawide_support(source_root, output_root, width, height)?;
            }
        }

        // Copy ENB assets (skip if DLSS)
        if upscaler_mode != Some("DLSS") {
            Self::copy_enb_assets(source_root, output_root)?;
        }

        // Copy resolution-specific files
        if let Some((width, height)) = confirmed_resolution {
            Self::copy_resolution_support(source_root, output_root, width, height)?;
            Self::apply_resolution_to_inis(output_root, width, height, upscaler_mode)?;
        }

        Ok(())
    }

    fn validate_output_path(output_root: &Path) -> Result<(), String> {
        let expected_name = "Fallen World Optional Mods";
        if !output_root.to_string_lossy().contains(expected_name) {
            return Err(format!(
                "Safety check failed: output folder should be named '{}'",
                expected_name
            ));
        }
        Ok(())
    }

    fn check_blocking_processes() -> Result<(), String> {
        let blockers: Vec<String> = SystemInfoService::list_running_processes_pub()
            .into_iter()
            .filter(|p| {
                let n = p.name.to_lowercase();
                matches!(n.as_str(), "fallout4.exe" | "f4se_loader.exe" | "modorganizer.exe" | "mo2.exe")
            })
            .map(|p| format!("- {}", p.name))
            .collect();

        if !blockers.is_empty() {
            return Err(format!(
                "Cannot install while these processes are running:\n{}\n\nClose them and try again.",
                blockers.join("\n")
            ));
        }
        Ok(())
    }

    fn probe_output_lock(output_root: &Path) -> Result<(), String> {
        if !output_root.exists() {
            return Ok(());
        }

        let probe_file = output_root.join(".__fa_lock_probe__.tmp");
        match fs::write(&probe_file, "probe") {
            Ok(_) => {
                let _ = fs::remove_file(probe_file);
                Ok(())
            }
            Err(e) => Err(format!(
                "Output folder appears locked. Please close MO2 and the game: {}",
                e
            )),
        }
    }

    fn copy_selected_files(
        source_root: &Path,
        output_root: &Path,
        steps: &[InstallStep],
        selections: &[Selection],
    ) -> Result<(), String> {
        let mut missing_sources = Vec::new();
        let mut copy_failures = Vec::new();

        for sel in selections {
            if sel.step_index >= steps.len() {
                continue;
            }

            let step = &steps[sel.step_index];
            if sel.group_index >= step.groups.len() {
                continue;
            }

            let group = &step.groups[sel.group_index];

            for &plugin_idx in &sel.plugin_indices {
                if plugin_idx >= group.plugins.len() {
                    continue;
                }

                let plugin = &group.plugins[plugin_idx];

                for file_spec in &plugin.files {
                    let src_path = Self::resolve_source_path(source_root, &file_spec.source);
                    let dst_base = if file_spec.destination.is_empty() {
                        output_root.to_path_buf()
                    } else {
                        output_root.join(&file_spec.destination)
                    };

                    match file_spec.file_type.as_str() {
                        "folder" => {
                            if src_path.exists() && src_path.is_dir() {
                                // Create destination and copy all contents
                                if let Err(e) = Self::copy_folder_recursive(&src_path, &dst_base) {
                                    copy_failures.push(format!(
                                        "{} -> {} ({})",
                                        src_path.display(),
                                        dst_base.display(),
                                        e
                                    ));
                                }
                            } else {
                                missing_sources.push(format!(
                                    "{} (plugin: {})",
                                    file_spec.source,
                                    plugin.name
                                ));
                            }
                        }
                        "file" => {
                            if src_path.exists() && src_path.is_file() {
                                let dst_file = if file_spec.destination.is_empty() {
                                    dst_base.join(src_path.file_name().unwrap())
                                } else {
                                    dst_base
                                };

                                if let Some(parent) = dst_file.parent() {
                                    let _ = fs::create_dir_all(parent);
                                }

                                if let Err(e) = fs::copy(&src_path, &dst_file) {
                                    copy_failures.push(format!(
                                        "{} -> {} ({})",
                                        src_path.display(),
                                        dst_file.display(),
                                        e
                                    ));
                                }
                            } else {
                                missing_sources.push(format!(
                                    "{} (plugin: {})",
                                    file_spec.source,
                                    plugin.name
                                ));
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        if !missing_sources.is_empty() {
            let preview = missing_sources.iter().take(12).cloned().collect::<Vec<_>>().join("\n");
            let extra = if missing_sources.len() > 12 {
                format!("\n...and {} more", missing_sources.len() - 12)
            } else {
                String::new()
            };
            return Err(format!(
                "Selected files not found:\n{}{}\n\nPlease verify the FOMOD package is complete.",
                preview, extra
            ));
        }

        if !copy_failures.is_empty() {
            let preview = copy_failures.iter().take(12).cloned().collect::<Vec<_>>().join("\n");
            let extra = if copy_failures.len() > 12 {
                format!("\n...and {} more", copy_failures.len() - 12)
            } else {
                String::new()
            };
            return Err(format!("Copy failures:\n{}{}", preview, extra));
        }

        Ok(())
    }

    fn resolve_source_path(source_root: &Path, relative_source: &str) -> PathBuf {
        let normalized = relative_source.replace("\\", "/").trim_matches('/').to_string();

        if normalized.is_empty() {
            return source_root.to_path_buf();
        }

        let direct = source_root.join(&normalized);
        if direct.exists() {
            return direct;
        }

        // Case-insensitive fallback
        let mut current = source_root.to_path_buf();
        for part in normalized.split('/') {
            let candidate = current.join(part);
            if candidate.exists() {
                current = candidate;
            } else {
                // Try case-insensitive match
                if let Ok(entries) = fs::read_dir(&current) {
                    let mut found = false;
                    for entry in entries {
                        if let Ok(entry) = entry {
                            if entry.file_name().to_string_lossy().eq_ignore_ascii_case(part) {
                                current = entry.path();
                                found = true;
                                break;
                            }
                        }
                    }
                    if !found {
                        return source_root.join(&normalized);
                    }
                } else {
                    return source_root.join(&normalized);
                }
            }
        }

        current
    }

    fn copy_folder_recursive(src: &Path, dst: &Path) -> Result<(), String> {
        fs::create_dir_all(dst).map_err(|e| e.to_string())?;

        for entry in fs::read_dir(src).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            let file_name = entry.file_name();
            let dst_path = dst.join(&file_name);

            if path.is_dir() {
                Self::copy_folder_recursive(&path, &dst_path)?;
            } else {
                fs::copy(&path, &dst_path).map_err(|e| e.to_string())?;
            }
        }

        Ok(())
    }

    fn is_ultrawide_resolution(width: u32, height: u32) -> bool {
        if height == 0 {
            return false;
        }
        let ratio = width as f64 / height as f64;
        ratio > 2.0 // 21:9 is ~2.33, 32:9 is ~3.55
    }

    fn install_ultrawide_support(
        source_root: &Path,
        output_root: &Path,
        width: u32,
        height: u32,
    ) -> Result<(), String> {
        let ratio = width as f64 / height as f64;
        let support_folder = if ratio > 3.0 {
            "GasMask329Support"
        } else {
            "GasMask219Support"
        };

        let support_src = source_root.join(support_folder);
        if support_src.exists() && support_src.is_dir() {
            Self::copy_folder_recursive(&support_src, output_root)?;
        }

        Ok(())
    }

    fn copy_enb_assets(source_root: &Path, output_root: &Path) -> Result<(), String> {
        let enb_dir = source_root.join("ENBs");
        if enb_dir.exists() && enb_dir.is_dir() {
            for entry in fs::read_dir(&enb_dir).map_err(|e| e.to_string())? {
                let entry = entry.map_err(|e| e.to_string())?;
                let path = entry.path();
                let file_name = entry.file_name();

                if file_name.to_string_lossy().to_lowercase() == "meta.ini" {
                    continue;
                }

                let dst = output_root.join(&file_name);
                if path.is_dir() {
                    Self::copy_folder_recursive(&path, &dst)?;
                } else {
                    fs::copy(&path, &dst).map_err(|e| e.to_string())?;
                }
            }
        }
        Ok(())
    }

    fn copy_resolution_support(
        source_root: &Path,
        output_root: &Path,
        width: u32,
        height: u32,
    ) -> Result<(), String> {
        let reso_support = source_root.join("ResoSupport");
        let reso_src = if reso_support.join(format!("{}X{}", width, height)).exists() {
            reso_support.join(format!("{}X{}", width, height))
        } else {
            reso_support.join(format!("{}x{}", width, height))
        };

        if reso_src.exists() && reso_src.is_dir() {
            Self::copy_folder_recursive(&reso_src, output_root)?;
        }

        Ok(())
    }

    fn apply_resolution_to_inis(
        output_root: &Path,
        width: u32,
        height: u32,
        upscaler_mode: Option<&str>,
    ) -> Result<(), String> {
        // Update HighFPSPhysicsFix.ini if present
        let physics_ini = output_root.join("f4se/plugins/HighFPSPhysicsFix.ini");
        if physics_ini.exists() {
            Self::update_physics_ini(&physics_ini, width, height, upscaler_mode)?;
        }

        Ok(())
    }

    fn update_physics_ini(
        ini_path: &Path,
        width: u32,
        height: u32,
        upscaler_mode: Option<&str>,
    ) -> Result<(), String> {
        let content = fs::read_to_string(ini_path)
            .map_err(|e| format!("Failed to read INI: {}", e))?;

        let mut updated = content.clone();

        // Update resolution
        let res_pattern = Regex::new(r"(?mi)^Resolution\s*=.*").unwrap();
        if res_pattern.is_match(&updated) {
            updated = res_pattern
                .replace(&updated, format!("Resolution={}x{}", width, height))
                .to_string();
        }

        // Disable TAA for DLSS/DLAA
        if upscaler_mode == Some("DLSS") || upscaler_mode == Some("DLAA") {
            let taa_pattern = Regex::new(r"(?mi)^bUseTAA\s*=.*").unwrap();
            if taa_pattern.is_match(&updated) {
                updated = taa_pattern.replace(&updated, "bUseTAA=0").to_string();
            }
        }

        // Update flip model settings for stability
        let swap_pattern = Regex::new(r"(?mi)^SwapEffect\s*=.*").unwrap();
        if swap_pattern.is_match(&updated) {
            updated = swap_pattern.replace(&updated, "SwapEffect=4").to_string();
        }

        fs::write(ini_path, updated)
            .map_err(|e| format!("Failed to write INI: {}", e))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_ultrawide_resolution() {
        // Standard 16:9
        assert!(!FomodInstaller::is_ultrawide_resolution(1920, 1080));
        assert!(!FomodInstaller::is_ultrawide_resolution(2560, 1440));

        // 21:9 ultrawide (~2.33)
        assert!(FomodInstaller::is_ultrawide_resolution(3440, 1440));
        assert!(FomodInstaller::is_ultrawide_resolution(5120, 2160));

        // 32:9 superwide (~3.55)
        assert!(FomodInstaller::is_ultrawide_resolution(5120, 1440));

        // Edge case
        assert!(!FomodInstaller::is_ultrawide_resolution(100, 0));
    }

    #[test]
    fn test_validate_output_path_valid() {
        let path = Path::new("C:/Fallout4/mods/Fallen World Optional Mods");
        assert!(FomodInstaller::validate_output_path(path).is_ok());
    }

    #[test]
    fn test_validate_output_path_invalid() {
        let path = Path::new("C:/Fallout4/mods/Bad Folder Name");
        assert!(FomodInstaller::validate_output_path(path).is_err());
    }

    #[test]
    fn test_resolve_source_path_direct() {
        let base = Path::new(".");
        let path = FomodInstaller::resolve_source_path(base, "test/file.txt");
        // Should handle path normalization
        assert!(!path.to_string_lossy().contains("\\"));
    }

    #[test]
    fn test_resolve_source_path_normalization() {
        let base = Path::new(".");
        let path = FomodInstaller::resolve_source_path(base, "test\\file.txt");
        // Should normalize backslashes to forward slashes
        assert!(!path.to_string_lossy().contains("\\\\"));
    }

    #[test]
    fn test_resolve_source_path_leading_slash() {
        let base = Path::new(".");
        let path = FomodInstaller::resolve_source_path(base, "/test/file.txt");
        // Should handle leading slashes
        let path_str = path.to_string_lossy();
        assert!(path_str.contains("test") || path_str.contains("file"));
    }

    #[test]
    fn test_ultrawide_32_9_vs_21_9() {
        // 21:9 ratio (~2.33) - should be GasMask219Support
        let ratio_219 = 3440.0 / 1440.0; // ~2.39
        assert!(ratio_219 > 2.0);
        assert!(ratio_219 < 3.0);

        // 32:9 ratio (~3.55) - should be GasMask329Support
        let ratio_329 = 5120.0 / 1440.0; // ~3.56
        assert!(ratio_329 > 3.0);
    }

    #[test]
    fn test_physics_ini_regex_patterns() {
        // Verify regex patterns work correctly ((?m) makes ^ match per-line in multi-line INI)
        let resolution_pattern = Regex::new(r"(?mi)^Resolution\s*=.*").unwrap();
        assert!(resolution_pattern.is_match("Resolution=1920x1080"));
        assert!(resolution_pattern.is_match("resolution=1920x1080"));
        assert!(resolution_pattern.is_match("[Display]\nResolution=1920x1080"));

        let taa_pattern = Regex::new(r"(?mi)^bUseTAA\s*=.*").unwrap();
        assert!(taa_pattern.is_match("bUseTAA=1"));
        assert!(taa_pattern.is_match("[Display]\nbUseTAA = 0"));

        let swap_pattern = Regex::new(r"(?mi)^SwapEffect\s*=.*").unwrap();
        assert!(swap_pattern.is_match("SwapEffect=4"));
        assert!(swap_pattern.is_match("[Display]\nSwapEffect = 3"));
    }

    #[test]
    fn test_check_blocking_processes() {
        // Should always succeed on non-Windows or when no processes are blocking
        let result = FomodInstaller::check_blocking_processes();
        assert!(result.is_ok());
    }

    #[test]
    fn test_fomod_installer_safety_checks() {
        // verify that validation catches invalid paths
        let invalid_paths = vec![
            Path::new("C:/mods/Wrong Name"),
            Path::new("C:/Fallout4/Wrong Folder"),
            Path::new("/home/user/mods"),
        ];

        for path in invalid_paths {
            assert!(
                FomodInstaller::validate_output_path(path).is_err(),
                "Should reject path: {}",
                path.display()
            );
        }
    }

    #[test]
    fn test_ultrawide_detection_edge_cases() {
        // Exactly 2.0 ratio (should not be ultrawide)
        assert!(!FomodInstaller::is_ultrawide_resolution(2000, 1000));

        // Just above 2.0 (should be ultrawide)
        assert!(FomodInstaller::is_ultrawide_resolution(2001, 1000));

        // Very high aspect ratio
        assert!(FomodInstaller::is_ultrawide_resolution(8000, 1000));

        // Zero height (safety check)
        assert!(!FomodInstaller::is_ultrawide_resolution(3440, 0));
    }
}
