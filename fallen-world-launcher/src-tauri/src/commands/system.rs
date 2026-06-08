use crate::services::{SystemInfoService, SystemInfo, PathDiscoveryService, GamePaths, SystemTasks, PagefileInfo};

#[tauri::command]
pub fn get_system_info() -> SystemInfo {
    SystemInfoService::get_system_info()
}

#[tauri::command]
pub fn check_blocking_processes() -> Vec<String> {
    SystemInfoService::check_blocking_processes()
        .iter()
        .map(|p| p.name.clone())
        .collect()
}

#[tauri::command]
pub fn detect_screen_resolution() -> (u32, u32) {
    SystemInfoService::detect_resolution()
}

#[tauri::command]
pub fn detect_gpu_vendor() -> String {
    SystemInfoService::detect_gpu_vendor()
}

#[tauri::command]
pub fn discover_game_paths() -> Result<GamePaths, String> {
    PathDiscoveryService::discover()
}

#[tauri::command]
pub fn set_game_path(path: String) -> Result<String, String> {
    PathDiscoveryService::save_game_path(&path)?;
    Ok(format!("Game path set to: {}", path))
}

#[tauri::command]
pub fn is_game_path_configured() -> bool {
    PathDiscoveryService::is_configured()
}

#[tauri::command]
pub fn get_configured_game_path() -> Option<String> {
    PathDiscoveryService::get_configured_path()
}

/// Force-terminate any running MO2 process. Returns true if MO2 was found and
/// killed. Called on launcher startup so MO2 can't overwrite the mod list.
#[tauri::command]
pub fn check_and_kill_mo2() -> bool {
    #[cfg(target_os = "windows")]
    {
        if SystemInfoService::is_running_under_mo2() {
            if let Ok(exe) = std::env::current_exe() {
                if let Some(exe_dir) = exe.parent() {
                    // Task Scheduler (schtasks) is the ultimate VFS escape. It asks the
                    // unhooked Task Scheduler service to spawn the process, which completely
                    // bypasses all MO2 hooks. Crucially, unlike WMI, it spawns it as the
                    // interactive user, preserving the crucial LOCALAPPDATA environment
                    // variable so WebView2 doesn't white-screen crash on startup.
                    let xml_path = std::env::temp_dir().join("fw_launch_task.xml");
                    let xml_content = format!(
                        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
                        <Task version=\"1.2\" xmlns=\"http://schemas.microsoft.com/windows/2004/02/mit/task\">\n\
                          <Principals>\n\
                            <Principal id=\"Author\">\n\
                              <LogonType>InteractiveToken</LogonType>\n\
                              <RunLevel>LeastPrivilege</RunLevel>\n\
                            </Principal>\n\
                          </Principals>\n\
                          <Settings>\n\
                            <AllowStartOnDemand>true</AllowStartOnDemand>\n\
                            <Enabled>true</Enabled>\n\
                            <Hidden>false</Hidden>\n\
                            <ExecutionTimeLimit>PT0S</ExecutionTimeLimit>\n\
                          </Settings>\n\
                          <Actions Context=\"Author\">\n\
                            <Exec>\n\
                              <Command>{}</Command>\n\
                              <WorkingDirectory>{}</WorkingDirectory>\n\
                            </Exec>\n\
                          </Actions>\n\
                        </Task>",
                        exe.display(),
                        exe_dir.display()
                    );
                    
                    if std::fs::write(&xml_path, xml_content).is_ok() {
                        let task_name = "FallenWorldLauncher_Escape";
                        
                        // Register the task
                        let _ = std::process::Command::new("schtasks")
                            .args(&["/create", "/tn", task_name, "/xml", &xml_path.display().to_string(), "/f"])
                            .output();
                            
                        // Execute it (this spawns the unhooked launcher)
                        let _ = std::process::Command::new("schtasks")
                            .args(&["/run", "/tn", task_name])
                            .output();
                            
                        // Give it a moment to initialize before we delete the task and kill MO2
                        std::thread::sleep(std::time::Duration::from_millis(1500));
                            
                        // Clean up
                        let _ = std::process::Command::new("schtasks")
                            .args(&["/delete", "/tn", task_name, "/f"])
                            .output();
                            
                        let _ = std::fs::remove_file(&xml_path);
                    }
                }
            }
            
            SystemInfoService::kill_mo2();
            std::process::exit(0);
        }
    }
    SystemInfoService::kill_mo2()
}

/// Startup MO2 guard: if launched inside MO2's VFS, relaunch outside it; else
/// close MO2 if running. See `SystemInfoService::mo2_startup`.
#[tauri::command]
pub fn mo2_startup() -> crate::services::system_info::Mo2StartupResult {
    SystemInfoService::mo2_startup()
}

/// RAM, recommended size, drives, and the modlist drive — for the pagefile UI.
#[tauri::command]
pub fn get_pagefile_info() -> PagefileInfo {
    let install_drive = PathDiscoveryService::get_configured_path()
        .and_then(|p| p.get(0..2).map(|s| s.to_string()))
        .unwrap_or_else(|| "C:".to_string());
    SystemTasks::pagefile_info(&install_drive)
}

/// Configure the Windows pagefile (elevated). Returns Ok or an error message.
#[tauri::command]
pub async fn configure_pagefile(drive: String, target_mb: u64) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || SystemTasks::configure_pagefile(&drive, target_mb))
        .await
        .map_err(|e| format!("pagefile task panicked: {}", e))?
}

/// Returns true if the Visual C++ 2015-2022 x64 Redistributable is installed.
#[tauri::command]
pub fn check_msvc_installed() -> bool {
    SystemInfoService::check_msvc_installed()
}

/// Add a Windows Defender folder exclusion for the MO2 modlist root (elevated UAC).
#[tauri::command]
pub async fn add_antivirus_exclusion() -> crate::models::OperationResult {
    let mo2_root = match PathDiscoveryService::discover() {
        Ok(paths) => match paths.mo2_root {
            Some(root) => root,
            None => return crate::models::OperationResult {
                success: false,
                message: "MO2 root not found. Configure your game path in Settings first.".to_string(),
            },
        },
        Err(e) => return crate::models::OperationResult { success: false, message: e },
    };

    tauri::async_runtime::spawn_blocking(move || {
        match SystemTasks::add_defender_exclusion(&mo2_root) {
            Ok(path) => crate::models::OperationResult {
                success: true,
                message: format!("Exclusion added for: {}", path),
            },
            Err(e) => crate::models::OperationResult {
                success: false,
                message: format!("Auto-exclusion failed: {}. Please add manually via Windows Defender settings.", e),
            },
        }
    })
    .await
    .unwrap_or_else(|_| crate::models::OperationResult {
        success: false,
        message: "Task panicked — please add exclusion manually.".to_string(),
    })
}
