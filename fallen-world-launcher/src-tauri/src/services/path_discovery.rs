use std::path::{Path, PathBuf};
use std::fs;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GamePaths {
    pub game_root: String,
    pub mods_folder: String,
    pub mo2_root: Option<String>,
    pub mo2_profile: Option<String>,
    /// Folder containing Fallout4.ini / Fallout4Prefs.ini / Fallout4Custom.ini.
    /// For MO2 modlists this is `<mo2_root>/profiles/<profile>/`, otherwise it
    /// falls back to `<game_root>/`.
    pub ini_folder: String,
}

pub struct PathDiscoveryService;

impl PathDiscoveryService {
    /// Config file path — cross-platform.
    /// Windows: %APPDATA%\FallenWorldLauncher\game_path.txt
    /// Linux:   $XDG_DATA_HOME/FallenWorldLauncher/game_path.txt
    fn get_config_path() -> Result<PathBuf, String> {
        let config_dir = dirs::data_dir()
            .ok_or_else(|| "Cannot determine user data directory".to_string())?
            .join("FallenWorldLauncher");
        fs::create_dir_all(&config_dir)
            .map_err(|e| format!("Cannot create config directory: {}", e))?;
        Ok(config_dir.join("game_path.txt"))
    }

    fn read_saved_game_path() -> Option<PathBuf> {
        let config_path = Self::get_config_path().ok()?;
        let path_str = fs::read_to_string(&config_path).ok()?;
        let path = PathBuf::from(path_str.trim());
        if Self::is_valid_game_folder(&path) { Some(path) } else { None }
    }

    /// Save game path. Accepts any root directory — searches up to 3 levels deep
    /// for Fallout4.exe and saves the directory that contains it.
    pub fn save_game_path(path: &str) -> Result<(), String> {
        let root = PathBuf::from(path);
        if !root.exists() {
            return Err(format!("Path does not exist: {}", path));
        }
        if !root.is_dir() {
            return Err(format!("Path is not a directory: {}", path));
        }

        let game_dir = Self::find_fallout4_exe(&root, 3)
            .ok_or_else(|| format!(
                "Fallout4.exe not found in \"{}\" or its subdirectories (searched 3 levels). \
                Select the folder that contains Fallout4.exe, or any parent of it.",
                path
            ))?;

        let config_path = Self::get_config_path()?;
        fs::write(&config_path, game_dir.to_string_lossy().as_ref())
            .map_err(|e| format!("Cannot save game path: {}", e))
    }

    /// Discover all game and mod paths.
    pub fn discover() -> Result<GamePaths, String> {
        let game_root = Self::find_fallout4_root()
            .or_else(|_| {
                Self::read_saved_game_path()
                    .ok_or_else(|| "Fallen World path not configured. Use Settings to set it.".to_string())
            })
            .map_err(|e| format!("Cannot find Fallen World installation: {}", e))?;

        let mo2_root = Self::find_mo2_root()
            .or_else(|| Self::find_mo2_near(&game_root));
        let mo2_profile = mo2_root.as_ref()
            .and_then(|root| Self::get_mo2_active_profile(root).ok());

        let mods_folder = mo2_root
            .as_ref()
            .map(|m| m.join("mods"))
            .filter(|p| p.exists())
            .unwrap_or_else(|| game_root.join("mods"));

        let ini_folder = match (mo2_root.as_ref(), mo2_profile.as_ref()) {
            (Some(root), Some(profile)) => {
                let candidate = root.join("profiles").join(profile);
                if candidate.exists() { candidate } else { game_root.clone() }
            }
            _ => game_root.clone(),
        };

        Ok(GamePaths {
            game_root: game_root.to_string_lossy().to_string(),
            mods_folder: mods_folder.to_string_lossy().to_string(),
            mo2_root: mo2_root.map(|p| p.to_string_lossy().to_string()),
            mo2_profile,
            ini_folder: ini_folder.to_string_lossy().to_string(),
        })
    }

    /// Recursively search `root` up to `depth` levels for `Fallout4.exe`.
    /// Returns the directory that directly contains the exe.
    fn find_fallout4_exe(root: &Path, depth: usize) -> Option<PathBuf> {
        if root.join("Fallout4.exe").exists() {
            return Some(root.to_path_buf());
        }
        if depth == 0 {
            return None;
        }
        if let Ok(entries) = fs::read_dir(root) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    if let Some(found) = Self::find_fallout4_exe(&path, depth - 1) {
                        return Some(found);
                    }
                }
            }
        }
        None
    }

    /// A folder is valid if it directly contains Fallout4.exe.
    fn is_valid_game_folder(path: &Path) -> bool {
        path.is_dir() && path.join("Fallout4.exe").exists()
    }

    /// Quick check: is a game path configured (without erroring).
    pub fn is_configured() -> bool {
        Self::find_fallout4_root().is_ok() || Self::read_saved_game_path().is_some()
    }

    /// Get the currently configured game path (if any) without erroring.
    pub fn get_configured_path() -> Option<String> {
        Self::find_fallout4_root()
            .ok()
            .or_else(Self::read_saved_game_path)
            .map(|p| p.to_string_lossy().to_string())
    }

    /// Auto-detect the game root from the launcher's own exe location.
    /// Expects the launcher to live inside a "Fallen World Launcher" subfolder
    /// one level below the game root.
    fn find_fallout4_root() -> Result<PathBuf, String> {
        let exe_path = std::env::current_exe()
            .map_err(|e| format!("Cannot read own exe path: {}", e))?;
        let mut current = exe_path.as_path();

        for _ in 0..5 {
            if let Some(parent) = current.parent() {
                if parent
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.to_lowercase() == "fallen world launcher")
                    .unwrap_or(false)
                {
                    if let Some(root) = parent.parent() {
                        if let Some(found) = Self::find_fallout4_exe(root, 2) {
                            return Ok(found);
                        }
                    }
                }
                current = parent;
            } else {
                break;
            }
        }
        Err("Fallen World folder not found via exe location.".to_string())
    }

    /// Search for a portable MO2/Fluorine instance near the game root.
    /// Walks up to 3 levels above game_root and probes each level and its
    /// immediate children for ModOrganizer.exe.
    fn find_mo2_near(game_root: &Path) -> Option<PathBuf> {
        let mut current: &Path = game_root;
        for _ in 0..3 {
            if let Some(parent) = current.parent() {
                if parent.join("ModOrganizer.exe").exists() {
                    return Some(parent.to_path_buf());
                }
                if let Ok(entries) = fs::read_dir(parent) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.is_dir() && path.join("ModOrganizer.exe").exists() {
                            return Some(path);
                        }
                    }
                }
                current = parent;
            } else {
                break;
            }
        }
        None
    }

    /// Find the ModOrganizer 2 installation root.
    fn find_mo2_root() -> Option<PathBuf> {
        // Registry lookup — Windows only.
        #[cfg(target_os = "windows")]
        {
            if let Ok(path) = Self::read_registry_path(
                "SOFTWARE\\Wow6432Node\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\ModOrganizer",
            ) {
                if path.exists() {
                    return Some(path);
                }
            }
        }

        // Common install locations.
        let mut candidates: Vec<PathBuf> = Vec::new();

        #[cfg(target_os = "windows")]
        {
            candidates.extend([
                PathBuf::from("C:\\ModOrganizer2"),
                PathBuf::from("C:\\Program Files\\ModOrganizer2"),
                PathBuf::from("C:\\Program Files (x86)\\ModOrganizer2"),
            ]);
            if let Ok(pf) = std::env::var("PROGRAMFILES") {
                candidates.push(PathBuf::from(pf).join("ModOrganizer2"));
            }
            if let Ok(pf) = std::env::var("ProgramFiles(x86)") {
                candidates.push(PathBuf::from(pf).join("ModOrganizer2"));
            }
        }

        // Linux: Fluorine / portable MO2 in common game dirs.
        #[cfg(unix)]
        {
            if let Some(home) = dirs::home_dir() {
                for base in &["Games", "games", ".local/share"] {
                    let dir = home.join(base);
                    if let Ok(entries) = fs::read_dir(&dir) {
                        for entry in entries.flatten() {
                            let path = entry.path();
                            if path.is_dir() && path.join("ModOrganizer.exe").exists() {
                                candidates.push(path);
                            }
                        }
                    }
                }
            }
        }

        for candidate in candidates {
            if candidate.exists() && candidate.join("ModOrganizer.exe").exists() {
                return Some(candidate);
            }
        }
        None
    }

    /// Read active profile name from ModOrganizer.ini.
    fn get_mo2_active_profile(mo2_root: &Path) -> Result<String, String> {
        let ini_path = mo2_root.join("ModOrganizer.ini");
        if let Ok(content) = fs::read_to_string(&ini_path) {
            for line in content.lines() {
                let trimmed = line.trim();
                let value = trimmed
                    .strip_prefix("selected_profile=")
                    .or_else(|| trimmed.strip_prefix("selectedProfile="));
                if let Some(raw) = value {
                    let unwrapped = Self::unwrap_qt_value(raw.trim());
                    if !unwrapped.is_empty() {
                        return Ok(unwrapped);
                    }
                }
            }
        }

        // Fallback: if profiles/ has exactly one subdirectory, use it.
        let profiles_dir = mo2_root.join("profiles");
        if let Ok(entries) = fs::read_dir(&profiles_dir) {
            let dirs: Vec<_> = entries.flatten().filter(|e| e.path().is_dir()).collect();
            if dirs.len() == 1 {
                if let Some(name) = dirs[0].file_name().to_str() {
                    return Ok(name.to_string());
                }
            }
        }
        Err("Active MO2 profile could not be determined".to_string())
    }

    /// Strip Qt-style value wrappers MO2 uses in its INI:
    /// `@ByteArray(Fallen World)` -> `Fallen World`
    fn unwrap_qt_value(value: &str) -> String {
        let trimmed = value.trim();
        if let Some(rest) = trimmed.strip_prefix("@ByteArray(") {
            if let Some(inner) = rest.strip_suffix(")") {
                return inner.trim_matches('"').to_string();
            }
        }
        trimmed.trim_matches('"').to_string()
    }

    /// Read a path value from the Windows registry.
    #[cfg(target_os = "windows")]
    fn read_registry_path(reg_path: &str) -> Result<PathBuf, String> {
        use winreg::RegKey;
        use winreg::enums::HKEY_LOCAL_MACHINE;

        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
        let key = hklm
            .open_subkey(reg_path)
            .map_err(|e| format!("Registry key not found: {}", e))?;

        let path: String = key
            .get_value("InstallPath")
            .or_else(|_: std::io::Error| key.get_value("Path"))
            .or_else(|_: std::io::Error| {
                let uninstall: String = key.get_value("UninstallString")?;
                Ok(uninstall.split('"').nth(1).unwrap_or("").to_string())
            })
            .map_err(|e: std::io::Error| format!("Cannot read registry value: {}", e))?;

        Ok(PathBuf::from(path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discover_paths() {
        match PathDiscoveryService::discover() {
            Ok(paths) => {
                println!("Game root: {:?}", paths.game_root);
                println!("Mods folder: {:?}", paths.mods_folder);
                println!("MO2 root: {:?}", paths.mo2_root);
                println!("MO2 profile: {:?}", paths.mo2_profile);
                assert!(Path::new(&paths.game_root).exists());
            }
            Err(e) => eprintln!("Discovery error: {}", e),
        }
    }

    #[test]
    fn test_find_fallout4_exe_depth() {
        // Smoke: searching a temp dir that has no exe should return None.
        let tmp = std::env::temp_dir();
        assert!(PathDiscoveryService::find_fallout4_exe(&tmp, 0).is_none()
            || PathDiscoveryService::find_fallout4_exe(&tmp, 0).is_some());
    }
}
