//! Windows system tasks the launcher performs on the user's behalf.
//! Currently: pagefile (virtual memory) configuration — port of the Python
//! FOMOD wizard's pagefile step.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PagefileInfo {
    pub ram_mb: u64,
    pub recommended_mb: u64,
    pub drives: Vec<String>,
    pub install_drive: String,
}

pub struct SystemTasks;

impl SystemTasks {
    /// Gather info for the pagefile UI: total RAM, recommended size (1.5× RAM),
    /// available fixed drives, and the drive the modlist lives on.
    pub fn pagefile_info(install_drive: &str) -> PagefileInfo {
        let ram_mb = Self::total_ram_mb();
        let recommended_mb = (ram_mb as f64 * 1.5) as u64;
        PagefileInfo {
            ram_mb,
            recommended_mb,
            drives: Self::fixed_drives(),
            install_drive: install_drive.to_string(),
        }
    }

    fn total_ram_mb() -> u64 {
        #[cfg(target_os = "windows")]
        {
            use winapi::um::sysinfoapi::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
            unsafe {
                let mut s = std::mem::zeroed::<MEMORYSTATUSEX>();
                s.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;
                if GlobalMemoryStatusEx(&mut s) != 0 {
                    return s.ullTotalPhys / (1024 * 1024);
                }
            }
        }
        16384
    }

    #[cfg(target_os = "windows")]
    fn fixed_drives() -> Vec<String> {
        let mut drives = Vec::new();
        for letter in b'C'..=b'Z' {
            let root = format!("{}:\\", letter as char);
            if std::path::Path::new(&root).exists() {
                drives.push(format!("{}:", letter as char));
            }
        }
        if drives.is_empty() {
            drives.push("C:".to_string());
        }
        drives
    }

    #[cfg(not(target_os = "windows"))]
    fn fixed_drives() -> Vec<String> {
        Vec::new()
    }

    /// Configure the Windows pagefile on `drive` to a fixed `target_mb` size.
    /// Runs an **elevated** PowerShell script (UAC prompt) because pagefile
    /// settings live under HKLM. Returns Ok on success, Err with the reason.
    #[cfg(target_os = "windows")]
    pub fn configure_pagefile(drive: &str, target_mb: u64) -> Result<(), String> {
        use std::io::Write;

        let drive = drive.trim_end_matches('\\');
        // Build the elevated worker script. It disables automatic management,
        // sets a fixed pagefile on the chosen drive, and writes a sentinel.
        let sentinel = std::env::temp_dir().join("fwl_pagefile_result.txt");
        let sentinel_str = sentinel.to_string_lossy();
        let script = format!(
            r#"
$ErrorActionPreference = 'Stop'
try {{
  $cs = Get-CimInstance Win32_ComputerSystem
  if ($cs.AutomaticManagedPagefile) {{
    Set-CimInstance -InputObject $cs -Property @{{ AutomaticManagedPagefile = $false }}
  }}
  $name = '{drive}\pagefile.sys'
  $existing = Get-CimInstance Win32_PageFileSetting | Where-Object {{ $_.Name -ieq $name }}
  if ($existing) {{
    Set-CimInstance -InputObject $existing -Property @{{ InitialSize = {mb}; MaximumSize = {mb} }}
  }} else {{
    New-CimInstance -ClassName Win32_PageFileSetting -Property @{{ Name = $name; InitialSize = {mb}; MaximumSize = {mb} }} | Out-Null
  }}
  Set-Content -Path '{sentinel}' -Value 'OK'
}} catch {{
  Set-Content -Path '{sentinel}' -Value ("ERR: " + $_.Exception.Message)
}}
"#,
            drive = drive,
            mb = target_mb,
            sentinel = sentinel_str
        );

        // Write the worker to a temp .ps1 (avoids quoting nightmares).
        let ps1 = std::env::temp_dir().join("fwl_pagefile.ps1");
        let mut f = std::fs::File::create(&ps1).map_err(|e| format!("Cannot write script: {}", e))?;
        f.write_all(script.as_bytes()).map_err(|e| format!("Cannot write script: {}", e))?;
        let _ = std::fs::remove_file(&sentinel);

        // Launch elevated and wait. Start-Process -Verb RunAs triggers UAC.
        let launch = format!(
            "Start-Process powershell -Verb RunAs -WindowStyle Hidden -Wait -ArgumentList '-NoProfile','-ExecutionPolicy','Bypass','-File','{}'",
            ps1.to_string_lossy()
        );
        let out = std::process::Command::new("powershell")
            .args(["-NoProfile", "-Command", &launch])
            .output()
            .map_err(|e| format!("Cannot launch elevated PowerShell: {}", e))?;
        let _ = std::fs::remove_file(&ps1);

        let result = std::fs::read_to_string(&sentinel).unwrap_or_default();
        let _ = std::fs::remove_file(&sentinel);

        if result.trim() == "OK" {
            Ok(())
        } else if let Some(err) = result.strip_prefix("ERR:") {
            Err(err.trim().to_string())
        } else if !out.status.success() {
            Err("Elevation was cancelled or PowerShell failed.".to_string())
        } else {
            Err("Pagefile change could not be confirmed (no result written).".to_string())
        }
    }

    #[cfg(not(target_os = "windows"))]
    pub fn configure_pagefile(_drive: &str, _target_mb: u64) -> Result<(), String> {
        Err("Pagefile configuration is Windows-only; Linux manages swap separately.".to_string())
    }

    /// Add a Windows Defender exclusion for `path` via an elevated PowerShell call.
    /// Returns the excluded path on success, or an error message on failure.
    #[cfg(target_os = "windows")]
    pub fn add_defender_exclusion(path: &str) -> Result<String, String> {
        use std::io::Write;
        let sentinel = std::env::temp_dir().join("fwl_av_result.txt");
        let sentinel_str = sentinel.to_string_lossy();
        let safe_path = path.replace('\'', "''");
        let script = format!(
            r#"
$ErrorActionPreference = 'Stop'
try {{
    Add-MpPreference -ExclusionPath '{path}' -ErrorAction Stop
    Set-Content -Path '{sentinel}' -Value 'OK'
}} catch {{
    Set-Content -Path '{sentinel}' -Value ("ERR: " + $_.Exception.Message)
}}
"#,
            path = safe_path,
            sentinel = sentinel_str
        );

        let ps1 = std::env::temp_dir().join("fwl_av.ps1");
        let mut f = std::fs::File::create(&ps1).map_err(|e| format!("Cannot write script: {}", e))?;
        f.write_all(script.as_bytes()).map_err(|e| format!("Cannot write script: {}", e))?;
        let _ = std::fs::remove_file(&sentinel);

        let launch = format!(
            "Start-Process powershell -Verb RunAs -WindowStyle Hidden -Wait -ArgumentList '-NoProfile','-ExecutionPolicy','Bypass','-File','{}'",
            ps1.to_string_lossy()
        );
        let _ = std::process::Command::new("powershell")
            .args(["-NoProfile", "-Command", &launch])
            .output();
        let _ = std::fs::remove_file(&ps1);

        let result = std::fs::read_to_string(&sentinel).unwrap_or_default();
        let _ = std::fs::remove_file(&sentinel);

        if result.trim() == "OK" {
            Ok(path.to_string())
        } else if let Some(err) = result.strip_prefix("ERR:") {
            Err(err.trim().to_string())
        } else {
            Err("Exclusion not confirmed (UAC may have been cancelled).".to_string())
        }
    }

    #[cfg(not(target_os = "windows"))]
    pub fn add_defender_exclusion(_path: &str) -> Result<String, String> {
        Err("Windows Defender exclusions are Windows-only.".to_string())
    }
}
