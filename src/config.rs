use std::fs;
use std::path::{Path, PathBuf};

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
        if self.config_path.exists() {
            let content = fs::read_to_string(&self.config_path)?;
            let config: AppConfig = serde_json::from_str(&content)
                .map_err(|e| AppError::Config(format!("Failed to parse config.json: {}", e)))?;
            Ok(config)
        } else {
            // Create default config file
            let default_config = AppConfig::default();
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

    pub fn get_ytdlp_path(&self, config: &AppConfig, app_dir: &Path) -> PathBuf {
        let ytdlp_location = &config.ytdlp_location;

        if Path::new(ytdlp_location).is_absolute() {
            PathBuf::from(ytdlp_location)
        } else {
            // Normalize path separators for the current platform
            let normalized_location = ytdlp_location.replace('/', std::path::MAIN_SEPARATOR_STR);
            app_dir.join(normalized_location)
        }
    }


}