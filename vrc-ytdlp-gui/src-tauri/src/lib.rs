mod commands;
mod config;
mod paths;

use commands::{cache, config_cmd, cookies, logs, server, updates};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            // Config
            config_cmd::get_config,
            config_cmd::save_config,
            config_cmd::config_exists,
            config_cmd::get_default_config,
            // Server
            server::check_server_health,
            server::start_server,
            server::stop_server,
            server::get_server_pid,
            // Cache
            cache::scan_cache,
            cache::delete_cache_entry,
            cache::clear_cache,
            // Logs
            logs::read_logs,
            // Updates
            updates::get_version_info,
            updates::check_for_update,
            updates::download_ytdlp,
            // Cookies
            cookies::check_cookies,
            cookies::extract_cookies,
            cookies::get_browsers,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
