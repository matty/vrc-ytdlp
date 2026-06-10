use crate::config::{self, Config};
use crate::paths;

#[tauri::command]
pub fn get_config() -> Result<Config, String> {
    let path = paths::config_path().map_err(|e| e.to_string())?;
    config::load_config(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_config(cfg: Config) -> Result<(), String> {
    let path = paths::config_path().map_err(|e| e.to_string())?;
    config::save_config(&path, &cfg).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn config_exists() -> Result<bool, String> {
    let path = paths::config_path().map_err(|e| e.to_string())?;
    Ok(config::config_exists(&path))
}

#[tauri::command]
pub fn get_default_config() -> Config {
    Config::default()
}
