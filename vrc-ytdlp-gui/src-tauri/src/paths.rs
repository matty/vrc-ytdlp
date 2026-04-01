use std::env;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

/// The VRChat Tools directory where vrc-ytdlp and its config live.
/// Windows: %USERPROFILE%\AppData\LocalLow\VRChat\VRChat\Tools
/// Other platforms (dev): directory containing the GUI executable.
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
        let exe = env::current_exe().context("cannot determine executable path")?;
        Ok(exe.parent().unwrap_or(Path::new(".")).to_path_buf())
    }
}

pub fn config_path() -> Result<PathBuf> {
    Ok(app_dir()?.join("config.json"))
}

pub fn pid_file_path() -> Result<PathBuf> {
    Ok(app_dir()?.join("server.pid"))
}

pub fn version_file_path(ytdlp_path: &Path) -> PathBuf {
    ytdlp_path
        .parent()
        .unwrap_or(Path::new("."))
        .join("version.txt")
}

pub fn resolve_path(path: &str) -> Result<PathBuf> {
    if Path::new(path).is_absolute() {
        Ok(PathBuf::from(path))
    } else {
        Ok(app_dir()?.join(path))
    }
}
