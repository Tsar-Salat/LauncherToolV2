//! Locate and launch Fluorine Manager (the Linux MO2 port).
//!
//! Port of the Python `fluorine_runner.py`. CLF3 registers the modlist as a
//! Fluorine portable instance at install time; our job is just to start
//! Fluorine focused on a specific instance directory.

use std::path::{Path, PathBuf};

use super::platform::Platform;

const FLUORINE_BIN_NAME: &str = "fluorine-manager";

pub struct Fluorine;

impl Fluorine {
    /// Resolve the `fluorine-manager` binary, or `None` if missing.
    /// Order: explicit override → `$PATH` → `~/.local/share/fluorine-manager/`
    /// → `~/Applications/fluorine-manager/`.
    pub fn find_binary(override_path: Option<&Path>) -> Option<PathBuf> {
        if let Some(p) = override_path {
            if let Some(found) = resolve_install(p) {
                return Some(found);
            }
        }
        if let Some(p) = which(FLUORINE_BIN_NAME) {
            return Some(p);
        }
        let home = Platform::home();
        for candidate in [
            home.join(".local").join("share").join(FLUORINE_BIN_NAME),
            home.join("Applications").join(FLUORINE_BIN_NAME),
        ] {
            if let Some(found) = resolve_install(&candidate) {
                return Some(found);
            }
        }
        None
    }

    /// Launch Fluorine focused on `install_dir` (detached).
    pub fn open_instance(install_dir: &str, override_path: Option<&Path>) -> Result<u32, String> {
        let binary = Self::find_binary(override_path).ok_or_else(|| {
            "fluorine-manager not found. Run the Linux bootstrap to install it, or set an \
             override path."
                .to_string()
        })?;
        Platform::spawn_detached(
            &binary.to_string_lossy(),
            &["--instance".to_string(), install_dir.to_string()],
            None,
        )
    }
}

/// Treat `path` as a file or a directory and return the binary inside it.
/// Release archives extract into a single `fluorine-manager-X.Y.Z/` subdir,
/// so we also probe one level down.
fn resolve_install(path: &Path) -> Option<PathBuf> {
    if !path.exists() {
        return None;
    }
    if path.is_file() {
        return Some(path.to_path_buf());
    }
    let direct = path.join(FLUORINE_BIN_NAME);
    if direct.is_file() {
        return Some(direct);
    }
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let child = entry.path();
            if child.is_dir() {
                let nested = child.join(FLUORINE_BIN_NAME);
                if nested.is_file() {
                    return Some(nested);
                }
            }
        }
    }
    None
}

fn which(name: &str) -> Option<PathBuf> {
    let path_var = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path_var) {
        let full = dir.join(name);
        if full.is_file() {
            return Some(full);
        }
    }
    None
}
