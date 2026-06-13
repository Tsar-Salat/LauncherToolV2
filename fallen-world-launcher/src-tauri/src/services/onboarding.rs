use std::fs;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

/// Persisted first-time setup result. Lives at
/// `%APPDATA%\FallenWorldLauncher\setup.json` so it survives launcher
/// upgrades and feeds downstream features (FOMOD install, game launch).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnboardingConfig {
    /// Resolution the user confirmed: (width, height).
    pub resolution: Option<(u32, u32)>,
    /// GPU vendor the user confirmed: "NVIDIA" | "AMD" | "Intel".
    pub gpu_vendor: Option<String>,
    /// Upscaler the user chose: "DLAA" | "DLSS" | "FSR".
    pub upscaler: Option<String>,
    /// Per-item acknowledgements for the System Prerequisites checklist.
    pub prereqs_acked: Vec<bool>,
    /// User confirmed they reviewed pagefile configuration.
    pub pagefile_acked: bool,
    /// Setup completed end-to-end at least once.
    pub complete: bool,
}

impl Default for OnboardingConfig {
    fn default() -> Self {
        Self {
            resolution: None,
            gpu_vendor: None,
            upscaler: None,
            prereqs_acked: Vec::new(),
            pagefile_acked: false,
            complete: false,
        }
    }
}

pub struct OnboardingService;

impl OnboardingService {
    /// Resolve `%APPDATA%\FallenWorldLauncher\setup.json`. Creates the
    /// parent directory if missing.
    fn config_path() -> Result<PathBuf, String> {
        let dir = dirs::data_dir()
            .ok_or_else(|| "Cannot determine AppData directory".to_string())?
            .join("FallenWorldLauncher");
        fs::create_dir_all(&dir)
            .map_err(|e| format!("Cannot create config directory: {}", e))?;
        Ok(dir.join("setup.json"))
    }

    /// Load the persisted config. Returns the default (all None / false)
    /// when no file exists or it can't be parsed — first-run looks the
    /// same as "fresh state".
    pub fn load() -> OnboardingConfig {
        let Ok(path) = Self::config_path() else {
            return OnboardingConfig::default();
        };
        let Ok(content) = fs::read_to_string(&path) else {
            return OnboardingConfig::default();
        };
        serde_json::from_str(&content).unwrap_or_default()
    }

    /// Persist the supplied config to disk.
    pub fn save(config: &OnboardingConfig) -> Result<(), String> {
        let path = Self::config_path()?;
        let json = serde_json::to_string_pretty(config)
            .map_err(|e| format!("Cannot serialize setup config: {}", e))?;
        fs::write(&path, json)
            .map_err(|e| format!("Cannot write setup config: {}", e))
    }

    /// Has the user finished first-time setup at least once?
    pub fn is_complete() -> bool {
        Self::load().complete
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_incomplete() {
        let cfg = OnboardingConfig::default();
        assert!(!cfg.complete);
        assert!(cfg.resolution.is_none());
        assert!(cfg.gpu_vendor.is_none());
    }

    #[test]
    fn config_roundtrip_json() {
        let cfg = OnboardingConfig {
            resolution: Some((2560, 1440)),
            gpu_vendor: Some("AMD".to_string()),
            upscaler: Some("FSR".to_string()),
            prereqs_acked: vec![true, true, false, true, false],
            pagefile_acked: true,
            complete: true,
        };
        let json = serde_json::to_string(&cfg).unwrap();
        let parsed: OnboardingConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.resolution, cfg.resolution);
        assert_eq!(parsed.gpu_vendor, cfg.gpu_vendor);
        assert_eq!(parsed.upscaler, cfg.upscaler);
        assert_eq!(parsed.prereqs_acked, cfg.prereqs_acked);
        assert_eq!(parsed.pagefile_acked, cfg.pagefile_acked);
        assert_eq!(parsed.complete, cfg.complete);
    }
}
