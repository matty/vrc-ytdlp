//! `config.json` schema, defaults, and load/save with legacy migration.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

fn default_ytdlp_location() -> String {
    if cfg!(windows) {
        "tools/yt-dlp.exe".into()
    } else {
        "./yt-dlp".into()
    }
}
fn default_ffmpeg_location() -> String {
    if cfg!(windows) {
        "tools/ffmpeg.exe".into()
    } else {
        "./ffmpeg".into()
    }
}
fn default_allowed_args() -> Vec<String> {
    vec!["--get-url".into()]
}
fn default_custom_args() -> Vec<String> {
    vec![
        "--no-check-certificate".into(),
        "--no-warnings".into(),
        "--no-cache-dir".into(),
        "-f".into(),
        "bv[vcodec^=avc][height<=1080]+ba[acodec^=mp4a]/bv[height<=1080]+ba/b[height<=1080]/b"
            .into(),
    ]
}
fn default_cookies_browser() -> String {
    "firefox".into()
}
fn default_execution_timeout_secs() -> u64 {
    120
}
fn default_update_check_days() -> u64 {
    1
}
pub(crate) fn default_server_port() -> u16 {
    9851
}
pub(crate) fn default_server_idle_timeout_secs() -> u64 {
    300
}
fn default_bgutil_pot_port() -> u16 {
    4416
}
fn default_cache_dir() -> String {
    "./cache".into()
}
fn default_cache_max_size_mb() -> u64 {
    2048
}
fn default_cache_ttl_secs() -> u64 {
    86400
}

#[derive(Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_ytdlp_location")]
    pub ytdlp_location: String,
    #[serde(default = "default_ffmpeg_location")]
    pub ffmpeg_location: String,
    #[serde(default = "default_allowed_args")]
    pub allowed_args: Vec<String>,
    #[serde(default = "default_custom_args")]
    pub custom_args: Vec<String>,
    #[serde(default)]
    pub cookies: bool,
    #[serde(default = "default_cookies_browser")]
    pub cookies_browser: String,
    #[serde(default = "default_execution_timeout_secs")]
    pub execution_timeout_secs: u64,
    #[serde(default = "default_update_check_days")]
    pub update_check_days: u64,
    #[serde(default = "default_server_port")]
    pub server_port: u16,
    #[serde(default = "default_server_idle_timeout_secs")]
    pub server_idle_timeout_secs: u64,
    /// Directory containing yt-dlp plugins (e.g., PO token provider)
    #[serde(default)]
    pub plugin_dirs: Option<String>,
    /// Extra yt-dlp extractor args (e.g., ["youtube:player-client=mweb"])
    #[serde(default)]
    pub extractor_args: Vec<String>,
    /// Port for the bgutil-pot PO token server
    #[serde(default = "default_bgutil_pot_port")]
    pub bgutil_pot_port: u16,
    /// Directory for cached videos
    #[serde(default = "default_cache_dir")]
    pub cache_dir: String,
    /// Maximum cache size in MB
    #[serde(default = "default_cache_max_size_mb")]
    pub cache_max_size_mb: u64,
    /// Cache entry time-to-live in seconds
    #[serde(default = "default_cache_ttl_secs")]
    pub cache_ttl_secs: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            ytdlp_location: default_ytdlp_location(),
            ffmpeg_location: default_ffmpeg_location(),
            allowed_args: default_allowed_args(),
            custom_args: default_custom_args(),
            cookies: false,
            cookies_browser: default_cookies_browser(),
            execution_timeout_secs: default_execution_timeout_secs(),
            update_check_days: default_update_check_days(),
            server_port: default_server_port(),
            server_idle_timeout_secs: default_server_idle_timeout_secs(),
            plugin_dirs: None,
            extractor_args: Vec::new(),
            bgutil_pot_port: default_bgutil_pot_port(),
            cache_dir: default_cache_dir(),
            cache_max_size_mb: default_cache_max_size_mb(),
            cache_ttl_secs: default_cache_ttl_secs(),
        }
    }
}

/// Load the config, writing defaults if the file is missing or invalid.
pub fn load_config(path: &Path) -> Result<Config> {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            let config = Config::default();
            write_config(path, &config)?;
            return Ok(config);
        }
        Err(e) => return Err(e).context("reading config file"),
    };

    let mut config: Config = match serde_json::from_str(&content) {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!("invalid config, resetting to defaults: {e}");
            let config = Config::default();
            write_config(path, &config)?;
            return Ok(config);
        }
    };

    // Migrate old allowed_args_with_values format
    if let Ok(raw) = serde_json::from_str::<serde_json::Value>(&content) {
        if let Some(arr) = raw
            .get("allowed_args_with_values")
            .and_then(|v| v.as_array())
        {
            for val in arr {
                if let Some(s) = val.as_str() {
                    let entry = if s.ends_with('=') {
                        s.to_string()
                    } else {
                        format!("{}=", s)
                    };
                    if !config.allowed_args.contains(&entry) {
                        config.allowed_args.push(entry);
                    }
                }
            }
        }
    }

    Ok(config)
}

pub fn write_config(path: &Path, config: &Config) -> Result<()> {
    let json = serde_json::to_string_pretty(config)?;
    fs::write(path, json).context("writing config file")
}
