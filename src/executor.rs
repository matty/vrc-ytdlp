#[cfg(windows)]
use std::os::windows::process::CommandExt;
use std::fs::create_dir_all;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::error::{AppError, Result};
use crate::logger::Logger;

pub struct Executor {
    exe_dir: PathBuf,
    pub logger: Logger,
}


impl Executor {
    /// Creates a new executor instance
    pub fn new(exe_dir: PathBuf, logger: Logger) -> Self {
        Self { exe_dir, logger }
    }


    /// Executes yt-dlp with the given arguments - simple and robust
    pub fn execute(&self, executable_path: &Path, args: &[String]) -> Result<()> {
        if args.is_empty() {
            self.logger.log_warning("No arguments provided for yt-dlp");
            return Ok(());
        }

        // Basic validation - just check if executable exists
        if !executable_path.exists() {
            return Err(AppError::FileNotFound(format!("Executable not found: {:?}", executable_path)));
        }

        self.logger.log_info(&format!("Executing {} with {} arguments", crate::constants::YT_DLP_EXECUTABLE, args.len()));
        self.logger.log_debug(&format!("Arguments: {:?}", args));

        // Set up custom temp directory
        let temp_dir = self.setup_temp_directory()?;

        // Create and configure command - keep it simple
        let mut cmd = Command::new(executable_path);
        cmd.args(args)
            .current_dir(&self.exe_dir)
            .env("TEMP", temp_dir.to_string_lossy().as_ref())
            .env("TMP", temp_dir.to_string_lossy().as_ref());

        // Windows process isolation (optional)
        #[cfg(windows)]
        {
            const CREATE_NEW_PROCESS_GROUP: u32 = 0x00000200;
            cmd.creation_flags(CREATE_NEW_PROCESS_GROUP);
        }

        // Execute and wait - simple approach
        self.logger.log_debug(&format!("Spawning process: {:?}", executable_path));
        
        let output = cmd.output().map_err(|e| {
            self.logger.log_error(&format!("Failed to execute {}: {}", crate::constants::YT_DLP_EXECUTABLE, e));
            AppError::Execution(format!("Failed to execute {}: {}", crate::constants::YT_DLP_EXECUTABLE, e))
        })?;

        // Forward output to console
        if !output.stdout.is_empty() {
            print!("{}", String::from_utf8_lossy(&output.stdout));
        }
        if !output.stderr.is_empty() {
            eprint!("{}", String::from_utf8_lossy(&output.stderr));
        }

        // Check exit status
        if !output.status.success() {
            let error_msg = format!("{} exited with status: {}", crate::constants::YT_DLP_EXECUTABLE, output.status);
            self.logger.log_error(&error_msg);
            
            // Log stderr if present
            if !output.stderr.is_empty() {
                self.logger.log_debug(&format!("Error output: {}", String::from_utf8_lossy(&output.stderr)));
            }
            
            return Err(AppError::Execution(error_msg));
        }

        self.logger.log_info("Process completed successfully");
        Ok(())
    }

    fn setup_temp_directory(&self) -> Result<PathBuf> {
        let temp_dir = self.exe_dir.join(crate::constants::TEMP_DIR_NAME);

        // Create directory if it doesn't exist
        if let Err(e) = create_dir_all(&temp_dir) {
            self.logger.log_warning(&format!("Could not create temp directory: {}", e));
            // Fall back to system temp if creation fails
            return Ok(std::env::temp_dir());
        }

        self.logger.log_debug(&format!("Using temp directory: {:?}", temp_dir));
        Ok(temp_dir)
    }


}
