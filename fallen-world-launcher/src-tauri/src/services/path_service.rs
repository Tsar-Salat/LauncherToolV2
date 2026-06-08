use crate::models::GamePaths;
use std::path::{Path, PathBuf};

#[cfg(target_os = "windows")]
use winreg::RegKey;

pub struct PathService;

impl PathService {
    pub fn discover_game_paths() -> Option<GamePaths> {
        #[cfg(target_os = "windows")]
        return Self::discover_windows();

        #[cfg(target_os = "linux")]
        return Self::discover_linux();

        #[cfg(target_os = "macos")]
        return Self::discover_macos();

        #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
        None
    }

    #[cfg(target_os = "windows")]
    fn discover_windows() -> Option<GamePaths> {
        let hklm = RegKey::predef(winreg::enums::HKEY_LOCAL_MACHINE);
        let path = hklm
            .open_subkey(r"SOFTWARE\Bethesda Softworks\Fallout4")
            .ok()?;
        let game_root: String = path.get_value("Installed Path").ok()?;

        let home_dir = dirs::home_dir()?;

        let mo2_paths = vec![
            PathBuf::from(&game_root)
                .parent()
                .map(|p| p.join("ModOrganizer2")),
            Some(home_dir.join("ModOrganizer2")),
            Some(PathBuf::from("C:\\Program Files\\ModOrganizer2")),
        ];

        let mods_folder = mo2_paths
            .into_iter()
            .filter_map(|p| p)
            .find(|p| p.exists())?
            .join("mods")
            .to_string_lossy()
            .to_string();

        Some(GamePaths {
            game_root,
            mods_folder,
            ini_file: format!(
                "{}\\Documents\\My Games\\Fallout4\\Fallout4.ini",
                home_dir.display()
            ),
            ini_prefs_file: format!(
                "{}\\Documents\\My Games\\Fallout4\\Fallout4Prefs.ini",
                home_dir.display()
            ),
            plugins_file: format!(
                "{}\\AppData\\Local\\Fallout4\\plugins.txt",
                home_dir.display()
            ),
            load_order_file: format!(
                "{}\\AppData\\Local\\Fallout4\\loadorder.txt",
                home_dir.display()
            ),
        })
    }

    #[cfg(target_os = "linux")]
    fn discover_linux() -> Option<GamePaths> {
        // Steam Proton paths
        let home = dirs::home_dir()?;
        let steam_paths = vec![
            home.join(".steam/steamapps/compatdata/287860/pfx/drive_c/Users/steamuser/My Documents/My Games/Fallout4"),
            home.join(".var/app/com.github.Proton-plus-gpl/data/Steam/steamapps/compatdata/287860/pfx/drive_c/Users/steamuser/My Documents/My Games/Fallout4"),
        ];

        let game_root = steam_paths.iter().find(|p| p.exists())?.to_string_lossy().to_string();

        let mods_folder = home
            .join("ModOrganizer2/mods")
            .to_string_lossy()
            .to_string();

        Some(GamePaths {
            game_root,
            mods_folder,
            ini_file: home.join(".config/Fallout4/Fallout4.ini").to_string_lossy().to_string(),
            ini_prefs_file: home.join(".config/Fallout4/Fallout4Prefs.ini").to_string_lossy().to_string(),
            plugins_file: home.join(".local/share/Fallout4/plugins.txt").to_string_lossy().to_string(),
            load_order_file: home.join(".local/share/Fallout4/loadorder.txt").to_string_lossy().to_string(),
        })
    }

    #[cfg(target_os = "macos")]
    fn discover_macos() -> Option<GamePaths> {
        // Similar to Linux but with macOS paths
        None // Fallout 4 doesn't officially support macOS
    }

    pub fn validate_paths(paths: &GamePaths) -> bool {
        let game_root_valid = !paths.game_root.is_empty() && Path::new(&paths.game_root).exists();
        let mods_valid = !paths.mods_folder.is_empty() && Path::new(&paths.mods_folder).exists();
        let ini_valid = !paths.ini_file.is_empty() && Path::new(&paths.ini_file).exists();
        let ini_prefs_valid =
            !paths.ini_prefs_file.is_empty() && Path::new(&paths.ini_prefs_file).exists();

        game_root_valid && mods_valid && ini_valid && ini_prefs_valid
    }
}
