//! Linux first-run installer orchestration.
//!
//! Port of the headless path of the Python `linux_bootstrap.py`. Fetches the
//! Fallen World gallery metadata (to discover the current `.wabbajack` URL,
//! which rotates every release) and drives CLF3 to install + register the
//! modlist as a Fluorine instance.
//!
//! HTTP is done via `curl` (present on Linux and Windows 10+) to avoid pulling
//! a TLS stack into the dependency tree.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::process::Command;

use super::clf3::Clf3;
use super::fluorine::Fluorine;

/// Authoritative gallery JSON. Every release rotates the UUID inside
/// `links.download`, so this must be fetched at runtime.
pub const MODLIST_JSON_URL: &str =
    "https://raw.githubusercontent.com/NomadsReach/Fallout-Anomaly/master/modlist.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModlistMetadata {
    pub title: String,
    pub version: String,
    pub download_url: String,
    pub machine_url: Option<String>,
    pub archives_size: Option<u64>,
    pub installed_size: Option<u64>,
}

pub struct LinuxBootstrap;

impl LinuxBootstrap {
    /// Fetch + parse the gallery metadata, returning the first (only) entry.
    pub fn fetch_modlist_metadata(url: Option<&str>) -> Result<ModlistMetadata, String> {
        let url = url.unwrap_or(MODLIST_JSON_URL);
        let output = Command::new("curl")
            .args(["-fsSL", "--max-time", "15", "-A", "FallenWorldLauncher/1.0", url])
            .output()
            .map_err(|e| format!("curl not available to fetch {}: {}", url, e))?;
        if !output.status.success() {
            return Err(format!(
                "Could not reach {} (curl exit {})",
                url,
                output.status.code().unwrap_or(-1)
            ));
        }
        let body = String::from_utf8_lossy(&output.stdout);
        let data: Value = serde_json::from_str(body.trim())
            .map_err(|e| format!("Invalid JSON at {}: {}", url, e))?;

        let entry = data
            .as_array()
            .and_then(|a| a.first())
            .ok_or_else(|| format!("{} did not return a non-empty array", url))?;

        let download_url = entry
            .pointer("/links/download")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if download_url.is_empty() {
            return Err(format!("{} entry has no links.download — modlist may be unpublished", url));
        }

        let dm = entry.get("download_metadata");
        Ok(ModlistMetadata {
            title: entry.get("title").and_then(|v| v.as_str()).unwrap_or("(unknown)").to_string(),
            version: entry.get("version").and_then(|v| v.as_str()).unwrap_or("(unknown)").to_string(),
            download_url,
            machine_url: entry
                .pointer("/links/machineURL")
                .and_then(|v| v.as_str())
                .map(String::from),
            archives_size: dm.and_then(|d| d.get("SizeOfArchives")).and_then(|v| v.as_u64()),
            installed_size: dm.and_then(|d| d.get("SizeOfInstalledFiles")).and_then(|v| v.as_u64()),
        })
    }

    /// Headless install: resolve the wabbajack URL (pinned or via gallery),
    /// then drive CLF3. Returns CLF3's parsed report JSON. Blocking.
    pub fn install(
        nexus_key: &str,
        downloads: &str,
        output: &str,
        pinned_wabbajack_url: Option<&str>,
        modlist_json: Option<&str>,
    ) -> Result<Value, String> {
        if Clf3::find().is_none() {
            return Err(
                "clf3 binary not found. Install CLF3 or set $CLF3_BIN to a local build."
                    .to_string(),
            );
        }

        let wabbajack_url = match pinned_wabbajack_url {
            Some(u) if !u.is_empty() => u.to_string(),
            _ => Self::fetch_modlist_metadata(modlist_json)?.download_url,
        };

        Clf3::install_modlist(&wabbajack_url, downloads, output, nexus_key, true)
    }

    /// After a successful install, open Fluorine focused on the install dir.
    pub fn launch_fluorine(install_dir: &str) -> Result<u32, String> {
        Fluorine::open_instance(install_dir, None)
    }
}
