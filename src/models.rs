use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct AppConfig {
    pub ytdlp_location: String,
    pub allowed_args: Vec<String>,
    pub custom_args: Vec<String>,
    pub cookies: bool,
    pub cookies_browser: String,
    #[serde(default)]
    pub logging: LoggingConfig,
    pub version: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct LoggingConfig {
    pub max_file_size_mb: u32,
    pub max_archived_logs: u32,
    pub debug_enabled: bool,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            max_file_size_mb: crate::constants::defaults::LOG_MAX_SIZE_MB,
            max_archived_logs: crate::constants::defaults::LOG_MAX_ARCHIVED,
            debug_enabled: false,
        }
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            ytdlp_location: "tools/ytdlpx.exe".to_string(),
            allowed_args: vec![
                "--get-url".to_string(),
            ],
            custom_args: vec![
                "--no-check-certificate".to_string(),
                "--no-warnings".to_string(),
                "--no-cache-dir".to_string(),
                "-f".to_string(),
                "best[height<=1080][protocol^=m3u8]/[protocol^=http]/[protocol^=m3u8]/best".to_string(),
            ],
            cookies: false,
            cookies_browser: "firefox".to_string(),
            logging: LoggingConfig::default(),
            version: "2".to_string(),
        }
    }
}

#[derive(Deserialize)]
pub struct GitHubRelease {
    pub tag_name: String,
    pub assets: Vec<GitHubAsset>,
}

#[derive(Deserialize)]
pub struct GitHubAsset {
    pub name: String,
    pub browser_download_url: String,
}

#[derive(Serialize, Deserialize, Default)]
pub struct VersionInfo {
    pub version: String,
    pub last_check: Option<DateTime<Utc>>,
}
