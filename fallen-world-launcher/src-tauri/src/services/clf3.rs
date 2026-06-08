//! Subprocess bridge to the CLF3 Wabbajack installer.
//!
//! Port of the Python `clf3_bridge.py`. CLF3 (https://github.com/SulfurNitride/CLF3)
//! is the Linux-native Wabbajack modlist installer. This is a thin synchronous
//! wrapper over the `clf3` binary; long-running calls should be invoked from a
//! background task by the caller.
//!
//! Binary resolution order: `$CLF3_BIN` → `clf3` on `$PATH` →
//! `~/.local/bin/clf3` → `~/.local/share/clf3/clf3`.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
use std::process::Command;

use super::platform::Platform;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Clf3Status {
    pub binary: Option<String>,
    pub version: Option<String>,
    pub has_api_key: bool,
    pub fluorine_binary: Option<String>,
}

pub struct Clf3;

impl Clf3 {
    // ── Binary resolution ────────────────────────────────────────────────

    /// Locate the `clf3` binary, or `None` if it isn't installed.
    pub fn find() -> Option<PathBuf> {
        if let Ok(env_override) = std::env::var("CLF3_BIN") {
            let p = PathBuf::from(env_override.trim());
            if p.is_file() {
                return Some(p);
            }
        }
        if let Some(p) = which("clf3") {
            return Some(p);
        }
        let home = Platform::home();
        for candidate in [
            home.join(".local").join("bin").join("clf3"),
            home.join(".local").join("share").join("clf3").join("clf3"),
        ] {
            if candidate.is_file() {
                return Some(candidate);
            }
        }
        None
    }

    fn require() -> Result<PathBuf, String> {
        Self::find().ok_or_else(|| {
            "clf3 binary not found on $PATH, ~/.local/bin/, or via $CLF3_BIN. \
             Install from https://github.com/SulfurNitride/CLF3/releases, or set \
             $CLF3_BIN to a local build."
                .to_string()
        })
    }

    // ── Internal runners ─────────────────────────────────────────────────

    fn run(args: &[&str]) -> Result<String, String> {
        let binary = Self::require()?;
        let output = Command::new(&binary)
            .args(args)
            .output()
            .map_err(|e| format!("clf3 {} failed to start: {}", args.first().unwrap_or(&""), e))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!(
                "clf3 {} exited {}: {}",
                args.first().unwrap_or(&""),
                output.status.code().unwrap_or(-1),
                stderr.trim()
            ));
        }
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    fn run_json(args: &[&str]) -> Result<Value, String> {
        let stdout = Self::run(args)?;
        let trimmed = stdout.trim();
        if trimmed.is_empty() {
            return Err(format!("clf3 {} produced no JSON output", args.first().unwrap_or(&"")));
        }
        serde_json::from_str(trimmed)
            .map_err(|e| format!("clf3 {} returned invalid JSON: {}", args.first().unwrap_or(&""), e))
    }

    // ── Settings (read directly, XDG-aware) ──────────────────────────────

    fn settings_path() -> PathBuf {
        let config_home = std::env::var("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| Platform::home().join(".config"));
        config_home.join("clf3").join("settings.json")
    }

    fn read_settings() -> Value {
        let path = Self::settings_path();
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|c| serde_json::from_str(&c).ok())
            .unwrap_or(Value::Null)
    }

    /// True when CLF3 already has a non-empty Nexus key saved.
    pub fn has_saved_api_key() -> bool {
        Self::read_settings()
            .get("nexus_api_key")
            .and_then(|v| v.as_str())
            .map(|s| !s.is_empty())
            .unwrap_or(false)
    }

    // ── Public API ───────────────────────────────────────────────────────

    /// Persist + validate a Nexus Mods API key (CLF3 verifies before saving).
    pub fn set_nexus_api_key(key: &str) -> Result<(), String> {
        if key.is_empty() {
            return Err("Nexus API key cannot be empty".to_string());
        }
        Self::run(&["set-api-key", key]).map(|_| ())
    }

    /// Toggle CLF3's `add_to_fluorine` setting if it doesn't already match.
    pub fn enable_fluorine_auto_register(enable: bool) -> Result<(), String> {
        if let Some(current) = Self::read_settings().get("add_to_fluorine").and_then(|v| v.as_bool()) {
            if current == enable {
                return Ok(());
            }
        }
        match Self::run(&["fluorine", "enable", if enable { "true" } else { "false" }]) {
            Ok(_) => Ok(()),
            // Known clap bug in CLF3 0.1.0 rejects the positional arg; the
            // install still succeeds, so warn and continue.
            Err(e) if e.contains("0 values required") => {
                eprintln!("clf3 fluorine enable rejected its arg (known 0.1.0 bug); continuing.");
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    /// Ensure Fluorine Manager is installed (downloads it if missing).
    pub fn ensure_fluorine() -> Result<Value, String> {
        let data = Self::run_json(&["fluorine", "ensure", "--json"])?;
        if data.get("binary").is_none() {
            return Err(format!("unexpected fluorine ensure payload: {}", data));
        }
        Ok(data)
    }

    /// Run a Wabbajack modlist install end-to-end via CLF3. Returns CLF3's
    /// parsed `--report-json`. Blocking; run on a background task.
    pub fn install_modlist(
        wabbajack_url: &str,
        downloads: &str,
        output: &str,
        nexus_key: &str,
        auto_fluorine: bool,
    ) -> Result<Value, String> {
        if !nexus_key.is_empty() {
            Self::set_nexus_api_key(nexus_key)?;
        } else if !Self::has_saved_api_key() {
            return Err(
                "No Nexus API key supplied and CLF3 has none saved. Pass a key or run \
                 `clf3 set-api-key YOUR_KEY` first."
                    .to_string(),
            );
        }
        Self::enable_fluorine_auto_register(auto_fluorine)?;

        std::fs::create_dir_all(downloads).map_err(|e| format!("Cannot create downloads dir: {}", e))?;
        std::fs::create_dir_all(output).map_err(|e| format!("Cannot create output dir: {}", e))?;

        let report = std::env::temp_dir().join("clf3-report.json");
        let report_str = report.to_string_lossy().to_string();
        Self::run(&[
            "install",
            wabbajack_url,
            downloads,
            output,
            "--report-json",
            &report_str,
        ])?;

        let content = std::fs::read_to_string(&report)
            .map_err(|e| format!("clf3 install produced no report at {}: {}", report_str, e))?;
        let _ = std::fs::remove_file(&report);
        serde_json::from_str(&content).map_err(|e| format!("clf3 report JSON invalid: {}", e))
    }

    /// Compare installed modlist versions against the gallery.
    pub fn check_modlist_updates(name: Option<&str>) -> Result<Value, String> {
        match name {
            Some(n) => Self::run_json(&["modlist", "check", "--json", "--name", n]),
            None => Self::run_json(&["modlist", "check", "--json"]),
        }
    }

    /// Enumerate available GPUs via CLF3's wgpu adapter list.
    pub fn list_gpus() -> Result<Vec<Value>, String> {
        let data = Self::run_json(&["list-gpu", "--json"])?;
        data.as_array()
            .cloned()
            .ok_or_else(|| format!("unexpected list-gpu payload: {}", data))
    }

    /// Derive vendor ("NVIDIA"/"AMD"/"Intel"/"Unknown") from `list_gpus`.
    pub fn gpu_vendor() -> String {
        let Ok(gpus) = Self::list_gpus() else {
            return "Unknown".to_string();
        };
        for gpu in gpus {
            let name = gpu.get("name").and_then(|v| v.as_str()).unwrap_or("").to_lowercase();
            if name.contains("nvidia") {
                return "NVIDIA".to_string();
            }
            if name.contains("amd") || name.contains("radeon") {
                return "AMD".to_string();
            }
            if name.contains("intel") {
                return "Intel".to_string();
            }
        }
        "Unknown".to_string()
    }

    /// One-shot status report for diagnostics UI.
    pub fn status() -> Clf3Status {
        let binary = Self::find();
        let version = binary
            .as_ref()
            .and_then(|b| Command::new(b).arg("--version").output().ok())
            .and_then(|o| {
                let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
                if s.is_empty() { None } else { Some(s) }
            });
        let fluorine_binary = binary
            .as_ref()
            .and_then(|_| Self::ensure_fluorine().ok())
            .and_then(|v| v.get("binary").and_then(|b| b.as_str()).map(String::from));
        Clf3Status {
            binary: binary.map(|b| b.to_string_lossy().to_string()),
            version,
            has_api_key: Self::has_saved_api_key(),
            fluorine_binary,
        }
    }
}

/// Minimal cross-platform `which`: scan `$PATH` for an executable named `name`
/// (appends `.exe` on Windows). Avoids pulling in an extra crate.
fn which(name: &str) -> Option<PathBuf> {
    let path_var = std::env::var_os("PATH")?;
    let exe_names: Vec<String> = if cfg!(target_os = "windows") {
        vec![format!("{}.exe", name), name.to_string()]
    } else {
        vec![name.to_string()]
    };
    for dir in std::env::split_paths(&path_var) {
        for candidate in &exe_names {
            let full = dir.join(candidate);
            if full.is_file() {
                return Some(full);
            }
        }
    }
    None
}
