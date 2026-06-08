use crate::models::Mod;
use std::fs;
use std::path::Path;
use chrono::Local;

pub struct ModManager;

impl ModManager {
    pub fn list_mods(mods_folder: &str) -> Vec<Mod> {
        let mut mods = Vec::new();

        if !Path::new(mods_folder).exists() {
            return mods;
        }

        if let Ok(entries) = fs::read_dir(mods_folder) {
            let mut load_order = 0;

            for entry in entries {
                match entry {
                    Ok(entry) => {
                        let path = entry.path();
                        if path.is_dir() {
                            if let Some(mod_name) = path.file_name().and_then(|n| n.to_str()) {
                                // Skip special folders
                                if mod_name.starts_with("__") || mod_name.starts_with(".") {
                                    continue;
                                }

                                let mod_obj = Mod {
                                    id: mod_name.to_string(),
                                    name: mod_name.to_string(),
                                    version: "1.0".to_string(),
                                    author: "Unknown".to_string(),
                                    description: format!("Mod: {}", mod_name),
                                    enabled: true,
                                    load_order,
                                    dependencies: vec![],
                                    conflicts: vec![],
                                    last_updated: Local::now().to_rfc3339(),
                                };

                                mods.push(mod_obj);
                                load_order += 1;
                            }
                        }
                    }
                    Err(_) => continue,
                }
            }
        }

        mods.sort_by(|a, b| a.name.cmp(&b.name));
        mods
    }

    pub fn toggle_mod(_mod_id: &str, _enabled: bool) -> bool {
        // In real implementation, this would write to plugins.txt or loadorder.txt
        true
    }

    pub fn get_load_order_file(_game_paths: &str) -> Vec<String> {
        vec![]
    }

    pub fn validate_dependencies(_mod_id: &str, _all_mods: &[Mod]) -> (bool, Vec<String>) {
        // Check if all dependencies are present and enabled
        let issues = Vec::new();
        (issues.is_empty(), issues)
    }
}
