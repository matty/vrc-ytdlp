use std::env;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};

/// The VRChat Tools directory where vrc-ytdlp, config, and binaries live.
/// On Windows: `%USERPROFILE%\AppData\LocalLow\VRChat\VRChat\Tools`
/// On other platforms (dev): falls back to the directory containing the GUI executable.
pub fn app_dir() -> Result<PathBuf> {
    #[cfg(windows)]
    {
        let user_profile = env::var("USERPROFILE")
            .context("USERPROFILE environment variable not set")?;
        let path = PathBuf::from(user_profile)
            .join("AppData")
            .join("LocalLow")
            .join("VRChat")
            .join("VRChat")
            .join("Tools");
        if !path.exists() {
            std::fs::create_dir_all(&path)
                .context("creating VRChat Tools directory")?;
        }
        Ok(path)
    }

    #[cfg(not(windows))]
    {
        // Dev fallback: use the directory containing the executable
        let exe = env::current_exe().context("cannot determine executable path")?;
        Ok(exe.parent().unwrap_or(Path::new(".")).to_path_buf())
    }
}

/// Path to config.json in the app directory.
pub fn config_path() -> Result<PathBuf> {
    Ok(app_dir()?.join("config.json"))
}

/// Path to the server PID file.
pub fn pid_file_path() -> Result<PathBuf> {
    Ok(app_dir()?.join("server.pid"))
}

/// Path to version.txt for yt-dlp.
pub fn version_file_path(ytdlp_path: &Path) -> PathBuf {
    ytdlp_path
        .parent()
        .unwrap_or(Path::new("."))
        .join("version.txt")
}

/// Resolve a potentially relative path against the app directory.
pub fn resolve_path(path: &str) -> Result<PathBuf> {
    if Path::new(path).is_absolute() {
        Ok(PathBuf::from(path))
    } else {
        Ok(app_dir()?.join(path))
    }
}
