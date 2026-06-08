//! Fallout Anomaly update notifier.
//!
//! Reads `version.md` from the modlist's changelog repo on launch, compares the
//! published version against the last one the user has seen, and reports whether
//! a new update is available so the UI can drop a banner.
//!
//! `version.md` format (single line): `Fallen World 0.1.3 - <description>`

use regex::Regex;
use crate::services::platform::CreationFlagsNoop;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

pub const VERSION_MD_URL: &str =
    "https://raw.githubusercontent.com/Fallout-Anomaly/changelog/main/version.md";

/// Fallback news shown in the Commonwealth News banner when there is no new
/// update to announce.
pub const NEWSBANNER_MD_URL: &str =
    "https://raw.githubusercontent.com/Fallout-Anomaly/changelog/main/newsbanner.md";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyUpdate {
    /// Parsed version string, e.g. "0.1.3".
    pub version: String,
    /// Human description after the " - " separator (may be empty).
    pub description: String,
    /// The full raw line from version.md.
    pub raw: String,
    /// True when `version` differs from the last version the user acknowledged.
    pub is_new: bool,
}

pub struct AnomalyUpdateService;

impl AnomalyUpdateService {
    /// Fetch + parse version.md and flag whether it's newer than last-seen.
    pub fn check() -> Result<AnomalyUpdate, String> {
        let raw = Self::fetch()?.trim().to_string();
        if raw.is_empty() {
            return Err("version.md was empty".to_string());
        }

        let version = Regex::new(r"\d+\.\d+(?:\.\d+)?")
            .ok()
            .and_then(|re| re.find(&raw).map(|m| m.as_str().to_string()))
            .unwrap_or_else(|| raw.clone());

        let description = raw
            .split_once(" - ")
            .map(|(_, d)| d.trim().to_string())
            .unwrap_or_default();

        let is_new = Self::last_seen().as_deref() != Some(version.as_str());

        Ok(AnomalyUpdate { version, description, raw, is_new })
    }

    /// Fetch the latest news-banner text (shown when no update is pending).
    pub fn news_banner() -> Result<String, String> {
        let out = Command::new("curl")
            .args(["-fsSL", "--max-time", "12", "-A", "FallenWorldLauncher/1.0", NEWSBANNER_MD_URL])
            .creation_flags_noop()
            .output()
            .map_err(|e| format!("curl unavailable: {}", e))?;
        if !out.status.success() {
            return Err("newsbanner.md fetch failed".to_string());
        }
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    }

    /// Record `version` as seen so it stops being flagged as new.
    pub fn mark_seen(version: &str) -> Result<(), String> {
        let path = Self::seen_path()?;
        fs::write(&path, version).map_err(|e| format!("Cannot persist seen version: {}", e))
    }

    fn fetch() -> Result<String, String> {
        let out = Command::new("curl")
            .args(["-fsSL", "--max-time", "15", "-A", "FallenWorldLauncher/1.0", VERSION_MD_URL])
            .creation_flags_noop()
            .output()
            .map_err(|e| format!("curl unavailable: {}", e))?;
        if !out.status.success() {
            return Err(format!(
                "Could not reach version.md (curl exit {})",
                out.status.code().unwrap_or(-1)
            ));
        }
        Ok(String::from_utf8_lossy(&out.stdout).to_string())
    }

    fn last_seen() -> Option<String> {
        let path = Self::seen_path().ok()?;
        fs::read_to_string(path).ok().map(|s| s.trim().to_string())
    }

    fn seen_path() -> Result<PathBuf, String> {
        let appdata = std::env::var("APPDATA")
            .map_err(|_| "Cannot determine AppData directory".to_string())?;
        let dir = PathBuf::from(appdata).join("FallenWorldLauncher");
        fs::create_dir_all(&dir).map_err(|e| format!("Cannot create config dir: {}", e))?;
        Ok(dir.join("last_seen_version.txt"))
    }
}
