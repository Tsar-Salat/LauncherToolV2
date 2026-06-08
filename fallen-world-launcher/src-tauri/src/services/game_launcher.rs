use crate::models::OperationResult;
use crate::services::{SystemInfoService, PathDiscoveryService};
use std::path::PathBuf;

pub struct GameLauncher;

impl GameLauncher {
    pub fn find_f4se_loader(game_root: &str) -> Option<PathBuf> {
        Self::find_f4se_loader_in(game_root, None)
    }

    /// Find f4se_loader.exe across the typical install layouts:
    /// 1. <game_root>/f4se_loader.exe  (vanilla install)
    /// 2. <game_root>/Fallout4/f4se_loader.exe  (nested Fallen World layout)
    /// 3. <mo2_root>/f4se_loader.exe  (some MO2 portable instances)
    /// 4. <mo2_root>/mods/*/f4se_loader.exe  (F4SE installed as an MO2 mod)
    pub fn find_f4se_loader_in(game_root: &str, mo2_root: Option<&str>) -> Option<PathBuf> {
        #[cfg(target_os = "windows")]
        {
            use std::fs;

            let game_path = PathBuf::from(game_root);
            for candidate in [
                game_path.join("f4se_loader.exe"),
                game_path.join("Fallout4").join("f4se_loader.exe"),
            ] {
                if candidate.exists() {
                    return Some(candidate);
                }
            }

            if let Some(mo2) = mo2_root {
                let mo2_path = PathBuf::from(mo2);
                let direct = mo2_path.join("f4se_loader.exe");
                if direct.exists() {
                    return Some(direct);
                }
                // Scan mods/ subfolders one level deep — F4SE is commonly
                // installed as a top-level mod folder by modlist installers.
                let mods_dir = mo2_path.join("mods");
                if let Ok(entries) = fs::read_dir(&mods_dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.is_dir() {
                            let candidate = path.join("f4se_loader.exe");
                            if candidate.exists() {
                                return Some(candidate);
                            }
                        }
                    }
                }
            }
        }
        let _ = mo2_root;
        None
    }

    /// Find the title of the MO2 custom executable that launches F4SE, by parsing
    /// the `[customExecutables]` section of `<mo2_root>/ModOrganizer.ini`.
    ///
    /// MO2 stores them as indexed keys:
    /// ```ini
    /// [customExecutables]
    /// 1\title=F4SE
    /// 1\binary=D:/Games/.../f4se_loader.exe
    /// ```
    /// Prefers the entry whose binary is `f4se_loader.exe`; falls back to any
    /// entry whose title contains "f4se". Returns the title for `moshortcut://`.
    fn find_mo2_f4se_executable(mo2_root: &str) -> Option<String> {
        use std::collections::HashMap;
        use std::fs;

        let ini = PathBuf::from(mo2_root).join("ModOrganizer.ini");
        let content = fs::read_to_string(&ini).ok()?;

        let mut titles: HashMap<String, String> = HashMap::new();
        let mut binaries: HashMap<String, String> = HashMap::new();
        let mut in_section = false;

        for line in content.lines() {
            let t = line.trim();
            if t.starts_with('[') && t.ends_with(']') {
                in_section = t.eq_ignore_ascii_case("[customExecutables]");
                continue;
            }
            if !in_section {
                continue;
            }
            let Some((key, val)) = t.split_once('=') else { continue };
            let Some((idx, field)) = key.trim().split_once('\\') else { continue };
            let value = Self::unwrap_ini_value(val.trim());
            match field.to_ascii_lowercase().as_str() {
                "title" => { titles.insert(idx.to_string(), value); }
                "binary" => { binaries.insert(idx.to_string(), value); }
                _ => {}
            }
        }

        // Preferred: the entry whose binary is f4se_loader.exe.
        for (idx, bin) in &binaries {
            if bin.to_ascii_lowercase().replace('\\', "/").ends_with("f4se_loader.exe") {
                if let Some(title) = titles.get(idx).filter(|t| !t.is_empty()) {
                    return Some(title.clone());
                }
            }
        }
        // Fallback: any executable whose title looks like F4SE.
        titles.into_values().find(|t| t.to_ascii_lowercase().contains("f4se"))
    }

    /// Strip Qt `@ByteArray(...)` wrappers and surrounding quotes from an MO2
    /// INI value.
    fn unwrap_ini_value(value: &str) -> String {
        let t = value.trim();
        if let Some(rest) = t.strip_prefix("@ByteArray(") {
            if let Some(inner) = rest.strip_suffix(')') {
                return inner.trim_matches('"').to_string();
            }
        }
        t.trim_matches('"').to_string()
    }

    /// Launch Mod Organizer 2. Resolves ModOrganizer.exe from the detected MO2
    /// root (registry / common locations / portable instance near the game).
    pub fn launch_mo2() -> OperationResult {
        #[cfg(target_os = "windows")]
        {
            let mo2_root = PathDiscoveryService::discover()
                .ok()
                .and_then(|p| p.mo2_root);
            let Some(root) = mo2_root else {
                return OperationResult {
                    success: false,
                    message: "Mod Organizer 2 not found. Set your game path in Settings.".to_string(),
                };
            };
            let exe = PathBuf::from(&root).join("ModOrganizer.exe");
            if !exe.exists() {
                return OperationResult {
                    success: false,
                    message: format!("ModOrganizer.exe not found at {}", root),
                };
            }
            match std::process::Command::new(&exe).current_dir(&root).spawn() {
                Ok(_) => OperationResult { success: true, message: "Mod Organizer 2 launched".to_string() },
                Err(e) => OperationResult { success: false, message: format!("Failed to launch MO2: {}", e) },
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            OperationResult { success: false, message: "MO2 launch is Windows-only".to_string() }
        }
    }

    pub async fn launch_game(game_root: &str) -> OperationResult {
        let blocking = SystemInfoService::check_blocking_processes();
        if !blocking.is_empty() {
            let process_list: Vec<String> = blocking
                .iter()
                .map(|p| p.name.clone())
                .collect();
            return OperationResult {
                success: false,
                message: format!(
                    "Cannot launch: {} running. Please close these processes first.",
                    process_list.join(", ")
                ),
            };
        }

        #[cfg(target_os = "windows")]
        {
            let paths = PathDiscoveryService::discover().ok();
            let mo2_root = paths.as_ref().and_then(|p| p.mo2_root.as_deref().map(|s| s.to_string()));

            // When MO2 is present, always launch through MO2's VFS so all mods
            // are active. Direct F4SE launch bypasses the virtual file system and
            // the game runs without any mods from the modlist.
            if let Some(ref root) = mo2_root {
                let mo2_exe = PathBuf::from(root).join("ModOrganizer.exe");
                if mo2_exe.exists() {
                    let profile = paths.as_ref().and_then(|p| p.mo2_profile.clone());

                    let mut cmd = std::process::Command::new(&mo2_exe);
                    cmd.current_dir(root);
                    // Select the modlist's profile so the correct load order / INIs
                    // are virtualised.
                    if let Some(ref prof) = profile {
                        cmd.arg("-p").arg(prof);
                    }

                    // Preferred: launch the executable the modlist author configured
                    // in MO2 (correct working dir + arguments) via the moshortcut URI.
                    // Fallback: hand MO2 the plain absolute path to f4se_loader.exe so
                    // it runs the binary through the VFS. NEVER prefix with "run:" —
                    // MO2 treats that as a relative binary path and mangles it.
                    let launch_desc;
                    if let Some(title) = Self::find_mo2_f4se_executable(root) {
                        cmd.arg(format!("moshortcut://:{}", title));
                        launch_desc = format!("MO2 shortcut '{}'", title);
                    } else if let Some(f4se) = Self::find_f4se_loader_in(game_root, Some(root)) {
                        cmd.arg(f4se.to_string_lossy().to_string());
                        launch_desc = "f4se_loader.exe via MO2 VFS".to_string();
                    } else {
                        return OperationResult {
                            success: false,
                            message: "F4SE not found. Configure an 'F4SE' executable in MO2, \
                                      or verify f4se_loader.exe is installed.".to_string(),
                        };
                    }

                    return match cmd.spawn() {
                        Ok(_) => OperationResult {
                            success: true,
                            message: format!("Game launched through MO2 ({})", launch_desc),
                        },
                        Err(e) => OperationResult {
                            success: false,
                            message: format!("Failed to launch through MO2: {}", e),
                        },
                    };
                }
            }

            // MO2 not found — direct F4SE launch (no mod virtualisation)
            if let Some(f4se_path) = Self::find_f4se_loader_in(game_root, mo2_root.as_deref()) {
                return match std::process::Command::new(&f4se_path)
                    .current_dir(game_root)
                    .spawn()
                {
                    Ok(_) => OperationResult {
                        success: true,
                        message: "Game launched via F4SE (MO2 not found — mods may not be active)".to_string(),
                    },
                    Err(e) => OperationResult {
                        success: false,
                        message: format!("Failed to launch F4SE: {}", e),
                    },
                };
            }

            let fallout4_exe = format!("{}\\Fallout4.exe", game_root);
            match std::process::Command::new(&fallout4_exe)
                .current_dir(game_root)
                .spawn()
            {
                Ok(_) => OperationResult {
                    success: true,
                    message: "Game launched (F4SE not found, using Fallout4.exe)".to_string(),
                },
                Err(e) => OperationResult {
                    success: false,
                    message: format!("Failed to launch game: {}", e),
                },
            }
        }

        #[cfg(target_os = "linux")]
        {
            match std::process::Command::new("steam")
                .arg("steam://run/287860")
                .spawn()
            {
                Ok(_) => OperationResult {
                    success: true,
                    message: "Game launched via Steam".to_string(),
                },
                Err(e) => OperationResult {
                    success: false,
                    message: format!("Failed to launch game: {}", e),
                },
            }
        }

        #[cfg(not(any(target_os = "windows", target_os = "linux")))]
        {
            OperationResult {
                success: false,
                message: "Game launching not supported on this platform".to_string(),
            }
        }
    }
}
