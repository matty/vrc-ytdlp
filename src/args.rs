use crate::models::AppConfig;
use crate::logger::Logger;

pub struct ArgumentParser;

impl ArgumentParser {
    pub fn filter_arguments(args: &[String], config: &AppConfig) -> Vec<String> {
        Self::filter_arguments_with_logger(args, config, None)
    }

    /// Builds the complete argument list with optional logging
    pub fn filter_arguments_with_logger(
        args: &[String], 
        config: &AppConfig, 
        logger: Option<&Logger>
    ) -> Vec<String> {
        if let Some(logger) = logger {
            logger.log_debug("Building yt-dlp arguments");
            logger.log_debug(&format!("Input arguments ({}): {:?}", args.len(), args));
            logger.log_debug(&format!("Allowed args: {:?}", config.allowed_args));
        }

        // Step 1: Filter input arguments (only keep allowed ones)
        let mut yt_dlp_args = Vec::new();
        let mut i = 0;

        while i < args.len() {
            let current_arg = &args[i];

            // Check if this argument is in our allowed list
            if config.allowed_args.contains(current_arg) {
                if let Some(logger) = logger {
                    logger.log_debug(&format!("Keeping allowed arg: {}", current_arg));
                }
                yt_dlp_args.push(current_arg.clone());

                // Smart detection: if next arg does not start with '-' or '--', treat as value
                if i + 1 < args.len() {
                    let next_arg = &args[i + 1];
                    if !next_arg.starts_with('-') {
                        if let Some(logger) = logger {
                            logger.log_debug(&format!("Adding arg value: {}", next_arg));
                        }
                        yt_dlp_args.push(next_arg.clone());
                        i += 1; // Skip the next argument since we've already processed it
                    }
                }
            } else if let Some(logger) = logger {
                logger.log_debug(&format!("Removing disallowed arg: {}", current_arg));
            }

            i += 1;
        }

        if let Some(logger) = logger {
            logger.log_debug(&format!("After filtering: {} args kept", yt_dlp_args.len()));
        }

        // Step 2: Add custom args from config (always passed to yt-dlp)
        if !config.custom_args.is_empty() {
            if let Some(logger) = logger {
                logger.log_debug(&format!("Adding {} custom args from config: {:?}", config.custom_args.len(), config.custom_args));
            }
            for custom_arg in &config.custom_args {
                yt_dlp_args.push(custom_arg.clone());
            }
        } else if let Some(logger) = logger {
            logger.log_debug("No custom args in config");
        }

        // Step 3: Add cookies flag if enabled in config
        if config.cookies {
            let cookie_arg = format!("--cookies-from-browser={}", config.cookies_browser);
            if let Some(logger) = logger {
                logger.log_debug(&format!("Adding cookies arg: {}", cookie_arg));
            }
            yt_dlp_args.push(cookie_arg);
        } else if let Some(logger) = logger {
            logger.log_debug("Cookies disabled in config");
        }

        if let Some(logger) = logger {
            logger.log_debug(&format!("yt-dlp arguments ({})", yt_dlp_args.len()));
            for (i, arg) in yt_dlp_args.iter().enumerate() {
                logger.log_debug(&format!("  [{}] {}", i + 1, arg));
            }
        }

        yt_dlp_args
    }
}
