use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;
use chrono::Local;

pub struct Logger {
    log_file: Mutex<Option<File>>,
    log_path: PathBuf,
    buffer: Mutex<Vec<String>>,
}

impl Logger {
    pub fn new() -> Self {
        let log_dir = dirs::config_dir().map(|p| p.join("FallenWorldLauncher")).unwrap_or_else(|| PathBuf::from("."));
        let _ = fs::create_dir_all(&log_dir);
        let log_path = log_dir.join(format!("launcher_{}.log", Local::now().format("%Y%m%d_%H%M%S")));

        Logger {
            log_file: Mutex::new(None),
            log_path,
            buffer: Mutex::new(Vec::new()),
        }
    }

    pub fn init(&self) -> std::io::Result<()> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)?;

        *self.log_file.lock().unwrap() = Some(file);
        self.info("=== LAUNCHER STARTED ===");
        Ok(())
    }

    pub fn info(&self, message: &str) {
        self._log("INFO", message);
    }

    pub fn warn(&self, message: &str) {
        self._log("WARN", message);
    }

    pub fn error(&self, message: &str) {
        self._log("ERROR", message);
    }

    pub fn debug(&self, message: &str) {
        self._log("DEBUG", message);
    }

    fn _log(&self, level: &str, message: &str) {
        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let log_line = format!("[{}] {} - {}", timestamp, level, message);

        // Add to buffer
        if let Ok(mut buf) = self.buffer.lock() {
            buf.push(log_line.clone());
        }

        // Write to file
        if let Ok(mut file_opt) = self.log_file.lock() {
            if let Some(ref mut file) = *file_opt {
                let _ = writeln!(file, "{}", log_line);
                let _ = file.flush();
            }
        }

        // Also print to console
        println!("{}", log_line);
    }

    pub fn get_logs(&self) -> Vec<String> {
        self.buffer.lock().unwrap().clone()
    }

    pub fn clear_buffer(&self) {
        if let Ok(mut buf) = self.buffer.lock() {
            buf.clear();
        }
    }

    pub fn get_log_path(&self) -> String {
        self.log_path.to_string_lossy().to_string()
    }

    pub fn write_diagnostic(&self) -> std::io::Result<()> {
        self.info("=== DIAGNOSTIC TEST START ===");
        self.info(&format!("System Time: {}", Local::now()));
        self.info(&format!("Log Path: {}", self.get_log_path()));
        self.info("=== DIAGNOSTIC TEST END ===");
        Ok(())
    }
}

lazy_static::lazy_static! {
    pub static ref LOGGER: Logger = {
        let logger = Logger::new();
        let _ = logger.init();
        logger
    };
}

#[macro_export]
macro_rules! log_info {
    ($msg:expr) => {
        $crate::services::logger::LOGGER.info($msg)
    };
}

#[macro_export]
macro_rules! log_warn {
    ($msg:expr) => {
        $crate::services::logger::LOGGER.warn($msg)
    };
}

#[macro_export]
macro_rules! log_error {
    ($msg:expr) => {
        $crate::services::logger::LOGGER.error($msg)
    };
}

#[macro_export]
macro_rules! log_debug {
    ($msg:expr) => {
        $crate::services::logger::LOGGER.debug($msg)
    };
}
