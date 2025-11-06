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
    
    let early_app_dir = env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."));
    let early_log_path = early_app_dir.join("logs.log");
    let early_logger = Logger::with_config(early_log_path, LogConfig::default());

    let runtime_config = match RuntimeConfig::from_env() {
        Ok(config) => config,
        Err(e) => {
            early_logger.log_error(&format!("Failed to initialize runtime config: {}", e));
            return Ok(());
        }
    };

    let config_manager = ConfigManager::new(runtime_config.app_dir.clone());
    let app_config = match config_manager.load_config() {
        Ok(config) => config,
        Err(e) => {
            early_logger.log_error(&format!("Failed to load config: {}", e));
            return Ok(());
        }
    };

    let log_config = LogConfig::from(&app_config.logging);
    let logger = Logger::with_config(runtime_config.log_path.clone(), log_config);

    let log_info = logger.get_log_info();
    logger.log_debug(&format!("Log file: {}", log_info.current_log_path.display()));
    logger.log_debug(&format!("Current log size: {} bytes", log_info.current_size));
    logger.log_debug(&format!("Max log size: {} bytes", log_info.max_size));
    logger.log_debug(&format!("Archived logs: {}", log_info.archived_logs.len()));

    if log_info.is_near_rotation() {
        logger.log_warning("Log file is approaching rotation threshold");
    }

    logger.log_info(&format!("yt-dlp location: {}", app_config.ytdlp_location));

    let ytdlp_path = match config_manager.get_ytdlp_path(&app_config, &runtime_config.app_dir) {
        Ok(path) => path,
        Err(e) => {
            logger.log_error(&format!("Failed to resolve yt-dlp path: {}", e));
            return Ok(());
        }
    };
    
    if ytdlp_path.as_os_str().is_empty() {
        return Ok(());
    }
    
    logger.log_info(&format!("yt-dlp full path: {}", ytdlp_path.display()));

    let downloader_logger = Logger::with_config(runtime_config.log_path.clone(), log_config);
    let downloader = Downloader::new(ytdlp_path.clone(), downloader_logger);

    if !downloader.executable_exists() {
        logger.log_info(&format!("{} not found, downloading...", ytdlp_path.display()));
        if let Err(e) = downloader.download_latest().await {
            logger.log_error(&format!("Failed to download yt-dlp: {}", e));
            return Ok(());
        }
    } else {
        if let Err(e) = downloader.check_and_update().await {
            logger.log_error(&format!("Failed to check for updates: {}", e));
        }
    }

    let yt_dlp_args = if app_config.logging.debug_enabled {
        ArgumentParser::filter_arguments_with_logger(&runtime_config.yt_dlp_args, &app_config, Some(&logger))
    } else {
        ArgumentParser::filter_arguments(&runtime_config.yt_dlp_args, &app_config)
    };

    logger.log_info(&format!("Arguments: {:?}", yt_dlp_args));

    let executor = Executor::new(runtime_config.app_dir, logger);
    let executable_path = downloader.get_executable_path();
    
    match executor.execute(&executable_path, &yt_dlp_args) {
        Ok(_) => executor.logger.log_info("Success"),
        Err(e) => executor.logger.log_error(&format!("Failed: {}", e)),
    }

    Ok(())
}

struct RuntimeConfig {
    yt_dlp_args: Vec<String>,
    app_dir: PathBuf,
    log_path: PathBuf,
}

impl RuntimeConfig {
    fn from_env() -> Result<Self> {
        let args: Vec<String> = env::args().collect();

        let app_dir = match env::current_exe() {
            Ok(exe_path) => exe_path.parent().unwrap_or_else(|| Path::new(".")).to_path_buf(),
            Err(_) => env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        };

        let log_path = app_dir.join("logs.log");

        let yt_dlp_args = if args.len() > 1 {
            args.iter().skip(1).cloned().collect::<Vec<String>>()
        } else {
            Vec::new()
        };

        Ok(Self {
            yt_dlp_args,
            app_dir,
            log_path,
        })
    }
}
