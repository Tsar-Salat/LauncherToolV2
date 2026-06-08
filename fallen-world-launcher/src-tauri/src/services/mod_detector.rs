use std::path::PathBuf;
use std::fs;
use crate::models::Mod;

pub struct SimpleMod {
    pub id: String,
    pub name: String,
    pub version: String,
}

pub struct ModDetector;

impl ModDetector {
    /// Get list of installed mods with basic info (for updates)
    pub fn get_installed_mod_list(mods_folder: &str) -> Result<Vec<SimpleMod>, String> {
        let mods_path = PathBuf::from(mods_folder);

        // Missing mods folder = no installed mods, not an error
        if !mods_path.exists() {
            return Ok(Vec::new());
        }

        let mut mods = Vec::new();
        let entries = fs::read_dir(&mods_path)
            .map_err(|e| format!("Cannot read mods folder: {}", e))?;

        for entry in entries {
            let entry = entry.map_err(|e| format!("Error reading entry: {}", e))?;
            let path = entry.path();

            if !path.is_dir() {
                continue;
            }

            let folder_name = path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();

            // Skip special folders
            if Self::is_special_folder(&folder_name) {
                continue;
            }

            mods.push(SimpleMod {
                id: folder_name.clone(),
                name: folder_name.clone(),
                version: "1.0.0".to_string(), // Default version, would be read from metadata
            });
        }

        Ok(mods)
    }

    /// Scan mods folder and list all mods
    pub fn list_mods(
        mods_folder: &str,
        mo2_root: Option<&str>,
        mo2_profile: Option<&str>,
    ) -> Result<Vec<Mod>, String> {
        let mods_path = PathBuf::from(mods_folder);

        // Missing mods folder isn't fatal — many setups (e.g. fresh installs,
        // some MO2 portable instances) won't have one until mods are added.
        // Return an empty list so the UI shows "No mods" rather than an error.
        if !mods_path.exists() {
            return Ok(Vec::new());
        }

        let mut mods = Vec::new();
        let enabled_mods = match (mo2_root, mo2_profile) {
            (Some(root), Some(profile)) => Self::read_mo2_enabled_mods(root, profile)?,
            _ => Vec::new(),
        };

        let entries = fs::read_dir(&mods_path)
            .map_err(|e| format!("Cannot read mods folder: {}", e))?;

        for entry in entries {
            let entry = entry.map_err(|e| format!("Error reading entry: {}", e))?;
            let path = entry.path();

            if !path.is_dir() {
                continue;
            }

            let folder_name = path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();

            // Skip special folders
            if Self::is_special_folder(&folder_name) {
                continue;
            }

            let enabled = enabled_mods.contains(&folder_name);

            mods.push(Mod {
                id: folder_name.clone(),
                name: Self::folder_to_display_name(&folder_name),
                version: "".to_string(),
                author: "".to_string(),
                description: "".to_string(),
                enabled,
                load_order: mods.len() as i32,
                dependencies: Vec::new(),
                conflicts: Vec::new(),
                last_updated: "".to_string(),
            });
        }

        // Sort: enabled mods first, then by name
        mods.sort_by(|a, b| {
            match b.enabled.cmp(&a.enabled) {
                std::cmp::Ordering::Equal => a.name.cmp(&b.name),
                other => other,
            }
        });

        // Re-number order
        for (i, m) in mods.iter_mut().enumerate() {
            m.load_order = i as i32;
        }

        Ok(mods)
    }

    /// Read enabled mods from modlist.txt in MO2 profile
    fn read_mo2_enabled_mods(mo2_root: &str, profile_name: &str) -> Result<Vec<String>, String> {
        // MO2 stores enabled mods in profiles/<profile>/modlist.txt
        // Format: +ModFolderName (+ = enabled, - = disabled)

        let modlist_path = std::path::PathBuf::from(mo2_root)
            .join("profiles")
            .join(profile_name)
            .join("modlist.txt");

        if !modlist_path.exists() {
            return Ok(Vec::new());
        }

        let content = std::fs::read_to_string(&modlist_path)
            .map_err(|e| format!("Cannot read modlist.txt: {}", e))?;

        let mut enabled = Vec::new();

        for line in content.lines() {
            if let Some(mod_name) = Self::parse_modlist_line(line) {
                enabled.push(mod_name);
            }
        }

        Ok(enabled)
    }

    /// Parse a single line from modlist.txt
    /// Format: +ModName (enabled) or -ModName (disabled)
    /// Returns: Some(mod_name) if enabled, None if disabled or invalid
    pub fn parse_modlist_line(line: &str) -> Option<String> {
        let trimmed = line.trim();

        if trimmed.is_empty() {
            return None;
        }

        // Must start with + (enabled) or - (disabled)
        if !trimmed.starts_with('+') && !trimmed.starts_with('-') {
            return None;
        }

        // Return only if enabled (starts with +)
        if trimmed.starts_with('+') {
            let mod_name = trimmed.trim_start_matches('+').trim();
            if !mod_name.is_empty() {
                return Some(mod_name.to_string());
            }
        }

        None
    }

    /// Check if folder name is a special folder that should be ignored
    fn is_special_folder(name: &str) -> bool {
        let lower = name.to_lowercase();
        matches!(lower.as_str(),
            "fallen world fomod resources" |
            "fallen world optional mods" |
            "fomod resources" |
            "optional mods" |
            "overwrite" |
            "downloads" |
            "backup" |
            ".git" |
            ".hg" |
            ".svn" |
            "._" |
            "__pycache__" |
            ".venv" |
            "node_modules"
        )
    }

    /// Convert folder name to display name
    /// Converts "My_Awesome_Mod" -> "My Awesome Mod"
    fn folder_to_display_name(folder: &str) -> String {
        folder
            .replace('_', " ")
            .replace('-', " ")
            .split_whitespace()
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Get load order from MO2 if available
    pub fn get_load_order(mo2_root: Option<&str>, mo2_profile: Option<&str>) -> Result<Vec<String>, String> {
        match (mo2_root, mo2_profile) {
            (Some(root), Some(profile)) => Self::read_mo2_load_order(root, profile),
            _ => Ok(Vec::new()),
        }
    }

    /// Read load order from MO2's modlist.txt
    /// Returns all mod names in order (enabled + disabled)
    fn read_mo2_load_order(mo2_root: &str, profile_name: &str) -> Result<Vec<String>, String> {
        let modlist_path = std::path::PathBuf::from(mo2_root)
            .join("profiles")
            .join(profile_name)
            .join("modlist.txt");

        if !modlist_path.exists() {
            return Ok(Vec::new());
        }

        let content = std::fs::read_to_string(&modlist_path)
            .map_err(|e| format!("Cannot read modlist.txt: {}", e))?;

        let mut load_order = Vec::new();

        for line in content.lines() {
            let trimmed = line.trim();

            if trimmed.is_empty() {
                continue;
            }

            // Parse both + and - prefixes
            if trimmed.starts_with('+') || trimmed.starts_with('-') {
                let mod_name = trimmed[1..].trim();
                if !mod_name.is_empty() {
                    load_order.push(mod_name.to_string());
                }
            }
        }

        Ok(load_order)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_folder_to_display_name() {
        assert_eq!(ModDetector::folder_to_display_name("My_Awesome_Mod"), "My Awesome Mod");
        assert_eq!(ModDetector::folder_to_display_name("test-mod"), "Test Mod");
        assert_eq!(ModDetector::folder_to_display_name("Fallout4"), "Fallout4");
    }

    #[test]
    fn test_is_special_folder() {
        assert!(ModDetector::is_special_folder("Overwrite"));
        assert!(ModDetector::is_special_folder("Downloads"));
        assert!(!ModDetector::is_special_folder("My Mod"));
    }

    #[test]
    fn test_parse_modlist_line_enabled() {
        let result = ModDetector::parse_modlist_line("+My Awesome Mod");
        assert_eq!(result, Some("My Awesome Mod".to_string()));
    }

    #[test]
    fn test_parse_modlist_line_disabled() {
        let result = ModDetector::parse_modlist_line("-My Awesome Mod");
        assert_eq!(result, None);
    }

    #[test]
    fn test_parse_modlist_line_empty() {
        assert_eq!(ModDetector::parse_modlist_line(""), None);
        assert_eq!(ModDetector::parse_modlist_line("   "), None);
    }

    #[test]
    fn test_parse_modlist_line_invalid() {
        assert_eq!(ModDetector::parse_modlist_line("No Prefix Mod"), None);
        assert_eq!(ModDetector::parse_modlist_line("+"), None);
    }

    #[test]
    fn test_parse_modlist_line_whitespace() {
        let result = ModDetector::parse_modlist_line("  +Trimmed Mod  ");
        assert_eq!(result, Some("Trimmed Mod".to_string()));
    }
}
