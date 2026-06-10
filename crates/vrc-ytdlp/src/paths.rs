//! Executable-relative path resolution for the app dir and external tools.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};

use crate::config::Config;

/// Directory containing the running executable — the app's home for
/// config.json, logs, tools, and cache.
pub fn exe_dir() -> Result<PathBuf> {
    let exe = env::current_exe().context("cannot determine executable path")?;
    Ok(exe.parent().unwrap_or(Path::new(".")).to_path_buf())
}

/// Find a binary on PATH.
pub fn which(binary: &str) -> Option<PathBuf> {
    let path_var = env::var_os("PATH")?;
    for dir in env::split_paths(&path_var) {
        let candidate = dir.join(binary);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

pub fn resolve_ytdlp_path(config: &Config, app_dir: &Path) -> Result<PathBuf> {
    let loc = &config.ytdlp_location;

    let resolved = if Path::new(loc).is_absolute() {
        PathBuf::from(loc)
    } else {
        app_dir.join(loc.replace('/', std::path::MAIN_SEPARATOR_STR))
    };

    if let Ok(current_exe) = env::current_exe() {
        if resolved.exists() {
            let a = fs::canonicalize(&resolved).unwrap_or_else(|_| resolved.clone());
            let b = fs::canonicalize(&current_exe).unwrap_or(current_exe);
            if a == b {
                bail!(
                    "ytdlp_location ('{}') points to this executable. Configure a different path.",
                    loc
                );
            }
        }
    }

    Ok(resolved)
}

pub fn resolve_ffmpeg_path(config: &Config, app_dir: &Path) -> Result<PathBuf> {
    let loc = &config.ffmpeg_location;

    // If explicitly set and non-empty, resolve it
    if !loc.is_empty() {
        let resolved = if Path::new(loc).is_absolute() {
            PathBuf::from(loc)
        } else {
            app_dir.join(loc.replace('/', std::path::MAIN_SEPARATOR_STR))
        };

        if resolved.exists() {
            return Ok(resolved);
        }
    }

    // Fall back to PATH
    let name = if cfg!(windows) {
        "ffmpeg.exe"
    } else {
        "ffmpeg"
    };
    if let Some(path) = which(name) {
        return Ok(path);
    }

    bail!(
        "ffmpeg not found. Place it next to the executable, set ffmpeg_location in config.json, or install it on PATH."
    );
}
