use crate::models::GameProfile;
use crate::services::PathDiscoveryService;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

pub struct ProfileManager;

impl ProfileManager {
    /// Returns `<mo2_root>/profiles/` — the canonical MO2 profiles directory.
    pub fn get_profiles_dir() -> Result<PathBuf, String> {
        let paths = PathDiscoveryService::discover()?;
        let mo2_root = paths
            .mo2_root
            .ok_or_else(|| "Mod Organizer 2 not detected".to_string())?;
        let dir = PathBuf::from(mo2_root).join("profiles");
        if !dir.exists() {
            return Err(format!(
                "MO2 profiles folder not found: {}",
                dir.display()
            ));
        }
        Ok(dir)
    }

    /// Active profile name from `ModOrganizer.ini` via path discovery.
    fn get_active_name() -> Option<String> {
        PathDiscoveryService::discover().ok()?.mo2_profile
    }

    // ── list ─────────────────────────────────────────────────────────────
    /// List every MO2 profile subfolder. Active profile sorts first.
    pub fn list_profiles() -> Result<Vec<GameProfile>, String> {
        let dir = Self::get_profiles_dir()?;
        let active = Self::get_active_name().unwrap_or_default();
        let mut profiles = Vec::new();

        for entry in fs::read_dir(&dir)
            .map_err(|e| format!("Cannot read profiles directory: {}", e))?
            .flatten()
        {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let name = match path.file_name().and_then(|n| n.to_str()) {
                Some(n) => n.to_string(),
                None => continue,
            };
            let (created_date, last_modified) = Self::folder_timestamps(&path);
            let enabled_mods = Self::read_enabled_mods(&path);
            profiles.push(GameProfile {
                is_active: name == active,
                name,
                enabled_mods,
                ini_overrides: HashMap::new(),
                mcm_preset: None,
                created_date,
                last_modified,
            });
        }

        // Active first, then alphabetical
        profiles.sort_by(|a, b| b.is_active.cmp(&a.is_active).then(a.name.cmp(&b.name)));
        Ok(profiles)
    }

    // ── create ───────────────────────────────────────────────────────────
    /// Create a new MO2 profile by copying the currently active profile
    /// folder. Falls back to a minimal empty profile if there is no active one.
    pub fn save_profile(profile: GameProfile) -> Result<(), String> {
        let name = profile.name.trim().to_string();
        Self::validate_name(&name)?;

        let dir = Self::get_profiles_dir()?;
        let dest = dir.join(&name);
        if dest.exists() {
            return Err(format!("Profile '{}' already exists", name));
        }

        if let Some(active) = Self::get_active_name() {
            let src = dir.join(&active);
            if src.is_dir() {
                return Self::copy_dir(&src, &dest);
            }
        }

        // Minimal fallback
        fs::create_dir_all(&dest)
            .map_err(|e| format!("Cannot create profile folder: {}", e))?;
        fs::write(dest.join("modlist.txt"), "")
            .map_err(|e| format!("Cannot create modlist.txt: {}", e))?;
        fs::write(dest.join("plugins.txt"), "")
            .map_err(|e| format!("Cannot create plugins.txt: {}", e))?;
        Ok(())
    }

    // ── load (info only) ─────────────────────────────────────────────────
    pub fn load_profile(name: &str) -> Result<GameProfile, String> {
        let dir = Self::get_profiles_dir()?;
        let profile_dir = dir.join(name);
        if !profile_dir.is_dir() {
            return Err(format!("Profile '{}' not found", name));
        }
        let active = Self::get_active_name().unwrap_or_default();
        let (created_date, last_modified) = Self::folder_timestamps(&profile_dir);
        Ok(GameProfile {
            is_active: name == active,
            name: name.to_string(),
            enabled_mods: Self::read_enabled_mods(&profile_dir),
            ini_overrides: HashMap::new(),
            mcm_preset: None,
            created_date,
            last_modified,
        })
    }

    // ── activate ─────────────────────────────────────────────────────────
    /// Set `name` as the active MO2 profile by rewriting `ModOrganizer.ini`.
    pub fn activate_profile(name: &str) -> Result<(), String> {
        let dir = Self::get_profiles_dir()?;
        if !dir.join(name).is_dir() {
            return Err(format!("Profile '{}' not found", name));
        }

        let mo2_root = dir
            .parent()
            .ok_or_else(|| "Cannot determine MO2 root".to_string())?;
        let ini_path = mo2_root.join("ModOrganizer.ini");
        if !ini_path.exists() {
            return Err("ModOrganizer.ini not found".to_string());
        }

        let content = fs::read_to_string(&ini_path)
            .map_err(|e| format!("Cannot read ModOrganizer.ini: {}", e))?;
        let new_content = Self::patch_selected_profile(&content, name);
        fs::write(&ini_path, new_content)
            .map_err(|e| format!("Cannot write ModOrganizer.ini: {}", e))
    }

    // ── delete ───────────────────────────────────────────────────────────
    /// Delete a profile folder. Refuses to delete the currently active profile.
    pub fn delete_profile(name: &str) -> Result<(), String> {
        if name.trim().is_empty() {
            return Err("Profile name cannot be empty".to_string());
        }
        if Self::get_active_name().as_deref() == Some(name) {
            return Err("Cannot delete the currently active profile — activate another profile first.".to_string());
        }

        let dir = Self::get_profiles_dir()?;
        let profile_dir = dir.join(name);
        if !profile_dir.is_dir() {
            return Err(format!("Profile '{}' not found", name));
        }

        fs::remove_dir_all(&profile_dir)
            .map_err(|e| format!("Cannot delete profile: {}", e))
    }

    // ── rename ───────────────────────────────────────────────────────────
    pub fn rename_profile(old_name: &str, new_name: &str) -> Result<(), String> {
        if old_name.eq_ignore_ascii_case("Fallen World") {
            return Err(
                "The 'Fallen World' profile is the source of truth and cannot be renamed.".to_string()
            );
        }
        let new_trimmed = new_name.trim().to_string();
        Self::validate_name(&new_trimmed)?;

        let dir = Self::get_profiles_dir()?;
        let old_dir = dir.join(old_name);
        let new_dir = dir.join(&new_trimmed);
        if !old_dir.is_dir() {
            return Err(format!("Profile '{}' not found", old_name));
        }
        if new_dir.exists() {
            return Err(format!("Profile '{}' already exists", new_trimmed));
        }

        fs::rename(&old_dir, &new_dir)
            .map_err(|e| format!("Cannot rename profile: {}", e))?;

        // Update active reference if this was the active profile
        if Self::get_active_name().as_deref() == Some(old_name) {
            let _ = Self::activate_profile(&new_trimmed);
        }
        Ok(())
    }

    // ── exists ───────────────────────────────────────────────────────────
    pub fn profile_exists(name: &str) -> Result<bool, String> {
        let dir = Self::get_profiles_dir()?;
        Ok(dir.join(name).is_dir())
    }

    // ── metadata ─────────────────────────────────────────────────────────
    pub fn get_profile_metadata(name: &str) -> Result<(String, String, String), String> {
        let p = Self::load_profile(name)?;
        Ok((p.created_date, p.last_modified, p.name))
    }

    // ── open folder ──────────────────────────────────────────────────────
    /// Open the MO2 profiles directory in Windows Explorer.
    pub fn open_profiles_folder() -> Result<(), String> {
        let dir = Self::get_profiles_dir()?;
        std::process::Command::new("explorer")
            .arg(dir.as_os_str())
            .spawn()
            .map_err(|e| format!("Cannot open folder: {}", e))?;
        Ok(())
    }

    // ── helpers ──────────────────────────────────────────────────────────

    fn validate_name(name: &str) -> Result<(), String> {
        if name.is_empty() {
            return Err("Profile name cannot be empty".to_string());
        }
        let bad: &[char] = &['/', '\\', ':', '*', '?', '"', '<', '>', '|'];
        if name.chars().any(|c| bad.contains(&c)) {
            return Err("Profile name contains invalid characters".to_string());
        }
        Ok(())
    }

    fn read_enabled_mods(profile_dir: &Path) -> Vec<String> {
        let content = match fs::read_to_string(profile_dir.join("modlist.txt")) {
            Ok(c) => c,
            Err(_) => return Vec::new(),
        };
        content
            .lines()
            .filter_map(|l| l.trim().strip_prefix('+').map(|n| n.trim().to_string()))
            .collect()
    }

    fn folder_timestamps(path: &Path) -> (String, String) {
        let Ok(meta) = fs::metadata(path) else {
            return (String::new(), String::new());
        };
        let to_rfc = |st: std::io::Result<std::time::SystemTime>| {
            st.ok()
                .map(|t| {
                    let dt: chrono::DateTime<chrono::Local> = t.into();
                    dt.to_rfc3339()
                })
                .unwrap_or_default()
        };
        (to_rfc(meta.created()), to_rfc(meta.modified()))
    }

    pub fn copy_dir(src: &Path, dest: &Path) -> Result<(), String> {
        fs::create_dir_all(dest)
            .map_err(|e| format!("Cannot create directory {}: {}", dest.display(), e))?;
        for entry in fs::read_dir(src)
            .map_err(|e| format!("Cannot read {}: {}", src.display(), e))?
            .flatten()
        {
            let s = entry.path();
            let d = dest.join(entry.file_name());
            if s.is_dir() {
                Self::copy_dir(&s, &d)?;
            } else {
                fs::copy(&s, &d)
                    .map_err(|e| format!("Cannot copy {}: {}", s.display(), e))?;
            }
        }
        Ok(())
    }

    /// Rewrite the `selected_profile` / `selectedProfile` key in the
    /// `[General]` section of `ModOrganizer.ini`, preserving Qt `@ByteArray()`
    /// encoding if that is what the original file used.
    fn patch_selected_profile(content: &str, profile_name: &str) -> String {
        let mut lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();
        let mut in_general = false;
        let mut patched = false;

        for line in &mut lines {
            let t = line.trim();
            if t.starts_with('[') {
                in_general = t.eq_ignore_ascii_case("[General]");
                continue;
            }
            if in_general {
                let is_key = t.starts_with("selected_profile=")
                    || t.starts_with("selectedProfile=");
                if is_key {
                    let key = if t.starts_with("selected_profile=") {
                        "selected_profile"
                    } else {
                        "selectedProfile"
                    };
                    let qt = t.contains("@ByteArray(");
                    *line = if qt {
                        format!("{}=@ByteArray({})", key, profile_name)
                    } else {
                        format!("{}={}", key, profile_name)
                    };
                    patched = true;
                }
            }
        }

        if !patched {
            // Append under [General], creating the section if absent
            let general_pos = lines
                .iter()
                .position(|l| l.trim().eq_ignore_ascii_case("[General]"));
            let insert_at = general_pos.map(|i| i + 1).unwrap_or(lines.len());
            if general_pos.is_none() {
                lines.push("[General]".to_string());
            }
            lines.insert(
                insert_at,
                format!("selected_profile=@ByteArray({})", profile_name),
            );
        }

        let mut out = lines.join("\n");
        if content.ends_with('\n') {
            out.push('\n');
        }
        out
    }
}
