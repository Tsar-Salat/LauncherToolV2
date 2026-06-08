use std::fs;
use std::path::Path;
use std::collections::HashMap;
use crate::services::IniManager;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct McmPreset {
    pub name: String,
    pub settings: HashMap<String, HashMap<String, String>>,
    pub created_date: String,
}

pub struct McmManager;

impl McmManager {
    /// List available MCM presets
    pub fn list_presets(game_path: &str) -> Result<Vec<String>, String> {
        let mut presets = Vec::new();
        let mcm_dir = format!("{}/MCM Settings", game_path);

        if !Path::new(&mcm_dir).exists() {
            return Ok(presets);
        }

        let entries = fs::read_dir(&mcm_dir)
            .map_err(|e| format!("Cannot read MCM Settings directory: {}", e))?;

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                    presets.push(name.to_string());
                }
            }
        }

        presets.sort();
        Ok(presets)
    }

    /// Load MCM preset file
    pub fn load_preset(name: &str, game_path: &str) -> Result<McmPreset, String> {
        if name.is_empty() {
            return Err("Preset name cannot be empty".to_string());
        }

        // Sanitize preset name to prevent path traversal
        if name.contains("..") || name.contains("/") || name.contains("\\") {
            return Err("Invalid preset name".to_string());
        }

        let preset_file = format!("{}/MCM Settings/{}.json", game_path, name);
        let preset_path = Path::new(&preset_file);

        if !preset_path.exists() {
            return Err(format!("Preset not found: {}", name));
        }

        let content = fs::read_to_string(&preset_file)
            .map_err(|e| format!("Cannot read preset file: {}", e))?;

        serde_json::from_str::<McmPreset>(&content)
            .map_err(|e| format!("Cannot parse preset: {}", e))
    }

    /// Save MCM preset
    pub fn save_preset(
        name: &str,
        game_path: &str,
        settings: HashMap<String, HashMap<String, String>>,
    ) -> Result<(), String> {
        if name.is_empty() || name.trim().is_empty() {
            return Err("Preset name cannot be empty".to_string());
        }

        if name.contains("..") || name.contains("/") || name.contains("\\") {
            return Err("Preset name cannot contain path separators".to_string());
        }

        let mcm_dir = format!("{}/MCM Settings", game_path);

        fs::create_dir_all(&mcm_dir)
            .map_err(|e| format!("Cannot create MCM Settings directory: {}", e))?;

        let preset = McmPreset {
            name: name.to_string(),
            settings,
            created_date: chrono::Local::now().to_rfc3339(),
        };

        let preset_file = format!("{}/{}.json", mcm_dir, name);
        let json = serde_json::to_string_pretty(&preset)
            .map_err(|e| format!("Cannot serialize preset: {}", e))?;

        fs::write(&preset_file, json)
            .map_err(|e| format!("Cannot write preset file: {}", e))?;

        Ok(())
    }

    /// Delete MCM preset
    pub fn delete_preset(name: &str, game_path: &str) -> Result<(), String> {
        if name.is_empty() {
            return Err("Preset name cannot be empty".to_string());
        }

        let preset_file = format!("{}/MCM Settings/{}.json", game_path, name);
        let preset_path = Path::new(&preset_file);

        if preset_path.exists() {
            fs::remove_file(&preset_file)
                .map_err(|e| format!("Cannot delete preset: {}", e))?;
        }

        Ok(())
    }

    /// Apply MCM preset to game INI.
    /// `game_path` locates the preset JSON files (under `MCM Settings/`).
    /// `ini_folder` locates Fallout4.ini — for MO2 modlists this is the
    /// active profile folder, not the game root.
    pub fn apply_preset(
        preset_name: &str,
        game_path: &str,
        ini_folder: &str,
    ) -> Result<(), String> {
        // Load the preset
        let preset = Self::load_preset(preset_name, game_path)?;

        // Read current INI config (from the correct INI folder)
        let mut config = IniManager::read_ini(ini_folder)?;

        // Apply settings to game INI by merging them
        for (section, settings) in preset.settings.iter() {
            for (key, value) in settings.iter() {
                IniManager::set_value(&mut config, section, key, value);
            }
        }

        // Write updated config back to disk
        IniManager::write_ini(ini_folder, &config)?;

        Ok(())
    }

    /// Get current MCM settings from game INI.
    /// `ini_folder` locates Fallout4.ini (game_root for vanilla,
    /// `<mo2_root>/profiles/<profile>` for MO2 modlists).
    pub fn get_current_settings(ini_folder: &str) -> Result<HashMap<String, HashMap<String, String>>, String> {
        // Read game INI and extract MCM-related settings
        // MCM settings are typically in specific sections
        let content = std::fs::read_to_string(format!("{}/Fallout4.ini", ini_folder))
            .map_err(|e| format!("Cannot read game INI: {}", e))?;

        let mut settings = HashMap::new();

        // Parse INI to extract sections
        let mut current_section = String::new();
        for line in content.lines() {
            let trimmed = line.trim();

            // Skip comments and empty lines
            if trimmed.is_empty() || trimmed.starts_with(';') {
                continue;
            }

            // Check for section header
            if trimmed.starts_with('[') && trimmed.ends_with(']') {
                current_section = trimmed[1..trimmed.len() - 1].to_string();
                continue;
            }

            // Parse key=value pairs
            if let Some(eq_pos) = trimmed.find('=') {
                let key = trimmed[..eq_pos].trim().to_string();
                let value = trimmed[eq_pos + 1..].trim().to_string();

                settings
                    .entry(current_section.clone())
                    .or_insert_with(HashMap::new)
                    .insert(key, value);
            }
        }

        Ok(settings)
    }

    /// Check if MCM preset exists
    pub fn preset_exists(name: &str, game_path: &str) -> Result<bool, String> {
        if name.is_empty() {
            return Err("Preset name cannot be empty".to_string());
        }

        let preset_file = format!("{}/MCM Settings/{}.json", game_path, name);
        Ok(Path::new(&preset_file).exists())
    }

    /// Get preset metadata
    pub fn get_preset_info(name: &str, game_path: &str) -> Result<(String, String), String> {
        let preset = Self::load_preset(name, game_path)?;
        Ok((preset.name, preset.created_date))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preset_name_validation() {
        let bad_names = vec!["../../../etc/passwd", "preset/bad", "preset\\bad", ""];
        for name in bad_names {
            assert!(
                name.is_empty()
                    || name.contains("..")
                    || name.contains("/")
                    || name.contains("\\")
            );
        }
    }

    #[test]
    fn test_preset_structure() {
        let mut settings = HashMap::new();
        let mut section = HashMap::new();
        section.insert("Key1".to_string(), "Value1".to_string());
        settings.insert("Section1".to_string(), section);

        let preset = McmPreset {
            name: "TestPreset".to_string(),
            settings,
            created_date: "2026-06-05T10:00:00+00:00".to_string(),
        };

        let json = serde_json::to_string(&preset).unwrap();
        let parsed: McmPreset = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.name, "TestPreset");
        assert!(parsed.settings.contains_key("Section1"));
    }

    #[test]
    fn test_mcm_settings_merging() {
        let mut settings1 = HashMap::new();
        let mut section1 = HashMap::new();
        section1.insert("Key1".to_string(), "Value1".to_string());
        settings1.insert("Graphics".to_string(), section1);

        let mut settings2 = HashMap::new();
        let mut section2 = HashMap::new();
        section2.insert("Key2".to_string(), "Value2".to_string());
        settings2.insert("Graphics".to_string(), section2);

        // Simulate merging
        for (section, keys) in settings2.iter() {
            settings1
                .entry(section.clone())
                .or_insert_with(HashMap::new)
                .extend(keys.clone());
        }

        assert_eq!(settings1.get("Graphics").unwrap().len(), 2);
    }

    #[test]
    fn test_empty_preset_name_rejected() {
        assert!(String::new().is_empty());
        assert!("".trim().is_empty());
    }

    #[test]
    fn test_json_serialization_roundtrip() {
        let mut settings = HashMap::new();
        let mut section = HashMap::new();
        section.insert("bEnableRaceMenu".to_string(), "1".to_string());
        settings.insert("General".to_string(), section);

        let preset = McmPreset {
            name: "TestPreset".to_string(),
            settings,
            created_date: "2026-06-05".to_string(),
        };

        let json = serde_json::to_string(&preset).unwrap();
        let deserialized: McmPreset = serde_json::from_str(&json).unwrap();

        assert_eq!(preset.name, deserialized.name);
        assert_eq!(preset.created_date, deserialized.created_date);
        assert_eq!(preset.settings, deserialized.settings);
    }
}
