use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::constants::CONFIG_FILE_NAME;
use crate::error::{AppError, Result};
use crate::models::AppConfig;

pub struct ConfigManager {
    config_path: PathBuf,
}

impl ConfigManager {
    pub fn new(app_dir: PathBuf) -> Self {
        let config_path = app_dir.join(CONFIG_FILE_NAME);
        Self { config_path }
    }

    pub fn load_config(&self) -> Result<AppConfig> {
        let default_config = AppConfig::default();

        if self.config_path.exists() {
            let content = fs::read_to_string(&self.config_path)?;

            let parsed_value: Value = match serde_json::from_str(&content) {
                Ok(v) => v,
                Err(_) => {
                    self.save_config(&default_config)?;
                    return Ok(default_config);
                }
            };

            let needs_upgrade = match parsed_value.get("version") {
                Some(v) => {
                    let existing_version = v.as_str().unwrap_or("");
                    version_is_less(existing_version, &default_config.version)
                }
                None => true,
            };

            if needs_upgrade {
                self.save_config(&default_config)?;
                return Ok(default_config);
            }

            let config: AppConfig = serde_json::from_str(&content)
                .map_err(|e| AppError::Config(format!("Failed to parse config.json: {}", e)))?;
            Ok(config)
        } else {
            self.save_config(&default_config)?;
            Ok(default_config)
        }
    }

    pub fn save_config(&self, config: &AppConfig) -> Result<()> {
        let config_json = serde_json::to_string_pretty(config)
            .map_err(|e| AppError::Config(format!("Failed to serialize config: {}", e)))?;

        fs::write(&self.config_path, config_json)
            .map_err(|e| AppError::Config(format!("Failed to write config file: {}", e)))?;

        Ok(())
    }

    pub fn get_ytdlp_path(&self, config: &AppConfig, app_dir: &Path) -> Result<PathBuf> {
        let ytdlp_location = &config.ytdlp_location;

        let resolved_path = if Path::new(ytdlp_location).is_absolute() {
            PathBuf::from(ytdlp_location)
        } else {
            let normalized_location = ytdlp_location.replace('/', std::path::MAIN_SEPARATOR_STR);
            app_dir.join(normalized_location)
        };

        if let Ok(current_exe) = std::env::current_exe() {
            if let (Some(current_name), Some(target_name)) = (current_exe.file_name(), resolved_path.file_name()) {
                let names_match = if cfg!(windows) {
                    current_name.to_string_lossy().eq_ignore_ascii_case(&target_name.to_string_lossy())
                } else {
                    current_name == target_name
                };
                if names_match {
                    return Err(AppError::Config(format!(
                        "ytdlp_location ('{}') points to the parent executable. Configure a different path (e.g. tools/yt-dlp.exe).",
                        ytdlp_location
                    )));
                }
            }
        }

        Ok(resolved_path)
    }
}

fn version_is_less(existing: &str, target: &str) -> bool {
    if existing == target { return false; }

    if let (Ok(e_int), Ok(t_int)) = (existing.parse::<u64>(), target.parse::<u64>()) {
        return e_int < t_int;
    }

    let e_parts: Vec<&str> = existing.split('.').collect();
    let t_parts: Vec<&str> = target.split('.').collect();
    for i in 0..t_parts.len() {
        let e_val = e_parts.get(i).and_then(|s| s.parse::<u64>().ok()).unwrap_or(0);
        let t_val = t_parts.get(i).and_then(|s| s.parse::<u64>().ok()).unwrap_or(0);
        if e_val < t_val { return true; }
        if e_val > t_val { return false; }
    }
    
    e_parts.len() < t_parts.len()
}