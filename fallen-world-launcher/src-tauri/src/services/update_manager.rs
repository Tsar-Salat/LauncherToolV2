use crate::models::ModUpdate;
use crate::services::{PathDiscoveryService, ModDetector};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

pub struct UpdateManager;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct ModVersionInfo {
    name: String,
    current_version: String,
    latest_version: String,
    download_url: Option<String>,
    changelog: Option<String>,
}

impl UpdateManager {
    /// Check for mod updates by comparing installed vs available versions
    pub async fn check_updates() -> Result<Vec<ModUpdate>, String> {
        let game_paths = PathDiscoveryService::discover()
            .map_err(|e| format!("Cannot determine game paths: {}", e))?;

        // Get list of installed mods
        let installed_mods = ModDetector::get_installed_mod_list(&game_paths.mods_folder)
            .map_err(|e| format!("Cannot get installed mods: {}", e))?;

        // Fetch version information from metadata file
        let available_versions =
            Self::fetch_version_metadata(&game_paths.mods_folder).await?;

        let mut updates = Vec::new();

        // Compare versions
        for installed_mod in installed_mods {
            if let Some(available) = available_versions.get(&installed_mod.name) {
                if Self::should_update(&installed_mod.version, &available.latest_version) {
                    updates.push(ModUpdate {
                        mod_id: installed_mod.id.clone(),
                        current_version: installed_mod.version.clone(),
                        available_version: available.latest_version.clone(),
                        changelog: available.changelog.clone().unwrap_or_default(),
                    });
                }
            }
        }

        Ok(updates)
    }

    /// Fetch version metadata from version file or remote source
    async fn fetch_version_metadata(
        mods_folder: &str,
    ) -> Result<HashMap<String, ModVersionInfo>, String> {
        let mut versions = HashMap::new();

        // First, try to load from local metadata file
        let metadata_path = format!("{}/.version_info.json", mods_folder);
        if Path::new(&metadata_path).exists() {
            if let Ok(content) = fs::read_to_string(&metadata_path) {
                if let Ok(parsed) = serde_json::from_str::<Vec<ModVersionInfo>>(&content) {
                    for mod_info in parsed {
                        versions.insert(mod_info.name.clone(), mod_info);
                    }
                    return Ok(versions);
                }
            }
        }

        // Try to fetch from remote source (simulated for now)
        // In production, this would fetch from GitHub, Nexus API, etc.
        // For now, return empty to indicate no updates available
        Ok(versions)
    }

    /// Download and install a mod update
    pub async fn update_mod(mod_id: &str) -> Result<(), String> {
        let game_paths = PathDiscoveryService::discover()
            .map_err(|e| format!("Cannot determine game paths: {}", e))?;

        // Get available versions
        let available_versions = Self::fetch_version_metadata(&game_paths.mods_folder).await?;

        // Find the mod to update
        let mut found = false;
        for mod_info in available_versions.values() {
            if mod_info.name == mod_id {
                found = true;

                // If download URL is available, download the update
                if let Some(url) = &mod_info.download_url {
                    Self::download_and_install_mod(url, mod_id, &game_paths.mods_folder)
                        .await?;
                } else {
                    return Err(
                        "No download URL available for this mod. Please update manually.".to_string()
                    );
                }

                break;
            }
        }

        if !found {
            return Err(format!("No update information found for mod: {}", mod_id));
        }

        Ok(())
    }

    /// Download and install a mod
    async fn download_and_install_mod(
        url: &str,
        mod_id: &str,
        mods_folder: &str,
    ) -> Result<(), String> {
        // Create temporary directory for download
        let temp_dir = format!("{}/._update_temp", mods_folder);
        fs::create_dir_all(&temp_dir)
            .map_err(|e| format!("Cannot create temp directory: {}", e))?;

        // Simulate download (in production, would use reqwest or similar)

        // Clean up temp directory
        let _ = fs::remove_dir_all(&temp_dir);

        Ok(())
    }

    /// Get changelog for a specific mod
    pub async fn get_changelog(mod_id: &str) -> Result<Option<String>, String> {
        let game_paths = PathDiscoveryService::discover()
            .map_err(|e| format!("Cannot determine game paths: {}", e))?;

        let available_versions =
            Self::fetch_version_metadata(&game_paths.mods_folder).await?;

        // Find the mod and return its changelog
        for mod_info in available_versions.values() {
            if mod_info.name == mod_id {
                return Ok(mod_info.changelog.clone());
            }
        }

        Ok(None)
    }

    /// Check for launcher updates from GitHub
    pub async fn check_launcher_update() -> Result<Option<String>, String> {
        // In production, this would check GitHub releases for new launcher versions
        // For now, return None (no updates available)
        // Simulated: Could check against current version in Cargo.toml

        // Version would be compared against latest release on GitHub
        Ok(None)
    }

    /// Compare two version strings (semantic versioning)
    fn should_update(current: &str, latest: &str) -> bool {
        let current_parts: Vec<&str> = current.split('.').collect();
        let latest_parts: Vec<&str> = latest.split('.').collect();

        // Parse major.minor.patch
        for i in 0..std::cmp::min(current_parts.len(), latest_parts.len()) {
            let curr_num: u32 = current_parts[i].parse().unwrap_or(0);
            let latest_num: u32 = latest_parts[i].parse().unwrap_or(0);

            if latest_num > curr_num {
                return true;
            } else if curr_num > latest_num {
                return false;
            }
        }

        // If all compared parts are equal, check if latest has more parts
        latest_parts.len() > current_parts.len()
    }

    /// Get all available mod versions
    pub async fn get_all_versions(
    ) -> Result<Vec<ModVersionInfo>, String> {
        let game_paths = PathDiscoveryService::discover()
            .map_err(|e| format!("Cannot determine game paths: {}", e))?;

        let available = Self::fetch_version_metadata(&game_paths.mods_folder).await?;

        Ok(available.into_values().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_comparison_patch() {
        assert!(UpdateManager::should_update("1.0.0", "1.0.1"));
    }

    #[test]
    fn test_version_comparison_minor() {
        assert!(UpdateManager::should_update("1.0.0", "1.1.0"));
    }

    #[test]
    fn test_version_comparison_major() {
        assert!(UpdateManager::should_update("1.0.0", "2.0.0"));
    }

    #[test]
    fn test_version_comparison_no_update() {
        assert!(!UpdateManager::should_update("1.0.0", "1.0.0"));
    }

    #[test]
    fn test_version_comparison_downgrade() {
        assert!(!UpdateManager::should_update("2.0.0", "1.0.0"));
    }

    #[test]
    fn test_version_comparison_partial() {
        assert!(UpdateManager::should_update("1.0", "1.0.1"));
    }
}
