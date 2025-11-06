use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

use chrono::Local;

/// Configuration for log rotation
#[derive(Debug, Clone, Copy)]
pub struct LogConfig {
    /// Maximum size of a log file in bytes before rotation (default: 10MB)
    pub max_file_size: u64,
    /// Maximum number of archived log files to keep (default: 5)
    pub max_archived_logs: u32,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            max_file_size: 10 * 1024 * 1024, // 10MB
            max_archived_logs: 5,
        }
    }
}

impl From<&crate::models::LoggingConfig> for LogConfig {
    fn from(config: &crate::models::LoggingConfig) -> Self {
        Self {
            max_file_size: (config.max_file_size_mb as u64) * 1024 * 1024,
            max_archived_logs: config.max_archived_logs,
        }
    }
}

/// Logger for writing messages to a log file with rotation support
pub struct Logger {
    log_path: PathBuf,
    config: LogConfig,
}

impl Logger {


    /// Creates a new logger instance with custom configuration
    pub fn with_config(log_path: PathBuf, config: LogConfig) -> Self {
        Self { log_path, config }
    }

    /// Logs a message to the log file
    pub fn log(&self, message: &str) {
        self.write_log_entry(message);
    }

    /// Internal method to write log entries
    fn write_log_entry(&self, message: &str) {
        // Check if rotation is needed before writing
        self.rotate_if_needed();

        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let formatted_message = format!("[{}] {}\n", timestamp, message);

        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)
        {
            let _ = file.write_all(formatted_message.as_bytes());
            let _ = file.flush();
        }
    }

    /// Checks if log rotation is needed and performs it
    fn rotate_if_needed(&self) {
        if let Ok(metadata) = fs::metadata(&self.log_path) {
            if metadata.len() > self.config.max_file_size {
                self.rotate_logs();
            }
        }
    }

    /// Performs log rotation
    fn rotate_logs(&self) {
        // First, remove the oldest log if we're at the limit
        let oldest_log = self.get_archived_log_path(self.config.max_archived_logs);
        if oldest_log.exists() {
            let _ = fs::remove_file(&oldest_log);
        }

        // Shift all existing archived logs
        for i in (1..self.config.max_archived_logs).rev() {
            let current_log = self.get_archived_log_path(i);
            let next_log = self.get_archived_log_path(i + 1);

            if current_log.exists() {
                let _ = fs::rename(&current_log, &next_log);
            }
        }

        // Move the current log to .1
        let first_archive = self.get_archived_log_path(1);
        if self.log_path.exists() {
            let _ = fs::rename(&self.log_path, &first_archive);
        }

        // Log the rotation event in the new file
        self.log_internal("Log rotated due to size limit");
    }

    /// Gets the path for an archived log file
    fn get_archived_log_path(&self, index: u32) -> PathBuf {
        let file_name = self.log_path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("log.log");

        let archived_name = format!("{}.{}", file_name, index);
        self.log_path.with_file_name(archived_name)
    }

    /// Internal logging method that doesn't trigger rotation (to avoid infinite recursion)
    fn log_internal(&self, message: &str) {
        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let formatted_message = format!("[{}] {}\n", timestamp, message);

        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)
        {
            let _ = file.write_all(formatted_message.as_bytes());
            let _ = file.flush();
        }
    }

    /// Logs an error message
    pub fn log_error(&self, error: &str) {
        self.log(&format!("ERROR: {}", error));
    }

    /// Logs an info message
    pub fn log_info(&self, info: &str) {
        self.log(&format!("INFO: {}", info));
    }

    /// Logs debug information
    pub fn log_debug(&self, debug: &str) {
        self.log(&format!("DEBUG: {}", debug));
    }

    /// Logs a warning message
    pub fn log_warning(&self, warning: &str) {
        self.log(&format!("WARNING: {}", warning));
    }

    /// Gets the current log file size in bytes
    pub fn get_log_size(&self) -> u64 {
        fs::metadata(&self.log_path)
            .map(|m| m.len())
            .unwrap_or(0)
    }

    /// Gets information about log files and their sizes
    pub fn get_log_info(&self) -> LogInfo {
        let current_size = self.get_log_size();
        let mut archived_logs = Vec::new();

        for i in 1..=self.config.max_archived_logs {
            let archived_path = self.get_archived_log_path(i);
            if archived_path.exists() {
                if let Ok(metadata) = fs::metadata(&archived_path) {
                    archived_logs.push(ArchivedLogInfo {
                        path: archived_path,
                        size: metadata.len(),
                        index: i,
                    });
                }
            }
        }

        LogInfo {
            current_log_path: self.log_path.clone(),
            current_size,
            max_size: self.config.max_file_size,
            archived_logs,

        }
    }


}

#[derive(Debug)]
pub struct LogInfo {
    pub current_log_path: PathBuf,
    pub current_size: u64,
    pub max_size: u64,
    pub archived_logs: Vec<ArchivedLogInfo>,

}

#[derive(Debug)]
pub struct ArchivedLogInfo {
    pub path: PathBuf,
    pub size: u64,
    pub index: u32,
}

impl LogInfo {
    /// Checks if the current log is close to the rotation threshold
    pub fn is_near_rotation(&self) -> bool {
        let threshold = (self.max_size as f64 * 0.8) as u64; // 80% of max size
        self.current_size >= threshold
    }
}
