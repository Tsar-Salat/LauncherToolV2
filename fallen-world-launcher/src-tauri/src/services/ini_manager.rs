use std::collections::HashMap;
use std::path::PathBuf;
use crate::models::IniConfig;
use serde::{Deserialize, Serialize};

/// The Fallout 4 INI files the editor knows about, in load order.
pub const INI_FILES: [&str; 3] = ["Fallout4.ini", "Fallout4Prefs.ini", "Fallout4Custom.ini"];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IniFileInfo {
    pub name: String,
    pub size: u64,
}

/// A single section/key/value edit to apply to one INI file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IniChange {
    pub section: String,
    pub key: String,
    pub value: String,
}

pub struct IniManager;

impl IniManager {
    /// Read and parse Fallout4 INI files from the given folder.
    /// MO2 profiles split settings across Fallout4.ini, Fallout4Prefs.ini, and
    /// Fallout4Custom.ini. We read all three that exist and merge their sections
    /// so the editor shows the complete picture.
    pub fn read_ini(ini_folder: &str) -> Result<IniConfig, String> {
        let folder = PathBuf::from(ini_folder);
        let candidates = ["Fallout4.ini", "Fallout4Prefs.ini", "Fallout4Custom.ini"];

        let mut merged = IniConfig { sections: HashMap::new() };
        let mut found_any = false;

        for filename in &candidates {
            let path = folder.join(filename);
            if !path.exists() {
                continue;
            }
            found_any = true;
            if let Ok(content) = std::fs::read_to_string(&path) {
                let parsed = Self::parse_ini_content(&content);
                for (section, entries) in parsed.sections {
                    merged.sections
                        .entry(section)
                        .or_insert_with(HashMap::new)
                        .extend(entries);
                }
            }
        }

        if !found_any {
            return Err(format!("No Fallout4 INI files found in {}", ini_folder));
        }

        Ok(merged)
    }

    /// List which known INI files exist in the folder, with sizes.
    pub fn list_files(ini_folder: &str) -> Vec<IniFileInfo> {
        let folder = PathBuf::from(ini_folder);
        INI_FILES
            .iter()
            .filter_map(|name| {
                let path = folder.join(name);
                std::fs::metadata(&path).ok().map(|m| IniFileInfo {
                    name: name.to_string(),
                    size: m.len(),
                })
            })
            .collect()
    }

    /// Read and parse a single INI file by name.
    pub fn read_file(ini_folder: &str, filename: &str) -> Result<IniConfig, String> {
        Self::guard_filename(filename)?;
        let path = PathBuf::from(ini_folder).join(filename);
        let content = std::fs::read_to_string(&path)
            .map_err(|e| format!("Cannot read {}: {}", filename, e))?;
        Ok(Self::parse_ini_content(&content))
    }

    /// Apply a batch of changes to one INI file, editing lines in place so
    /// comments, ordering, and untouched keys are preserved. Missing keys are
    /// appended under their section; missing sections are appended at the end.
    pub fn save_changes(ini_folder: &str, filename: &str, changes: &[IniChange]) -> Result<(), String> {
        Self::guard_filename(filename)?;
        let path = PathBuf::from(ini_folder).join(filename);
        let content = std::fs::read_to_string(&path).unwrap_or_default();
        let mut lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();
        let mut applied = vec![false; changes.len()];

        // Pass 1 — in-place replacement of existing keys.
        let mut current = String::new();
        for line in lines.iter_mut() {
            let trimmed = line.trim();
            if trimmed.starts_with('[') && trimmed.ends_with(']') {
                current = trimmed[1..trimmed.len() - 1].to_lowercase();
                continue;
            }
            let Some(eq) = line.find('=') else { continue };
            let key_lower = line[..eq].trim().to_lowercase();
            for (i, c) in changes.iter().enumerate() {
                if !applied[i] && c.section.to_lowercase() == current && c.key.to_lowercase() == key_lower {
                    let key_part = line[..eq].trim_end().to_string();
                    *line = format!("{}={}", key_part, c.value);
                    applied[i] = true;
                }
            }
        }

        // Pass 2 — append remaining changes under their (existing or new) section.
        for (i, c) in changes.iter().enumerate() {
            if applied[i] {
                continue;
            }
            match Self::section_end_index(&lines, &c.section) {
                Some(end) => lines.insert(end, format!("{}={}", c.key, c.value)),
                None => {
                    if !lines.is_empty() && !lines.last().map(|l| l.trim().is_empty()).unwrap_or(true) {
                        lines.push(String::new());
                    }
                    lines.push(format!("[{}]", c.section));
                    lines.push(format!("{}={}", c.key, c.value));
                }
            }
        }

        let mut out = lines.join("\n");
        if !out.ends_with('\n') {
            out.push('\n');
        }
        std::fs::write(&path, out).map_err(|e| format!("Cannot write {}: {}", filename, e))
    }

    /// Index just past the last content line of `section` (where a new key would
    /// be inserted), or None if the section header isn't present.
    fn section_end_index(lines: &[String], section: &str) -> Option<usize> {
        let target = section.to_lowercase();
        let mut in_section = false;
        let mut last_content = None;
        for (i, line) in lines.iter().enumerate() {
            let t = line.trim();
            if t.starts_with('[') && t.ends_with(']') {
                let name = t[1..t.len() - 1].to_lowercase();
                if in_section {
                    break; // reached the next section
                }
                in_section = name == target;
                if in_section {
                    last_content = Some(i + 1);
                }
            } else if in_section && !t.is_empty() {
                last_content = Some(i + 1);
            }
        }
        last_content
    }

    fn guard_filename(filename: &str) -> Result<(), String> {
        if INI_FILES.contains(&filename) {
            Ok(())
        } else {
            Err(format!("Unsupported INI file: {}", filename))
        }
    }

    /// Write INI config back to disk
    pub fn write_ini(ini_folder: &str, config: &IniConfig) -> Result<(), String> {
        let ini_path = PathBuf::from(ini_folder).join("Fallout4.ini");

        let content = Self::serialize_ini(config);

        std::fs::write(&ini_path, content)
            .map_err(|e| format!("Cannot write Fallout4.ini: {}", e))
    }

    /// Create backup of INI file before changes
    pub fn backup_ini(ini_folder: &str) -> Result<String, String> {
        let ini_path = PathBuf::from(ini_folder).join("Fallout4.ini");
        let backup_path = ini_path.with_extension("ini.bak");

        if !ini_path.exists() {
            return Err(format!("Fallout4.ini not found in {}", ini_folder));
        }

        std::fs::copy(&ini_path, &backup_path)
            .map_err(|e| format!("Cannot backup INI: {}", e))?;

        Ok(backup_path.to_string_lossy().to_string())
    }

    /// Restore INI from backup
    pub fn restore_ini(ini_folder: &str) -> Result<(), String> {
        let ini_path = PathBuf::from(ini_folder).join("Fallout4.ini");
        let backup_path = ini_path.with_extension("ini.bak");

        if !backup_path.exists() {
            return Err("No backup file found".to_string());
        }

        std::fs::copy(&backup_path, &ini_path)
            .map_err(|e| format!("Cannot restore from backup: {}", e))?;

        Ok(())
    }

    /// Get a specific INI value
    pub fn get_value(config: &IniConfig, section: &str, key: &str) -> Option<String> {
        config
            .sections
            .get(section)
            .and_then(|section_map| section_map.get(key).cloned())
    }

    /// Set a specific INI value (creates section if needed)
    pub fn set_value(config: &mut IniConfig, section: &str, key: &str, value: &str) {
        config
            .sections
            .entry(section.to_string())
            .or_insert_with(HashMap::new)
            .insert(key.to_string(), value.to_string());
    }

    /// Apply optimization preset
    pub fn apply_preset(config: &mut IniConfig, preset: &str) -> Result<(), String> {
        let settings = match preset {
            "Performance" => vec![
                ("Display", "iMaxAnisotropy", "1"),
                ("Display", "iMaximumAnimatedObjectCount", "4"),
                ("Display", "iTexMipSkip", "3"),
                ("Display", "bUseMap", "0"),
                ("Audio", "iAudioMasterVolume", "80"),
                ("Interface", "fMenuBGAlpha", "0.5"),
            ],
            "Graphics" => vec![
                ("Display", "iMaxAnisotropy", "8"),
                ("Display", "iMaximumAnimatedObjectCount", "512"),
                ("Display", "iTexMipSkip", "1"),
                ("Display", "bUseMap", "1"),
                ("Audio", "iAudioMasterVolume", "100"),
                ("Interface", "fMenuBGAlpha", "0.8"),
            ],
            "Ultra" => vec![
                ("Display", "iMaxAnisotropy", "16"),
                ("Display", "iMaximumAnimatedObjectCount", "512"),
                ("Display", "iTexMipSkip", "0"),
                ("Display", "bUseMap", "1"),
                ("Display", "iShadowMapResolution", "4096"),
                ("Audio", "iAudioMasterVolume", "100"),
                ("Interface", "fMenuBGAlpha", "1.0"),
            ],
            _ => return Err(format!("Unknown preset: {}", preset)),
        };

        for (section, key, value) in settings {
            Self::set_value(config, section, key, value);
        }

        Ok(())
    }

    /// Parse INI file content into sections and key-value pairs
    fn parse_ini_content(content: &str) -> IniConfig {
        let mut sections = HashMap::new();
        let mut current_section = String::new();

        for line in content.lines() {
            let trimmed = line.trim();

            // Skip empty lines and comments
            if trimmed.is_empty() || trimmed.starts_with(';') {
                continue;
            }

            // Section header: [SectionName]
            if trimmed.starts_with('[') && trimmed.ends_with(']') {
                current_section = trimmed[1..trimmed.len() - 1].to_string();
                sections.entry(current_section.clone()).or_insert_with(HashMap::new);
                continue;
            }

            // Key=Value pair
            if let Some(eq_pos) = trimmed.find('=') {
                let key = trimmed[..eq_pos].trim().to_string();
                let mut value = trimmed[eq_pos + 1..].trim().to_string();

                // Remove quotes if present
                if value.starts_with('"') && value.ends_with('"') {
                    value = value[1..value.len() - 1].to_string();
                }

                sections
                    .entry(current_section.clone())
                    .or_insert_with(HashMap::new)
                    .insert(key, value);
            }
        }

        IniConfig { sections }
    }

    /// Serialize INI config back to string format
    fn serialize_ini(config: &IniConfig) -> String {
        let mut result = String::new();

        // Sort sections for consistent output
        let mut section_names: Vec<_> = config.sections.keys().collect();
        section_names.sort();

        for section_name in section_names {
            if let Some(entries) = config.sections.get(section_name) {
                result.push_str(&format!("[{}]\n", section_name));

                // Sort entries within section
                let mut keys: Vec<_> = entries.keys().collect();
                keys.sort();

                for key in keys {
                    if let Some(value) = entries.get(key) {
                        result.push_str(&format!("{}={}\n", key, value));
                    }
                }

                result.push('\n');
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ini_basic() {
        let content = "[Display]\niResolution=1920x1080\nbBorderless=1\n";
        let config = IniManager::parse_ini_content(content);

        assert_eq!(config.sections.len(), 1);
        assert_eq!(
            IniManager::get_value(&config, "Display", "iResolution"),
            Some("1920x1080".to_string())
        );
        assert_eq!(
            IniManager::get_value(&config, "Display", "bBorderless"),
            Some("1".to_string())
        );
    }

    #[test]
    fn test_parse_ini_with_comments() {
        let content = "; Comment\n[Display]\n; Another comment\niResolution=1920x1080\n";
        let config = IniManager::parse_ini_content(content);

        assert_eq!(
            IniManager::get_value(&config, "Display", "iResolution"),
            Some("1920x1080".to_string())
        );
    }

    #[test]
    fn test_parse_ini_multiple_sections() {
        let content = "[Display]\niResolution=1920x1080\n\n[Audio]\niMasterVolume=100\n";
        let config = IniManager::parse_ini_content(content);

        assert_eq!(config.sections.len(), 2);
        assert!(config.sections.contains_key("Display"));
        assert!(config.sections.contains_key("Audio"));
    }

    #[test]
    fn test_parse_ini_with_quotes() {
        let content = "[Interface]\nsValue=\"My String Value\"\n";
        let config = IniManager::parse_ini_content(content);

        assert_eq!(
            IniManager::get_value(&config, "Interface", "sValue"),
            Some("My String Value".to_string())
        );
    }

    #[test]
    fn test_parse_ini_whitespace() {
        let content = "  [Display]  \n  iResolution  =  1920x1080  \n";
        let config = IniManager::parse_ini_content(content);

        assert_eq!(
            IniManager::get_value(&config, "Display", "iResolution"),
            Some("1920x1080".to_string())
        );
    }

    #[test]
    fn test_set_value_new_section() {
        let mut config = IniConfig {
            sections: HashMap::new(),
        };

        IniManager::set_value(&mut config, "Display", "iResolution", "3840x2160");

        assert_eq!(
            IniManager::get_value(&config, "Display", "iResolution"),
            Some("3840x2160".to_string())
        );
    }

    #[test]
    fn test_set_value_existing_section() {
        let mut sections = HashMap::new();
        let mut display = HashMap::new();
        display.insert("iResolution".to_string(), "1920x1080".to_string());
        sections.insert("Display".to_string(), display);

        let mut config = IniConfig { sections };

        IniManager::set_value(&mut config, "Display", "bBorderless", "1");

        assert_eq!(
            IniManager::get_value(&config, "Display", "iResolution"),
            Some("1920x1080".to_string())
        );
        assert_eq!(
            IniManager::get_value(&config, "Display", "bBorderless"),
            Some("1".to_string())
        );
    }

    #[test]
    fn test_serialize_ini() {
        let mut sections = HashMap::new();
        let mut display = HashMap::new();
        display.insert("iResolution".to_string(), "1920x1080".to_string());
        sections.insert("Display".to_string(), display);

        let config = IniConfig { sections };
        let output = IniManager::serialize_ini(&config);

        assert!(output.contains("[Display]"));
        assert!(output.contains("iResolution=1920x1080"));
    }

    #[test]
    fn test_apply_preset_performance() {
        let mut config = IniConfig {
            sections: HashMap::new(),
        };

        IniManager::apply_preset(&mut config, "Performance").unwrap();

        assert_eq!(
            IniManager::get_value(&config, "Display", "iMaxAnisotropy"),
            Some("1".to_string())
        );
        assert_eq!(
            IniManager::get_value(&config, "Display", "iTexMipSkip"),
            Some("3".to_string())
        );
    }

    #[test]
    fn test_apply_preset_ultra() {
        let mut config = IniConfig {
            sections: HashMap::new(),
        };

        IniManager::apply_preset(&mut config, "Ultra").unwrap();

        assert_eq!(
            IniManager::get_value(&config, "Display", "iMaxAnisotropy"),
            Some("16".to_string())
        );
        assert_eq!(
            IniManager::get_value(&config, "Display", "iTexMipSkip"),
            Some("0".to_string())
        );
    }

    #[test]
    fn test_apply_preset_invalid() {
        let mut config = IniConfig {
            sections: HashMap::new(),
        };

        let result = IniManager::apply_preset(&mut config, "Invalid");
        assert!(result.is_err());
    }
}
