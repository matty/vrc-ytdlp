//! proxy requests from vrchat to yt-dlp

use std::env;
use std::path::{Path, PathBuf};

mod args;
mod config;
mod constants;
mod downloader;
mod error;
mod executor;
mod logger;
mod models;

use args::ArgumentParser;
use config::ConfigManager;
use downloader::Downloader;
use error::Result;
use executor::Executor;
use logger::{LogConfig, Logger};

#[tokio::main]
async fn main() -> Result<()> {
    let runtime_config = RuntimeConfig::from_env()?;

    // Load application configuration first to get logging settings
    let config_manager = ConfigManager::new(runtime_config.app_dir.clone());
    let app_config = config_manager.load_config()?;

    // Create logger with configuration from app config
    let log_config = LogConfig::from(&app_config.logging);
    let logger = Logger::with_config(runtime_config.log_path.clone(), log_config);

    // Log startup information
    logger.log_info(&format!("Arguments: {:?}", runtime_config.original_args));
    logger.log_info(&format!("Current executable: {:?}", env::args().next().unwrap_or_else(|| "unknown".to_string())));
    logger.log_info(&format!("Current working directory: {:?}", runtime_config.app_dir));

    // Log configuration information
    let log_info = logger.get_log_info();
    logger.log_debug(&format!("Log file: {}", log_info.current_log_path.display()));
    logger.log_debug(&format!("Current log size: {} bytes", log_info.current_size));
    logger.log_debug(&format!("Max log size: {} bytes", log_info.max_size));
    logger.log_debug(&format!("Archived logs: {}", log_info.archived_logs.len()));

    if log_info.is_near_rotation() {
        logger.log_warning("Log file is approaching rotation threshold");
    }

    logger.log_info(&format!("yt-dlp location: {}", app_config.ytdlp_location));

    // Get the full path to yt-dlp executable
    let ytdlp_path = config_manager.get_ytdlp_path(&app_config, &runtime_config.app_dir);
    logger.log_info(&format!("yt-dlp full path: {}", ytdlp_path.display()));

    // Create downloader with same logger configuration
    let downloader_logger = Logger::with_config(runtime_config.log_path.clone(), log_config);
    let downloader = Downloader::new(ytdlp_path.clone(), downloader_logger);

    // Ensure yt-dlp is available and up-to-date
    if !downloader.executable_exists() {
        logger.log_info(&format!("{} not found, downloading...", ytdlp_path.display()));
        downloader.download_latest().await?;
    } else {
        if let Err(e) = downloader.check_and_update().await {
            logger.log_error(&format!("Failed to check for updates: {}", e));
            // Continue anyway, we have a working version
        }
    }

    // Build complete argument list for yt-dlp
    let yt_dlp_args = if app_config.logging.debug_enabled {
        ArgumentParser::filter_arguments_with_logger(&runtime_config.yt_dlp_args, &app_config, Some(&logger))
    } else {
        ArgumentParser::filter_arguments(&runtime_config.yt_dlp_args, &app_config)
    };

    // Execute yt-dlp with process isolation
    let executor = Executor::new(runtime_config.app_dir, logger);
    let executable_path = downloader.get_executable_path();
    let result = executor.execute(&executable_path, &yt_dlp_args);

    // Log completion
    match &result {
        Ok(_) => executor.logger.log_info("Success"),
        Err(e) => executor.logger.log_error(&format!("Failed: {}", e)),
    }

    result
}

/// Runtime configuration derived from environment
struct RuntimeConfig {
    original_args: Vec<String>,
    yt_dlp_args: Vec<String>,
    app_dir: PathBuf,
    log_path: PathBuf,
}

impl RuntimeConfig {
    /// Creates configuration from environment
    fn from_env() -> Result<Self> {
        let args: Vec<String> = env::args().collect();

        let app_dir = match env::current_exe() {
            Ok(exe_path) => exe_path.parent().unwrap_or_else(|| Path::new(".")).to_path_buf(),
            Err(_) => env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        };

        // Create log file path in the same directory as the executable
        let log_path = app_dir.join("logs.log");

        // Skip the first argument (program name)
        let yt_dlp_args = if args.len() > 1 {
            args.iter().skip(1).cloned().collect::<Vec<String>>()
        } else {
            Vec::new()
        };

        Ok(Self {
            original_args: args,
            yt_dlp_args,
            app_dir,
            log_path,
        })
    }
}

fn print_help() {
    println!("Usage: yt-dlp-proxy [yt-dlp arguments...]");
}
