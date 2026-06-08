//! MO2-backed mod list management.
//!
//! Reads/writes the active profile's `modlist.txt` directly so the launcher
//! stays in lockstep with Mod Organizer 2. modlist.txt format:
//!   `# comment` · `+Mod` (enabled) · `-Mod` (disabled) · `*DLC/CC` (managed)
//! Bottom of file = highest priority (MO2 stores lower-priority mods first;
//! the MO2 UI reverses this so highest-priority appears at the top).
//! We treat `+`/`-` entries as mods, skip `*` and comments, and flag
//! `_separator` rows.
//!
//! "Base" mods (shipped with the modlist) are locked in the UI; mods the user
//! adds through the launcher are tracked in `user_mods.json` and appended at
//! the end of the file (highest priority).

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

use super::file_service::FileService;
use super::system_info::SystemInfoService;
use super::PathDiscoveryService;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModEntry {
    pub name: String,
    pub enabled: bool,
    pub is_user: bool,
    pub is_separator: bool,
}

pub struct ModsManager;

impl ModsManager {
    fn modlist_path() -> Result<PathBuf, String> {
        let paths = PathDiscoveryService::discover()?;
        let root = paths.mo2_root.ok_or_else(|| "Mod Organizer 2 not detected".to_string())?;
        let profile = paths.mo2_profile.ok_or_else(|| "No active MO2 profile".to_string())?;
        Ok(PathBuf::from(root).join("profiles").join(profile).join("modlist.txt"))
    }

    fn mods_folder() -> Result<String, String> {
        Ok(PathDiscoveryService::discover()?.mods_folder)
    }

    /// MO2 caches the mod list in memory and rewrites `modlist.txt` when it
    /// exits, silently clobbering any edits made while it was open. Refuse to
    /// write in that case so changes actually stick. (This is the usual cause of
    /// "the launcher doesn't write to MO2".)
    fn ensure_mo2_closed() -> Result<(), String> {
        if SystemInfoService::process_status().mo2_running {
            return Err(
                "Mod Organizer 2 is running. Close MO2 first — it rewrites the mod list when it \
                 exits, which would undo any changes made here."
                    .to_string(),
            );
        }
        Ok(())
    }

    // ── user-mod registry (appdata) ──────────────────────────────────────
    fn user_mods_path() -> Option<PathBuf> {
        let appdata = std::env::var("APPDATA").ok()?;
        let dir = PathBuf::from(appdata).join("FallenWorldLauncher");
        let _ = fs::create_dir_all(&dir);
        Some(dir.join("user_mods.json"))
    }
    fn user_mods() -> Vec<String> {
        Self::user_mods_path()
            .and_then(|p| fs::read_to_string(p).ok())
            .and_then(|c| serde_json::from_str(&c).ok())
            .unwrap_or_default()
    }
    fn save_user_mods(list: &[String]) {
        if let Some(p) = Self::user_mods_path() {
            let _ = fs::write(p, serde_json::to_string_pretty(list).unwrap_or_default());
        }
    }

    // ── read ─────────────────────────────────────────────────────────────
    pub fn list() -> Result<Vec<ModEntry>, String> {
        let path = Self::modlist_path()?;
        let content = fs::read_to_string(&path)
            .map_err(|e| format!("Cannot read modlist.txt: {}", e))?;
        let user = Self::user_mods();

        let mut entries = Vec::new();
        for line in content.lines() {
            let t = line.trim();
            if t.is_empty() || t.starts_with('#') || t.starts_with('*') {
                continue;
            }
            let (enabled, name) = match t.strip_prefix('+') {
                Some(rest) => (true, rest.trim()),
                None => match t.strip_prefix('-') {
                    Some(rest) => (false, rest.trim()),
                    None => continue,
                },
            };
            if name.is_empty() {
                continue;
            }
            entries.push(ModEntry {
                name: name.to_string(),
                enabled,
                is_user: user.iter().any(|u| u == name),
                is_separator: name.ends_with("_separator"),
            });
        }
        Ok(entries)
    }

    // ── write: toggle ────────────────────────────────────────────────────
    /// Flip a mod's enabled state by rewriting only its `+`/`-` prefix.
    pub fn set_enabled(name: &str, enabled: bool) -> Result<(), String> {
        Self::ensure_mo2_closed()?;
        let path = Self::modlist_path()?;
        let content = fs::read_to_string(&path)
            .map_err(|e| format!("Cannot read modlist.txt: {}", e))?;
        let mut found = false;
        let new: Vec<String> = content
            .lines()
            .map(|line| {
                let t = line.trim();
                if (t.starts_with('+') || t.starts_with('-')) && t[1..].trim() == name {
                    found = true;
                    format!("{}{}", if enabled { '+' } else { '-' }, name)
                } else {
                    line.to_string()
                }
            })
            .collect();
        if !found {
            return Err(format!("Mod not found in modlist: {}", name));
        }
        Self::write_lines(&path, &new)
    }

    // ── write: add user mod ──────────────────────────────────────────────
    /// Install a mod from either a folder or a `.zip` archive into MO2's mods
    /// dir, enable it at the top (highest priority), and record it as a user
    /// mod. Returns the mod name.
    pub fn add_user_mod(source: &str) -> Result<String, String> {
        Self::ensure_mo2_closed()?;
        let src = PathBuf::from(source);

        // Resolve the folder we will copy into MO2 and the mod's name. Zips are
        // extracted to a temp dir first; `temp_cleanup` is removed afterwards.
        let mut temp_cleanup: Option<PathBuf> = None;
        let (mod_root, name) = if Self::is_zip(&src) {
            let (temp, root, name) = Self::extract_zip(&src)?;
            temp_cleanup = Some(temp);
            (root, name)
        } else if src.is_dir() {
            let name = src
                .file_name()
                .and_then(|n| n.to_str())
                .ok_or_else(|| "Invalid folder name".to_string())?
                .to_string();
            (src.clone(), name)
        } else {
            return Err("Select a mod folder or a .zip archive.".to_string());
        };

        let result = Self::install_resolved_mod(&mod_root, &name);
        if let Some(tmp) = temp_cleanup {
            let _ = fs::remove_dir_all(&tmp);
        }
        result
    }

    fn is_zip(path: &Path) -> bool {
        path.is_file()
            && path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.eq_ignore_ascii_case("zip"))
                .unwrap_or(false)
    }

    /// Extract a zip to a fresh temp dir. Returns `(temp_dir_to_clean, mod_root,
    /// name)`. If the archive has a single top-level folder we treat that folder
    /// as the mod root (and use its name); otherwise the temp dir itself is the
    /// root and the zip's file stem is the name.
    fn extract_zip(zip_path: &Path) -> Result<(PathBuf, PathBuf, String), String> {
        let file = fs::File::open(zip_path).map_err(|e| format!("Cannot open zip: {}", e))?;
        let mut archive =
            zip::ZipArchive::new(file).map_err(|e| format!("Not a valid zip archive: {}", e))?;

        let temp = std::env::temp_dir().join(format!("fwl_addmod_{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&temp).map_err(|e| format!("Cannot create temp dir: {}", e))?;
        archive
            .extract(&temp)
            .map_err(|e| format!("Failed to extract zip: {}", e))?;

        let zip_stem = zip_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Added Mod")
            .to_string();

        let entries: Vec<_> = fs::read_dir(&temp)
            .map_err(|e| format!("Cannot read extracted files: {}", e))?
            .flatten()
            .collect();
        if entries.len() == 1 && entries[0].path().is_dir() {
            let root = entries[0].path();
            let name = root
                .file_name()
                .and_then(|n| n.to_str())
                .map(|s| s.to_string())
                .unwrap_or(zip_stem);
            Ok((temp, root, name))
        } else {
            Ok((temp.clone(), temp, zip_stem))
        }
    }

    /// Copy a resolved mod folder into MO2's mods dir, enable it at the top of
    /// the load order, and record it as a user mod.
    ///
    /// The `[NoDelete]` prefix is added to the mod name so Wabbajack skips this
    /// folder when reinstalling or updating the base modlist.
    fn install_resolved_mod(mod_root: &Path, name: &str) -> Result<String, String> {
        // Strip any existing [NoDelete] prefix before re-applying so we don't
        // double-prefix a mod the caller already named correctly.
        let base = name.trim_start_matches("[NoDelete]").trim();
        let name = format!("[NoDelete] {}", base);
        let name = name.as_str();

        let mods_folder = Self::mods_folder()?;
        let dest = PathBuf::from(&mods_folder).join(name);
        if !dest.exists() {
            FileService::copy_recursive(&mod_root.to_string_lossy(), &dest.to_string_lossy())
                .map_err(|e| format!("Failed to copy mod: {}", e))?;
        }

        // Append at end of modlist — the bottom of modlist.txt is highest
        // priority in MO2, so appending here puts the new mod at the top of
        // MO2's displayed list and ensures it wins file conflicts with all
        // existing base mods.
        let path = Self::modlist_path()?;
        let content = fs::read_to_string(&path).unwrap_or_default();
        let mut lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();
        // Don't duplicate if already present.
        let already = lines.iter().any(|l| l.trim_start_matches(['+', '-']).trim() == name);
        if !already {
            // Insert before any trailing blank lines so there's no gap at the end.
            let trailing_blanks = lines.iter().rev().take_while(|l| l.trim().is_empty()).count();
            let insert_at = lines.len() - trailing_blanks;
            lines.insert(insert_at, format!("+{}", name));
            Self::write_lines(&path, &lines)?;
        }

        let mut user = Self::user_mods();
        if !user.iter().any(|u| u == name) {
            user.push(name.to_string());
            Self::save_user_mods(&user);
        }
        Ok(name.to_string())
    }

    // ── write: reorder ───────────────────────────────────────────────────
    /// Move a mod one slot toward the top of the file (`up = true`) or toward
    /// the bottom (`up = false`). Because MO2 treats file-bottom as highest
    /// priority, `up = false` *increases* in-game priority.
    /// Comments and `*` (DLC/CC) lines are skipped. No-op at the edges.
    pub fn move_mod(name: &str, up: bool) -> Result<(), String> {
        Self::ensure_mo2_closed()?;
        let path = Self::modlist_path()?;
        let content = fs::read_to_string(&path)
            .map_err(|e| format!("Cannot read modlist.txt: {}", e))?;
        let mut lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();

        let is_mod_line = |l: &str| {
            let t = l.trim();
            t.starts_with('+') || t.starts_with('-')
        };
        let target = lines
            .iter()
            .position(|l| {
                let t = l.trim();
                (t.starts_with('+') || t.starts_with('-')) && t[1..].trim() == name
            })
            .ok_or_else(|| format!("Mod not found in modlist: {}", name))?;

        let neighbor = if up {
            (0..target).rev().find(|&i| is_mod_line(&lines[i]))
        } else {
            ((target + 1)..lines.len()).find(|&i| is_mod_line(&lines[i]))
        };

        match neighbor {
            Some(n) => {
                lines.swap(target, n);
                Self::write_lines(&path, &lines)
            }
            None => Ok(()), // already at the edge
        }
    }

    fn write_lines(path: &PathBuf, lines: &[String]) -> Result<(), String> {
        let mut out = lines.join("\n");
        if !out.ends_with('\n') {
            out.push('\n');
        }
        fs::write(path, out).map_err(|e| format!("Cannot write modlist.txt: {}", e))
    }
}
