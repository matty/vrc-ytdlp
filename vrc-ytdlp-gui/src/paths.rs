use std::env;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

/// Directory containing the GUI executable.
pub fn exe_dir() -> Result<PathBuf> {
    let exe = env::current_exe().context("cannot determine executable path")?;
    Ok(exe.parent().unwrap_or(Path::new(".")).to_path_buf())
}

/// Path to config.json next to the executable.
pub fn config_path() -> Result<PathBuf> {
    Ok(exe_dir()?.join("config.json"))
}

/// Path to the server PID file.
pub fn pid_file_path() -> Result<PathBuf> {
    Ok(exe_dir()?.join("server.pid"))
}

/// Path to version.txt for yt-dlp.
pub fn version_file_path(ytdlp_path: &Path) -> PathBuf {
    ytdlp_path
        .parent()
        .unwrap_or(Path::new("."))
        .join("version.txt")
}

/// Resolve a potentially relative path against exe_dir.
pub fn resolve_path(path: &str) -> Result<PathBuf> {
    if Path::new(path).is_absolute() {
        Ok(PathBuf::from(path))
    } else {
        Ok(exe_dir()?.join(path))
    }
}
