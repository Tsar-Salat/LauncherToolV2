use crate::models::OperationResult;
use crate::services::{SystemInfoService, GameLauncher, PathDiscoveryService, ModDetector};
use crate::services::logger::LOGGER;

#[tauri::command]
pub fn run_diagnostics() -> String {
    LOGGER.info("=== DIAGNOSTIC TEST STARTED ===");
    let mut results = String::new();

    // Test 1: System Detection
    LOGGER.info("Test 1: System Detection");
    let system_info = SystemInfoService::get_system_info();
    results.push_str(&format!(
        "✓ System Detection: Resolution {}x{}, GPU: {}, Scale: {}%\n",
        system_info.resolution.0,
        system_info.resolution.1,
        system_info.gpu_vendor,
        (system_info.display_scale * 100.0) as i32
    ));
    LOGGER.info(&format!("Resolution: {}x{}", system_info.resolution.0, system_info.resolution.1));
    LOGGER.info(&format!("GPU: {}", system_info.gpu_vendor));

    // Test 2: Blocking Processes
    LOGGER.info("Test 2: Blocking Processes Check");
    let blocking = SystemInfoService::check_blocking_processes();
    if blocking.is_empty() {
        results.push_str("✓ No blocking processes detected\n");
        LOGGER.info("No blocking processes");
    } else {
        let procs = blocking.iter().map(|p| p.name.clone()).collect::<Vec<_>>().join(", ");
        results.push_str(&format!("⚠ Blocking processes: {}\n", procs));
        LOGGER.warn(&format!("Blocking processes: {}", procs));
    }

    // Test 3: Path Discovery
    LOGGER.info("Test 3: Path Discovery");
    match PathDiscoveryService::discover() {
        Ok(paths) => {
            results.push_str(&format!("✓ Game root: {}\n", paths.game_root));
            results.push_str(&format!("✓ Mods folder: {}\n", paths.mods_folder));
            if let Some(mo2) = &paths.mo2_root {
                results.push_str(&format!("✓ MO2 detected: {}\n", mo2));
                LOGGER.info(&format!("MO2 root: {}", mo2));
            }
            if let Some(profile) = &paths.mo2_profile {
                results.push_str(&format!("✓ Active profile: {}\n", profile));
                LOGGER.info(&format!("Active MO2 profile: {}", profile));
            }
            LOGGER.info(&format!("Game root: {}", paths.game_root));
            LOGGER.info(&format!("Mods folder: {}", paths.mods_folder));

            // Test 4: Mod Detection
            LOGGER.info("Test 4: Mod Detection");
            match ModDetector::list_mods(
                &paths.mods_folder,
                paths.mo2_root.as_deref(),
                paths.mo2_profile.as_deref(),
            ) {
                Ok(mods) => {
                    results.push_str(&format!("✓ Found {} mods\n", mods.len()));
                    LOGGER.info(&format!("Detected {} mods", mods.len()));
                    if mods.len() > 0 && mods.len() <= 10 {
                        for m in &mods {
                            results.push_str(&format!("  - {}: {}\n", m.id, if m.enabled { "ENABLED" } else { "disabled" }));
                            LOGGER.info(&format!("Mod: {} ({})", m.name, if m.enabled { "enabled" } else { "disabled" }));
                        }
                    }
                }
                Err(e) => {
                    results.push_str(&format!("✗ Mod detection failed: {}\n", e));
                    LOGGER.warn(&format!("Mod detection error: {}", e));
                }
            }

            // Test 5: F4SE Detection
            LOGGER.info("Test 5: F4SE Loader Detection");
            let f4se_path = GameLauncher::find_f4se_loader_in(
                &paths.game_root,
                paths.mo2_root.as_deref(),
            );
            if let Some(path) = f4se_path {
                results.push_str(&format!("✓ F4SE loader detected: {}\n", path.display()));
                LOGGER.info(&format!("F4SE loader found: {}", path.display()));
            } else {
                results.push_str("⚠ F4SE loader not found (will use Fallout4.exe)\n");
                LOGGER.warn("F4SE loader not found");
            }
        }
        Err(e) => {
            results.push_str(&format!("✗ Path discovery failed: {}\n", e));
            LOGGER.error(&format!("Path discovery error: {}", e));
        }
    }

    // Test 6: Log File Path
    LOGGER.info("Test 6: Log File Configuration");
    let log_path = LOGGER.get_log_path();
    results.push_str(&format!("✓ Log file: {}\n", log_path));
    LOGGER.info(&format!("Log path: {}", log_path));

    // Test 7: Environment
    LOGGER.info("Test 7: Environment");
    let is_windows = cfg!(target_os = "windows");
    results.push_str(&format!("✓ Platform: Windows={}\n", is_windows));
    LOGGER.info(&format!("Platform detection: {}", if is_windows { "Windows" } else { "Other" }));

    // Test 8: Timestamp
    LOGGER.info("Test 8: System Time");
    let now = chrono::Local::now();
    results.push_str(&format!("✓ System Time: {}\n", now.format("%Y-%m-%d %H:%M:%S")));
    LOGGER.info(&format!("Timestamp: {}", now));

    LOGGER.info("=== DIAGNOSTIC TEST COMPLETED ===");
    results.push_str("\n✓ All diagnostics completed successfully!");
    results
}

#[tauri::command]
pub fn get_logs() -> Vec<String> {
    LOGGER.get_logs()
}

#[tauri::command]
pub fn get_log_file_path() -> String {
    LOGGER.get_log_path()
}

#[tauri::command]
pub fn clear_logs() -> OperationResult {
    LOGGER.clear_buffer();
    LOGGER.info("Logs cleared by user");
    OperationResult {
        success: true,
        message: "Log buffer cleared.".to_string(),
    }
}

/// Open a log file in Explorer with the file pre-selected.
#[tauri::command]
pub fn reveal_log_file(path: String) -> OperationResult {
    #[cfg(target_os = "windows")]
    {
        match std::process::Command::new("explorer")
            .arg(format!("/select,{}", path))
            .spawn()
        {
            Ok(_) => OperationResult { success: true, message: "Opened in Explorer".to_string() },
            Err(e) => OperationResult { success: false, message: format!("Cannot open Explorer: {}", e) },
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = path;
        OperationResult { success: false, message: "Explorer reveal is Windows-only".to_string() }
    }
}

#[tauri::command]
pub fn export_logs(filename: String) -> OperationResult {
    let logs = LOGGER.get_logs();
    let content = logs.join("\n");

    // Write to %APPDATA%\FallenWorldLauncher\ so the file lands in a
    // user-accessible, writable location regardless of where the app is
    // installed (packaged apps often have a read-only CWD).
    let dir = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("FallenWorldLauncher");
    let _ = std::fs::create_dir_all(&dir);
    let out_path = dir.join(&filename);

    match std::fs::write(&out_path, &content) {
        Ok(_) => {
            let path_str = out_path.to_string_lossy().to_string();
            LOGGER.info(&format!("Logs exported to: {}", path_str));
            OperationResult {
                success: true,
                message: format!("Logs exported to:\n{}", path_str),
            }
        }
        Err(e) => {
            LOGGER.error(&format!("Failed to export logs: {}", e));
            OperationResult {
                success: false,
                message: format!("Failed to export logs: {}", e),
            }
        }
    }
}
