use vrc_ytdlp::config::Config;

use super::CmdError;

/// Manager version for the status bar.
#[tauri::command]
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Default backend config, straight from the shared core type — the
/// Config screen's reset-to-defaults source.
#[tauri::command]
pub fn default_config() -> Result<Config, CmdError> {
    Ok(Config::default())
}
