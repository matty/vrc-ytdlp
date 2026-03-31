use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

// --- Default functions (mirror vrc-ytdlp exactly) ---

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

fn default_server_port() -> u16 {
    9851
}

fn default_server_idle_timeout_secs() -> u64 {
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

// --- Config struct ---

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

// --- Validation ---

pub type ValidationErrors = Vec<(String, String)>;

impl Config {
    pub fn validate(&self) -> ValidationErrors {
        let mut errors = Vec::new();

        if self.server_port == 0 {
            errors.push(("server_port".into(), "must not be zero".into()));
        }
        if self.bgutil_pot_port == 0 {
            errors.push(("bgutil_pot_port".into(), "must not be zero".into()));
        }
        if self.server_port != 0
            && self.bgutil_pot_port != 0
            && self.server_port == self.bgutil_pot_port
        {
            errors.push((
                "server_port".into(),
                "must not be the same as bgutil_pot_port".into(),
            ));
        }
        if self.cache_max_size_mb == 0 {
            errors.push(("cache_max_size_mb".into(), "must not be zero".into()));
        }
        if self.execution_timeout_secs == 0 {
            errors.push(("execution_timeout_secs".into(), "must not be zero".into()));
        }

        errors
    }
}

// --- Load / Save ---

pub fn load_config(path: &Path) -> Result<Config> {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Config::default());
        }
        Err(e) => return Err(e).context("reading config file"),
    };

    let config: Config = serde_json::from_str(&content).context("parsing config file")?;
    Ok(config)
}

pub fn save_config(path: &Path, config: &Config) -> Result<()> {
    let json = serde_json::to_string_pretty(config).context("serializing config")?;

    // Atomic write: write to a temp file alongside the target, then rename
    let parent = path.parent().unwrap_or(Path::new("."));
    let tmp_path = parent.join(format!(
        ".config.tmp.{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(0)
    ));

    fs::write(&tmp_path, &json).context("writing temp config file")?;
    fs::rename(&tmp_path, path).context("renaming temp config to final path")?;

    Ok(())
}

pub fn config_exists(path: &Path) -> bool {
    path.exists()
}

// --- Tests ---

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn default_config_roundtrips() {
        let config = Config::default();
        let json = serde_json::to_string_pretty(&config).expect("serialize");
        let loaded: Config = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(config, loaded);
    }

    #[test]
    fn missing_fields_get_defaults() {
        let json = r#"{"cookies": true}"#;
        let config: Config = serde_json::from_str(json).expect("deserialize");
        let defaults = Config::default();

        assert!(config.cookies);
        assert_eq!(config.ytdlp_location, defaults.ytdlp_location);
        assert_eq!(config.ffmpeg_location, defaults.ffmpeg_location);
        assert_eq!(config.allowed_args, defaults.allowed_args);
        assert_eq!(config.custom_args, defaults.custom_args);
        assert_eq!(config.cookies_browser, defaults.cookies_browser);
        assert_eq!(config.execution_timeout_secs, defaults.execution_timeout_secs);
        assert_eq!(config.update_check_days, defaults.update_check_days);
        assert_eq!(config.server_port, defaults.server_port);
        assert_eq!(config.server_idle_timeout_secs, defaults.server_idle_timeout_secs);
        assert_eq!(config.bgutil_pot_port, defaults.bgutil_pot_port);
        assert_eq!(config.cache_dir, defaults.cache_dir);
        assert_eq!(config.cache_max_size_mb, defaults.cache_max_size_mb);
        assert_eq!(config.cache_ttl_secs, defaults.cache_ttl_secs);
    }

    #[test]
    fn validate_catches_zero_port() {
        let mut config = Config::default();
        config.server_port = 0;
        let errors = config.validate();
        assert!(
            errors.iter().any(|(field, _)| field == "server_port"),
            "expected server_port error, got: {errors:?}"
        );
    }

    #[test]
    fn validate_catches_duplicate_ports() {
        let mut config = Config::default();
        config.server_port = 9851;
        config.bgutil_pot_port = 9851;
        let errors = config.validate();
        assert!(
            errors.iter().any(|(field, _)| field == "server_port"),
            "expected duplicate port error, got: {errors:?}"
        );
    }

    #[test]
    fn validate_passes_for_defaults() {
        let config = Config::default();
        let errors = config.validate();
        assert!(errors.is_empty(), "expected no errors, got: {errors:?}");
    }

    #[test]
    fn load_missing_file_returns_default() {
        let path = PathBuf::from("/tmp/vrc-ytdlp-gui-test-nonexistent-config.json");
        // Ensure it doesn't exist
        let _ = fs::remove_file(&path);

        let config = load_config(&path).expect("load should succeed");
        assert_eq!(config, Config::default());
    }

    #[test]
    fn save_and_load_roundtrip() {
        let dir = std::env::temp_dir();
        let path = dir.join(format!(
            "vrc-ytdlp-gui-test-config-{}.json",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));

        let mut config = Config::default();
        config.cookies = true;
        config.server_port = 12345;

        save_config(&path, &config).expect("save");
        let loaded = load_config(&path).expect("load");
        assert_eq!(config, loaded);

        // Cleanup
        let _ = fs::remove_file(&path);
    }
}
