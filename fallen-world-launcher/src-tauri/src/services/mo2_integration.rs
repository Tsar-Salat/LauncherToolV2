use std::fs;
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use regex::Regex;

pub struct Mo2Integration;

impl Mo2Integration {
    /// Enable .esp/.esm/.esl plugins in MO2's plugins.txt
    pub fn enable_plugins_in_mo2(
        output_folder: &Path,
        modlist_root: &Path,
    ) -> Result<bool, String> {
        let plugin_names = Self::collect_plugins(output_folder)?;
        if plugin_names.is_empty() {
            return Ok(false);
        }

        let active_profile = Self::get_active_profile(modlist_root)?;
        let plugins_file = modlist_root
            .join("profiles")
            .join(&active_profile)
            .join("plugins.txt");

        if !plugins_file.exists() {
            return Err("plugins.txt not found".to_string());
        }

        Self::add_plugins_to_file(&plugins_file, &plugin_names)
    }

    /// Collect all .esp, .esm, .esl files from folder
    fn collect_plugins(folder: &Path) -> Result<Vec<String>, String> {
        let mut plugins = Vec::new();

        if !folder.exists() || !folder.is_dir() {
            return Ok(plugins);
        }

        for entry in fs::read_dir(folder).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    let ext_str = ext.to_string_lossy().to_lowercase();
                    if ext_str == "esp" || ext_str == "esm" || ext_str == "esl" {
                        if let Some(name) = path.file_name() {
                            plugins.push(name.to_string_lossy().to_string());
                        }
                    }
                }
            }
        }

        plugins.sort();
        Ok(plugins)
    }

    /// Read active profile from ModOrganizer.ini
    fn get_active_profile(modlist_root: &Path) -> Result<String, String> {
        let ini_path = modlist_root.join("ModOrganizer.ini");
        if !ini_path.exists() {
            return Err("ModOrganizer.ini not found".to_string());
        }

        let content = fs::read_to_string(&ini_path)
            .map_err(|e| format!("Failed to read ModOrganizer.ini: {}", e))?;

        // Try to extract selected_profile with or without @ByteArray()
        let profile_pattern = Regex::new(r"(?i)^\s*selected_profile\s*=\s*(?:@ByteArray\(([^)]*)\)|([^\r\n#;]+))")
            .unwrap();

        if let Some(caps) = profile_pattern.captures(&content) {
            let profile = caps
                .get(1)
                .or_else(|| caps.get(2))
                .map(|m| m.as_str().trim())
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string());

            if let Some(p) = profile {
                return Ok(p);
            }
        }

        Err("Could not determine active profile".to_string())
    }

    /// Add plugins to plugins.txt with smart placement
    fn add_plugins_to_file(plugins_file: &Path, plugin_names: &[String]) -> Result<bool, String> {
        let content = fs::read_to_string(plugins_file)
            .map_err(|e| format!("Failed to read plugins.txt: {}", e))?;

        let mut lines: Vec<String> = content
            .lines()
            .map(|ln| ln.trim_end().to_string())
            .collect();

        let mut changed = false;
        let mut name_to_idx: HashMap<String, usize> = HashMap::new();
        let mut master_anchor_index = 0;

        // Scan existing lines
        for (i, ln) in lines.iter().enumerate() {
            if !ln.is_empty() {
                let clean_name = ln.trim_start_matches('*').trim();
                let clean_lower = clean_name.to_lowercase();

                name_to_idx.insert(clean_lower.clone(), i);

                // Track master ESMs for insertion point
                if Self::is_master_esm(clean_lower) {
                    master_anchor_index = i + 1;
                }
            }
        }

        // Determine insertion point (after masters, before PRP if present)
        let mut insert_index = master_anchor_index;
        if let Some(&idx) = name_to_idx.get("prp.esp") {
            if idx < insert_index {
                insert_index = idx;
            }
        }
        if let Some(&idx) = name_to_idx.get("prp - patch.esp") {
            if idx < insert_index {
                insert_index = idx;
            }
        }

        // Process each plugin
        for name in plugin_names {
            let name_lower = name.to_lowercase();

            if let Some(&idx) = name_to_idx.get(&name_lower) {
                // Enable existing plugin
                if !lines[idx].starts_with('*') {
                    lines[idx] = format!("*{}", lines[idx]);
                    changed = true;
                }
            } else {
                // Add new plugin
                lines.insert(insert_index, format!("*{}", name));
                insert_index += 1;
                changed = true;
            }
        }

        if changed {
            let output = lines.join("\n");
            let output_with_newline = if output.ends_with('\n') {
                output
            } else {
                format!("{}\n", output)
            };

            fs::write(plugins_file, output_with_newline)
                .map_err(|e| format!("Failed to write plugins.txt: {}", e))?;
        }

        Ok(changed)
    }

    fn is_master_esm(plugin_lower: String) -> bool {
        matches!(
            plugin_lower.as_str(),
            "fallout4.esm"
                | "dlccoast.esm"
                | "dlcnukaworld.esm"
                | "dlcrobot.esm"
                | "dlcworkshop01.esm"
                | "dlcworkshop02.esm"
                | "dlcworkshop03.esm"
        )
    }

    /// Read modlist root from game paths
    pub fn find_modlist_root(game_root: Option<&Path>) -> Result<PathBuf, String> {
        // Try provided game root first
        if let Some(root) = game_root {
            if root.parent().map(|p| p.parent()).flatten().is_some() {
                return Ok(root.parent().unwrap().to_path_buf());
            }
        }

        // Fallback: use environment or current directory
        let home = dirs::home_dir().ok_or("Could not determine home directory".to_string())?;
        Ok(home.join("Fallout 4 Modlist"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_master_esm() {
        assert!(Mo2Integration::is_master_esm("fallout4.esm".to_string()));
        assert!(Mo2Integration::is_master_esm("dlcnukaworld.esm".to_string()));
        assert!(!Mo2Integration::is_master_esm("custom.esp".to_string()));
    }
}
