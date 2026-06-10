//! File-based tracing setup with daily rotation and old-log cleanup.

use std::fs;
use std::path::{Path, PathBuf};

/// Initialize tracing to a daily-rolling log file in `app_dir`.
/// The returned guard must be kept alive for the process lifetime.
pub fn setup_logging(app_dir: &Path) -> tracing_appender::non_blocking::WorkerGuard {
    let file_appender = tracing_appender::rolling::daily(app_dir, "vrc-ytdlp.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::fmt()
        .with_writer(non_blocking)
        .with_ansi(false)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    cleanup_old_logs(app_dir);
    guard
}

/// Keep only the three most recent rotated log files.
fn cleanup_old_logs(dir: &Path) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    let mut log_files: Vec<PathBuf> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with("vrc-ytdlp.log."))
                .unwrap_or(false)
        })
        .collect();

    log_files.sort();

    if log_files.len() > 3 {
        for old in &log_files[..log_files.len() - 3] {
            let _ = fs::remove_file(old);
        }
    }
}
