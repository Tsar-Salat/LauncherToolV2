use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mod {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub enabled: bool,
    pub load_order: i32,
    pub dependencies: Vec<String>,
    pub conflicts: Vec<String>,
    pub last_updated: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameProfile {
    pub name: String,
    #[serde(default)]
    pub is_active: bool,
    pub enabled_mods: Vec<String>,
    pub ini_overrides: HashMap<String, String>,
    pub mcm_preset: Option<String>,
    pub created_date: String,
    pub last_modified: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IniConfig {
    pub sections: HashMap<String, HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModUpdate {
    pub mod_id: String,
    pub current_version: String,
    pub available_version: String,
    pub changelog: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadOrder {
    pub order: Vec<String>,
    pub mod_info: HashMap<String, ModInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModInfo {
    pub name: String,
    pub version: String,
    pub has_esm: bool,
    pub master_files: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preset {
    pub name: String,
    pub preset_type: String, // "ENB" or "ReShade"
    pub preview_image: Option<Vec<u8>>,
    pub installed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationResult {
    pub success: bool,
    pub message: String,
}
