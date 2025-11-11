use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use crate::error::{AppError, Result};
use crate::logger::Logger;

pub struct Executor {
    exe_dir: PathBuf,
    pub logger: Logger,
}

impl Executor {
    pub fn new(exe_dir: PathBuf, logger: Logger) -> Self {
        Self { exe_dir, logger }
    }

    pub fn execute(&self, executable_path: &Path, args: &[String]) -> Result<()> {
        if args.is_empty() {
            self.logger.log_warning("No arguments provided for yt-dlp");
            return Ok(());
        }

        if !executable_path.exists() {
            return Err(AppError::FileNotFound(format!(
                "Executable not found: {:?}",
                executable_path
            )));
        }

        self.logger.log_info(&format!(
            "Executing {} with {} arguments",
            crate::constants::YT_DLP_EXECUTABLE,
            args.len()
        ));
        self.logger
            .log_debug(&format!("Arguments: {:?}", args));

        // Use the executable directory as temp directory
        let temp_dir = self.exe_dir.clone();

        // Create and configure command - set temp envs to exe dir
        let mut cmd = Command::new(executable_path);
        cmd.args(args)
            .current_dir(&self.exe_dir)
            .env("TEMP", &temp_dir)
            .env("TMP", &temp_dir)
            .stdin(Stdio::null())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());

        // Execute with streaming IO
        self.logger
            .log_debug(&format!("Spawning process: {:?}", executable_path));

        let child = cmd.spawn().map_err(|e| {
            let msg = format!(
                "Failed to spawn {}: {}",
                crate::constants::YT_DLP_EXECUTABLE, e
            );
            self.logger.log_error(&msg);
            AppError::Execution(msg)
        })?;

        self.logger
            .log_debug(&format!("Spawned with PID: {}", child.id()));

        let guard = ChildGuard::new(&self.logger, child);

        // Wait for completion
        let status = guard.wait().map_err(|e| {
            let msg = format!(
                "Failed while waiting for {}: {}",
                crate::constants::YT_DLP_EXECUTABLE, e
            );
            self.logger.log_error(&msg);
            AppError::Execution(msg)
        })?;

        if !status.success() {
            let error_msg = if let Some(code) = status.code() {
                format!(
                    "{} exited with non-zero status code: {}",
                    crate::constants::YT_DLP_EXECUTABLE, code
                )
            } else {
                format!(
                    "{} terminated by signal/unknown status",
                    crate::constants::YT_DLP_EXECUTABLE
                )
            };
            self.logger.log_error(&error_msg);
            return Err(AppError::Execution(error_msg));
        }

        self.logger.log_info("Process completed successfully");
        Ok(())
    }

}

struct ChildGuard<'a> {
    child: Option<Child>,
    logger: &'a Logger,
}

impl<'a> ChildGuard<'a> {
    fn new(logger: &'a Logger, child: Child) -> Self {
        Self {
            child: Some(child),
            logger,
        }
    }

    /// Waits for the child to exit and consumes the guard.
    fn wait(mut self) -> std::io::Result<std::process::ExitStatus> {
        if let Some(mut child) = self.child.take() {
            child.wait()
        } else {
            // Should never happen
            Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "child already taken",
            ))
        }
    }
}

impl<'a> Drop for ChildGuard<'a> {
    fn drop(&mut self) {
        if let Some(child) = self.child.as_mut() {
            match child.try_wait() {
                Ok(Some(_status)) => {
                    // already exited
                }
                Ok(None) => {
                    // Still running -> try to terminate and wait
                    self.logger
                        .log_warning("Child process still running, attempting to terminate...");
                    if let Err(e) = child.kill() {
                        self.logger
                            .log_warning(&format!("Failed to terminate child: {}", e));
                    }
                    let _ = child.wait();
                }
                Err(e) => {
                    self.logger
                        .log_warning(&format!("Failed to query child status: {}", e));
                }
            }
        }
    }
}
