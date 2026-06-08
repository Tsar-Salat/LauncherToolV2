//! Platform abstraction — single source of truth for OS-specific behaviour.
//!
//! Port of the Python `platform_compat.py`. Callers use these named helpers
//! instead of sprinkling `cfg!(windows)` branches at every call site.

use std::path::{Path, PathBuf};
use std::process::Command;
#[cfg(unix)]
use std::process::Stdio;

pub const IS_WINDOWS: bool = cfg!(target_os = "windows");
pub const IS_LINUX: bool = cfg!(target_os = "linux");

pub struct Platform;

impl Platform {
    /// Spawn a process that survives this launcher exiting.
    ///
    /// Windows: `DETACHED_PROCESS | CREATE_NO_WINDOW`.
    /// POSIX: a new session (setsid) with stdio detached from the terminal.
    pub fn spawn_detached(program: &str, args: &[String], cwd: Option<&Path>) -> Result<u32, String> {
        let mut cmd = Command::new(program);
        cmd.args(args);
        if let Some(dir) = cwd {
            cmd.current_dir(dir);
        }

        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            const DETACHED_PROCESS: u32 = 0x0000_0008;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            cmd.creation_flags(DETACHED_PROCESS | CREATE_NO_WINDOW);
        }

        #[cfg(unix)]
        {
            cmd.stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null());
            // start_new_session is stable via CommandExt on unix.
            use std::os::unix::process::CommandExt as _;
            unsafe {
                cmd.pre_exec(|| {
                    libc_setsid();
                    Ok(())
                });
            }
        }

        cmd.spawn()
            .map(|child| child.id())
            .map_err(|e| format!("Failed to spawn {}: {}", program, e))
    }

    /// Open a URL or file path with the OS default handler.
    pub fn open_external(target: &str) -> Result<(), String> {
        #[cfg(target_os = "windows")]
        let result = Command::new("cmd")
            .args(["/C", "start", "", target])
            .creation_flags_noop()
            .spawn();
        #[cfg(target_os = "macos")]
        let result = Command::new("open").arg(target).spawn();
        #[cfg(all(unix, not(target_os = "macos")))]
        let result = Command::new("xdg-open").arg(target).spawn();

        result
            .map(|_| ())
            .map_err(|e| format!("Failed to open {}: {}", target, e))
    }

    /// Per-user log/state directory, created on demand.
    /// Windows: `%LOCALAPPDATA%\<app>\logs`. Linux: `$XDG_STATE_HOME/<app>`.
    pub fn user_log_dir(app: &str) -> PathBuf {
        let path = if cfg!(target_os = "windows") {
            let root = std::env::var("LOCALAPPDATA")
                .map(PathBuf::from)
                .unwrap_or_else(|_| Self::home().join("AppData").join("Local"));
            root.join(app).join("logs")
        } else {
            let root = std::env::var("XDG_STATE_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|_| Self::home().join(".local").join("state"));
            root.join(app)
        };
        let _ = std::fs::create_dir_all(&path);
        path
    }

    /// Filename of the launcher binary asset published in GitHub releases.
    pub fn release_asset_name() -> &'static str {
        if cfg!(target_os = "windows") {
            "Launcher.exe"
        } else {
            "Launcher"
        }
    }

    pub fn home() -> PathBuf {
        dirs::home_dir().unwrap_or_else(|| PathBuf::from("."))
    }
}

// Tiny shim so the Windows `start` path compiles without pulling
// CommandExt into scope on non-Windows targets. Public so other services can
// suppress the console window on short-lived helper processes (curl, etc.).
pub trait CreationFlagsNoop {
    fn creation_flags_noop(&mut self) -> &mut Self;
}
impl CreationFlagsNoop for Command {
    #[cfg(target_os = "windows")]
    fn creation_flags_noop(&mut self) -> &mut Self {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        self.creation_flags(CREATE_NO_WINDOW)
    }
    #[cfg(not(target_os = "windows"))]
    fn creation_flags_noop(&mut self) -> &mut Self {
        self
    }
}

#[cfg(unix)]
fn libc_setsid() {
    // Avoid a libc dependency: call setsid via the syscall wrapper exposed
    // by the C runtime. SAFETY: setsid takes no args and only fails when the
    // caller is already a process-group leader, which a freshly forked child
    // (post-fork, pre-exec) is not.
    extern "C" {
        fn setsid() -> i32;
    }
    unsafe {
        let _ = setsid();
    }
}
