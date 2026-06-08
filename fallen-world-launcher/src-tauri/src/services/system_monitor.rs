use serde::{Deserialize, Serialize};

/// A single live-stats sample for the dashboard "Status" widget. Percentages
/// are 0-100 floats; `gpu_usage` is `None` when no usable reading is available
/// (non-NVIDIA GPUs, missing `nvidia-smi`, or unsupported platforms).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveStats {
    pub cpu_name: String,
    pub cpu_usage: f32,
    pub gpu_name: String,
    pub gpu_usage: Option<f32>,
    pub ram_used_gb: f32,
    pub ram_total_gb: f32,
    pub ram_usage: f32,
}

pub struct SystemMonitor;

impl SystemMonitor {
    /// Take one live sample. Intended to be polled (~every 2s) by the UI.
    pub fn sample() -> LiveStats {
        let (ram_used_gb, ram_total_gb, ram_usage) = Self::memory();
        LiveStats {
            cpu_name: Self::cpu_name(),
            cpu_usage: Self::cpu_usage(),
            gpu_name: Self::gpu_name(),
            gpu_usage: Self::gpu_usage(),
            ram_used_gb,
            ram_total_gb,
            ram_usage,
        }
    }

    // ── CPU ────────────────────────────────────────────────────────────

    fn cpu_name() -> String {
        #[cfg(target_os = "windows")]
        {
            use winreg::RegKey;
            if let Ok(key) = RegKey::predef(winreg::enums::HKEY_LOCAL_MACHINE)
                .open_subkey(r"HARDWARE\DESCRIPTION\System\CentralProcessor\0")
            {
                if let Ok(name) = key.get_value::<String, _>("ProcessorNameString") {
                    return Self::clean_cpu_name(&name);
                }
            }
        }
        #[cfg(target_os = "linux")]
        {
            if let Ok(content) = std::fs::read_to_string("/proc/cpuinfo") {
                for line in content.lines() {
                    if let Some(rest) = line.strip_prefix("model name") {
                        if let Some((_, v)) = rest.split_once(':') {
                            return Self::clean_cpu_name(v);
                        }
                    }
                }
            }
        }
        "Unknown CPU".to_string()
    }

    /// Trim marketing cruft so the name fits the sidebar:
    /// "AMD Ryzen 5 9600X 6-Core Processor" → "AMD Ryzen 5 9600X".
    fn clean_cpu_name(raw: &str) -> String {
        let mut s = raw.to_string();
        for junk in ["(R)", "(TM)", "(r)", "(tm)"] {
            s = s.replace(junk, "");
        }
        // Drop everything from a core-count / "Processor" / "CPU @" marker on.
        let lower = s.to_lowercase();
        for marker in [" with ", " w/ ", " cpu", " processor"] {
            if let Some(pos) = lower.find(marker) {
                s.truncate(pos);
                break;
            }
        }
        // Strip a trailing "N-Core" if it survived.
        if let Some(pos) = s.to_lowercase().find("-core") {
            // back up to the start of the number before "-core"
            let head = &s[..pos];
            if let Some(sp) = head.rfind(' ') {
                s.truncate(sp);
            }
        }
        s.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    /// Instantaneous CPU load, derived from the delta between successive
    /// `GetSystemTimes` reads. The first call returns 0 (no baseline yet);
    /// subsequent polls return a real figure.
    fn cpu_usage() -> f32 {
        #[cfg(target_os = "windows")]
        {
            use lazy_static::lazy_static;
            use std::sync::Mutex;
            use winapi::shared::minwindef::FILETIME;
            use winapi::um::processthreadsapi::GetSystemTimes;

            lazy_static! {
                // (idle, kernel, user) totals from the previous sample.
                static ref PREV: Mutex<Option<(u64, u64, u64)>> = Mutex::new(None);
            }

            fn ft_to_u64(ft: FILETIME) -> u64 {
                ((ft.dwHighDateTime as u64) << 32) | (ft.dwLowDateTime as u64)
            }

            unsafe {
                let mut idle = std::mem::zeroed::<FILETIME>();
                let mut kernel = std::mem::zeroed::<FILETIME>();
                let mut user = std::mem::zeroed::<FILETIME>();
                if GetSystemTimes(&mut idle, &mut kernel, &mut user) == 0 {
                    return 0.0;
                }
                let (idle, kernel, user) =
                    (ft_to_u64(idle), ft_to_u64(kernel), ft_to_u64(user));

                let mut prev = PREV.lock().unwrap();
                let usage = match *prev {
                    Some((p_idle, p_kernel, p_user)) => {
                        // Kernel time already includes idle time on Windows.
                        let idle_d = idle.saturating_sub(p_idle);
                        let total_d = (kernel.saturating_sub(p_kernel))
                            + (user.saturating_sub(p_user));
                        if total_d == 0 {
                            0.0
                        } else {
                            let busy = total_d.saturating_sub(idle_d) as f64;
                            ((busy / total_d as f64) * 100.0) as f32
                        }
                    }
                    None => 0.0,
                };
                *prev = Some((idle, kernel, user));
                return usage.clamp(0.0, 100.0);
            }
        }
        #[allow(unreachable_code)]
        {
            0.0
        }
    }

    // ── Memory ─────────────────────────────────────────────────────────

    /// Returns (used_gb, total_gb, load_percent).
    fn memory() -> (f32, f32, f32) {
        #[cfg(target_os = "windows")]
        {
            use winapi::um::sysinfoapi::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
            unsafe {
                let mut status = std::mem::zeroed::<MEMORYSTATUSEX>();
                status.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;
                if GlobalMemoryStatusEx(&mut status) != 0 {
                    let total = status.ullTotalPhys as f64;
                    let avail = status.ullAvailPhys as f64;
                    let used = total - avail;
                    let gib = 1024.0 * 1024.0 * 1024.0;
                    return (
                        (used / gib) as f32,
                        (total / gib) as f32,
                        status.dwMemoryLoad as f32,
                    );
                }
            }
        }
        #[cfg(target_os = "linux")]
        {
            if let Ok(content) = std::fs::read_to_string("/proc/meminfo") {
                let mut total_kb = 0.0f64;
                let mut avail_kb = 0.0f64;
                for line in content.lines() {
                    if let Some(v) = line.strip_prefix("MemTotal:") {
                        total_kb = v.trim().trim_end_matches(" kB").trim().parse().unwrap_or(0.0);
                    } else if let Some(v) = line.strip_prefix("MemAvailable:") {
                        avail_kb = v.trim().trim_end_matches(" kB").trim().parse().unwrap_or(0.0);
                    }
                }
                if total_kb > 0.0 {
                    let used_kb = total_kb - avail_kb;
                    let to_gib = 1024.0 * 1024.0;
                    let load = (used_kb / total_kb * 100.0) as f32;
                    return ((used_kb / to_gib) as f32, (total_kb / to_gib) as f32, load);
                }
            }
        }
        (0.0, 0.0, 0.0)
    }

    // ── GPU ────────────────────────────────────────────────────────────

    fn gpu_name() -> String {
        // The GPU name never changes during a session — resolve it once and cache
        // it so we don't re-scan the registry / spawn nvidia-smi on every poll.
        use lazy_static::lazy_static;
        use std::sync::Mutex;
        lazy_static! {
            static ref CACHE: Mutex<Option<String>> = Mutex::new(None);
        }
        if let Some(name) = CACHE.lock().unwrap().clone() {
            return name;
        }
        let name = Self::gpu_name_uncached();
        *CACHE.lock().unwrap() = Some(name.clone());
        name
    }

    fn gpu_name_uncached() -> String {
        // Prefer nvidia-smi (real physical GPU) on every platform — the Windows
        // registry can surface virtual display adapters (Virtual Desktop, IDD,
        // Parsec, RDP) ahead of the real card.
        if let Some(name) = Self::nvidia_smi_field("name") {
            return name;
        }

        #[cfg(target_os = "windows")]
        {
            use winreg::RegKey;
            // Junk adapters to skip when scanning the display class.
            const SKIP: [&str; 8] =
                ["virtual", "basic", "remote", "idd", "parsec", "citrix", "rdp", "meta"];
            if let Ok(class) = RegKey::predef(winreg::enums::HKEY_LOCAL_MACHINE).open_subkey(
                r"SYSTEM\CurrentControlSet\Control\Class\{4D36E968-E325-11CE-BFC1-08002BE10318}",
            ) {
                for key_name in class.enum_keys().flatten() {
                    if let Ok(sub) = class.open_subkey(&key_name) {
                        if let Ok(desc) = sub.get_value::<String, _>("DriverDesc") {
                            let d = desc.trim();
                            let lower = d.to_lowercase();
                            if !d.is_empty() && !SKIP.iter().any(|s| lower.contains(s)) {
                                return d.to_string();
                            }
                        }
                    }
                }
            }
        }
        "Unknown GPU".to_string()
    }

    /// GPU utilisation. Only NVIDIA via `nvidia-smi` is currently supported;
    /// returns `None` for everything else so the UI can render a dash.
    fn gpu_usage() -> Option<f32> {
        Self::nvidia_smi_field("utilization.gpu")
            .and_then(|s| s.trim().trim_end_matches('%').trim().parse::<f32>().ok())
    }

    /// Query a single `nvidia-smi` field for GPU 0. Returns `None` if the tool
    /// is absent or errors (i.e. no NVIDIA GPU / driver).
    ///
    /// The console window is suppressed (`CREATE_NO_WINDOW`) so polling never
    /// flashes a command prompt, and availability is cached: once a probe shows
    /// nvidia-smi is missing/unusable, we stop spawning it entirely.
    fn nvidia_smi_field(field: &str) -> Option<String> {
        use lazy_static::lazy_static;
        use std::sync::Mutex;
        lazy_static! {
            static ref NVIDIA_OK: Mutex<Option<bool>> = Mutex::new(None);
        }
        if let Some(false) = *NVIDIA_OK.lock().unwrap() {
            return None;
        }

        let mut cmd = std::process::Command::new("nvidia-smi");
        cmd.arg(format!("--query-gpu={}", field))
            .arg("--format=csv,noheader,nounits");
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }

        let out = match cmd.output() {
            Ok(o) => o,
            Err(_) => {
                *NVIDIA_OK.lock().unwrap() = Some(false);
                return None;
            }
        };
        if !out.status.success() {
            *NVIDIA_OK.lock().unwrap() = Some(false);
            return None;
        }
        *NVIDIA_OK.lock().unwrap() = Some(true);

        let text = String::from_utf8_lossy(&out.stdout);
        text.lines().next().map(|l| l.trim().to_string()).filter(|s| !s.is_empty())
    }
}
