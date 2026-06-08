use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    pub resolution: (u32, u32),
    pub gpu_vendor: String,
    pub display_scale: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockingProcess {
    pub name: String,
    pub pid: u32,
}

/// Result of the startup MO2 guard.
#[derive(Debug, Clone, Serialize)]
pub struct Mo2StartupResult {
    /// MO2 was running and has been closed automatically (normal launch).
    pub killed_mo2: bool,
    /// The launcher was started *through* MO2's VFS — the UI prompts the user to
    /// close MO2 via an OK button rather than killing it automatically.
    pub under_mo2: bool,
}

/// Live running/stopped state for the dashboard "Game Status" widget.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessStatus {
    pub fallout4_running: bool,
    pub mo2_running: bool,
    pub f4se_running: bool,
}

pub struct SystemInfoService;

impl SystemInfoService {
    pub fn detect_resolution() -> (u32, u32) {
        #[cfg(target_os = "windows")]
        {
            use winapi::um::winuser::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};
            unsafe {
                let width = GetSystemMetrics(SM_CXSCREEN) as u32;
                let height = GetSystemMetrics(SM_CYSCREEN) as u32;
                if width > 0 && height > 0 {
                    return (width, height);
                }
            }
        }

        // Fallback
        (1920, 1080)
    }

    pub fn detect_gpu_vendor() -> String {
        #[cfg(target_os = "windows")]
        {
            use winreg::RegKey;

            // Skip virtual / remote display adapters that can shadow the real card.
            const SKIP: [&str; 8] =
                ["virtual", "basic", "remote", "idd", "parsec", "citrix", "rdp", "meta"];

            // Scan ALL display adapters and prefer the discrete gaming GPU:
            // NVIDIA wins over an integrated AMD/Intel iGPU (which often enumerates
            // first on Ryzen/Intel systems and used to mask a real NVIDIA card).
            let mut found_amd = false;
            let mut found_intel = false;

            if let Ok(hklm) = RegKey::predef(winreg::enums::HKEY_LOCAL_MACHINE).open_subkey(
                r"SYSTEM\CurrentControlSet\Control\Class\{4D36E968-E325-11CE-BFC1-08002BE10318}"
            ) {
                for key_name in hklm.enum_keys().flatten() {
                    if let Ok(subkey) = hklm.open_subkey(&key_name) {
                        if let Ok(driver_desc) = subkey.get_value::<String, _>("DriverDesc") {
                            let d = driver_desc.to_lowercase();
                            if SKIP.iter().any(|s| d.contains(s)) {
                                continue;
                            }
                            if d.contains("nvidia") {
                                return "NVIDIA".to_string();
                            }
                            if d.contains("amd") || d.contains("radeon") {
                                found_amd = true;
                            } else if d.contains("intel") {
                                found_intel = true;
                            }
                        }
                    }
                }
            }

            if found_amd {
                return "AMD".to_string();
            }
            if found_intel {
                return "Intel".to_string();
            }
        }

        "Unknown".to_string()
    }

    pub fn detect_display_scale() -> f32 {
        // For now, return 1.0 (100% scale)
        // Full DPI detection requires complex Windows API calls
        // This can be implemented later if needed
        1.0
    }

    pub fn get_system_info() -> SystemInfo {
        SystemInfo {
            resolution: Self::detect_resolution(),
            gpu_vendor: Self::detect_gpu_vendor(),
            display_scale: Self::detect_display_scale(),
        }
    }

    pub fn check_blocking_processes() -> Vec<BlockingProcess> {
        #[cfg(target_os = "windows")]
        {
            let processes = Self::list_running_processes();
            let blockers = vec!["fallout4.exe", "f4se_loader.exe", "modorganizer.exe", "mo2.exe"];

            processes
                .into_iter()
                .filter(|p| blockers.iter().any(|b| p.name.to_lowercase().contains(b)))
                .collect()
        }

        #[cfg(not(target_os = "windows"))]
        Vec::new()
    }

    /// Report which key processes are currently running. Used by the dashboard
    /// to show live Running/Stopped chips for Fallout 4 and Mod Organizer 2.
    ///
    /// Uses **exact** basename matching so modding tools like
    /// `Spriggit.Json.Fallout4.exe` are not mistaken for the game.
    pub fn process_status() -> ProcessStatus {
        let names: Vec<String> = Self::list_running_processes()
            .into_iter()
            .map(|p| p.name.to_lowercase())
            .collect();
        let is = |exe: &str| names.iter().any(|n| n == exe);
        ProcessStatus {
            fallout4_running: is("fallout4.exe"),
            mo2_running: is("modorganizer.exe") || is("mo2.exe"),
            f4se_running: is("f4se_loader.exe"),
        }
    }

    /// Kill every running MO2 process. Returns `true` if at least one was found
    /// and terminated. Used on launcher startup so the user's mod list isn't
    /// silently overwritten by MO2 on exit.
    pub fn kill_mo2() -> bool {
        let mo2_pids: Vec<u32> = Self::list_running_processes()
            .into_iter()
            .filter(|p| {
                let n = p.name.to_lowercase();
                n == "modorganizer.exe" || n == "mo2.exe"
            })
            .map(|p| p.pid)
            .collect();

        if mo2_pids.is_empty() {
            return false;
        }

        #[cfg(target_os = "windows")]
        {
            use winapi::um::handleapi::CloseHandle;
            use winapi::um::processthreadsapi::{OpenProcess, TerminateProcess};
            use winapi::um::winnt::PROCESS_TERMINATE;

            for pid in &mo2_pids {
                unsafe {
                    let handle = OpenProcess(PROCESS_TERMINATE, 0, *pid);
                    if !handle.is_null() && handle as isize != -1 {
                        TerminateProcess(handle, 1);
                        CloseHandle(handle);
                    }
                }
            }
        }

        true
    }

    pub fn list_running_processes_pub() -> Vec<BlockingProcess> {
        Self::list_running_processes()
    }

    /// Startup MO2 guard.
    ///
    /// Auto-escaping MO2's VFS proved unreliable across setups, so instead:
    ///   • Launched **through MO2** → report `under_mo2` and DON'T touch MO2. The
    ///     UI keeps the launcher forced-open with an OK button that closes MO2 on
    ///     demand (the launcher is a separate process and survives MO2 exiting).
    ///   • Launched normally with MO2 running → close MO2 and report `killed_mo2`.
    pub fn mo2_startup() -> Mo2StartupResult {
        #[cfg(target_os = "windows")]
        {
            if Self::is_running_under_mo2() {
                return Mo2StartupResult { killed_mo2: false, under_mo2: true };
            }
            let killed = Self::kill_mo2();
            Mo2StartupResult { killed_mo2: killed, under_mo2: false }
        }
        #[cfg(not(target_os = "windows"))]
        {
            Mo2StartupResult { killed_mo2: false, under_mo2: false }
        }
    }

    /// True if this process is running inside MO2's usvfs (a usvfs module is
    /// loaded) or was launched directly by ModOrganizer.exe.
    #[cfg(target_os = "windows")]
    pub fn is_running_under_mo2() -> bool {
        use std::mem;
        use winapi::shared::minwindef::FALSE;
        use winapi::um::handleapi::CloseHandle;
        use winapi::um::tlhelp32::{
            CreateToolhelp32Snapshot, Module32First, Module32Next, MODULEENTRY32,
            TH32CS_SNAPMODULE, TH32CS_SNAPMODULE32,
        };

        unsafe {
            let pid = std::process::id();
            let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPMODULE | TH32CS_SNAPMODULE32, pid);
            if snapshot as isize != -1 && !snapshot.is_null() {
                let mut me: MODULEENTRY32 = mem::zeroed();
                me.dwSize = mem::size_of::<MODULEENTRY32>() as u32;
                let mut found = false;
                if Module32First(snapshot, &mut me) != FALSE {
                    loop {
                        let bytes: &[u8] = std::mem::transmute::<&[i8], &[u8]>(&me.szModule[..]);
                        let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
                        let name = String::from_utf8_lossy(&bytes[..end]).to_lowercase();
                        if name.contains("usvfs") {
                            found = true;
                            break;
                        }
                        if Module32Next(snapshot, &mut me) == FALSE {
                            break;
                        }
                    }
                }
                CloseHandle(snapshot);
                if found {
                    return true;
                }
            }
        }
        Self::parent_is_mo2()
    }

    #[cfg(target_os = "windows")]
    fn parent_is_mo2() -> bool {
        use std::mem;
        use winapi::shared::minwindef::FALSE;
        use winapi::um::handleapi::CloseHandle;
        use winapi::um::tlhelp32::{
            CreateToolhelp32Snapshot, Process32First, Process32Next, PROCESSENTRY32, TH32CS_SNAPPROCESS,
        };

        let my_pid = std::process::id();
        unsafe {
            // Find our parent PID.
            let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
            if snapshot as isize == -1 || snapshot.is_null() {
                return false;
            }
            let mut pe: PROCESSENTRY32 = mem::zeroed();
            pe.dwSize = mem::size_of::<PROCESSENTRY32>() as u32;
            let mut parent_pid = 0u32;
            if Process32First(snapshot, &mut pe) != FALSE {
                loop {
                    if pe.th32ProcessID == my_pid {
                        parent_pid = pe.th32ParentProcessID;
                        break;
                    }
                    if Process32Next(snapshot, &mut pe) == FALSE {
                        break;
                    }
                }
            }
            CloseHandle(snapshot);
            if parent_pid == 0 {
                return false;
            }

            // Resolve the parent's executable name.
            let snap2 = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
            if snap2 as isize == -1 || snap2.is_null() {
                return false;
            }
            let mut pe2: PROCESSENTRY32 = mem::zeroed();
            pe2.dwSize = mem::size_of::<PROCESSENTRY32>() as u32;
            let mut is_mo2 = false;
            if Process32First(snap2, &mut pe2) != FALSE {
                loop {
                    if pe2.th32ProcessID == parent_pid {
                        let bytes: &[u8] = std::mem::transmute::<&[i8], &[u8]>(&pe2.szExeFile[..]);
                        let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
                        let name = String::from_utf8_lossy(&bytes[..end]).to_lowercase();
                        is_mo2 = name.contains("modorganizer");
                        break;
                    }
                    if Process32Next(snap2, &mut pe2) == FALSE {
                        break;
                    }
                }
            }
            CloseHandle(snap2);
            is_mo2
        }
    }

    /// Check whether the Visual C++ 2015-2022 x64 Redistributable is installed.
    /// Reads from the registry path MO2 and F4SE both depend on.
    pub fn check_msvc_installed() -> bool {
        #[cfg(target_os = "windows")]
        {
            use winreg::RegKey;
            use winreg::enums::HKEY_LOCAL_MACHINE;

            // Try both the native 64-bit hive and the WOW64 redirect
            let paths = [
                r"SOFTWARE\Microsoft\VisualStudio\14.0\VC\Runtimes\x64",
                r"SOFTWARE\WOW6432Node\Microsoft\VisualStudio\14.0\VC\Runtimes\x64",
            ];
            for path in &paths {
                if let Ok(key) = RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey(path) {
                    // `Installed` DWORD == 1 means the runtime is present
                    if let Ok(val) = key.get_value::<u32, _>("Installed") {
                        if val == 1 {
                            return true;
                        }
                    }
                }
            }
            // Fallback: presence of vcruntime140.dll in System32
            std::path::Path::new(r"C:\Windows\System32\vcruntime140.dll").exists()
        }

        #[cfg(not(target_os = "windows"))]
        {
            true // Non-Windows: assume present, not applicable
        }
    }

    #[cfg(target_os = "windows")]
    fn list_running_processes() -> Vec<BlockingProcess> {
        use std::mem;
        use std::ptr;
        use winapi::shared::minwindef::FALSE;
        use winapi::um::tlhelp32::{CreateToolhelp32Snapshot, Process32First, Process32Next, TH32CS_SNAPPROCESS, PROCESSENTRY32};
        use winapi::um::winnt::HANDLE;

        let mut processes = Vec::new();

        unsafe {
            let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
            if snapshot == ptr::null_mut() as HANDLE || snapshot as i32 == -1 {
                return processes;
            }

            let mut pe: PROCESSENTRY32 = mem::zeroed();
            pe.dwSize = mem::size_of::<PROCESSENTRY32>() as u32;

            if Process32First(snapshot, &mut pe) != FALSE {
                loop {
                    // szExeFile is a fixed 260-byte buffer; the actual name is
                    // terminated by the first null byte. Anything after that is
                    // stale data left over from previous iterations.
                    let bytes: &[u8] = std::mem::transmute::<&[i8], &[u8]>(&pe.szExeFile[..]);
                    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
                    let name = String::from_utf8_lossy(&bytes[..end]).to_string();

                    processes.push(BlockingProcess {
                        name,
                        pid: pe.th32ProcessID,
                    });

                    if Process32Next(snapshot, &mut pe) == FALSE {
                        break;
                    }
                }
            }

            let _ = winapi::um::handleapi::CloseHandle(snapshot);
        }

        processes
    }

    #[cfg(not(target_os = "windows"))]
    fn list_running_processes() -> Vec<BlockingProcess> {
        Vec::new()
    }
}

