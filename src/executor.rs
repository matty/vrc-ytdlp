use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};
use std::thread::sleep;

use crate::error::{AppError, Result};
use crate::logger::Logger;
use crate::constants::YT_DLP_EXECUTABLE;
use sysinfo::System;

pub struct Executor {
    exe_dir: PathBuf,
    pub logger: Logger,
}

impl Executor {
    pub fn new(exe_dir: PathBuf, logger: Logger) -> Self {
        Self { exe_dir, logger }
    }

    pub fn execute(&self, executable_path: &Path, args: &[String]) -> Result<()> {
        if Self::is_yt_dlp_running() {
            self.logger
                .log_warning("Detected an existing yt-dlp process. Skipping new invocation.");
            return Ok(());
        }

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
                YT_DLP_EXECUTABLE, e
            );
            self.logger.log_error(&msg);
            AppError::Execution(msg)
        })?;

        self.logger
            .log_debug(&format!("Spawned with PID: {}", child.id()));

        let guard = ChildGuard::new(&self.logger, child);

        // Wait for completion with timeout
        let status = guard
            .wait_with_timeout(Duration::from_secs(30))
            .map_err(|e| {
                let msg = if e.kind() == std::io::ErrorKind::TimedOut {
                    format!(
                        "{} did not respond within 30 seconds and was terminated",
                        YT_DLP_EXECUTABLE
                    )
                } else {
                    format!(
                        "Failed while waiting for {}: {}",
                        YT_DLP_EXECUTABLE, e
                    )
                };
                self.logger.log_error(&msg);
                AppError::Execution(msg)
            })?;

        if !status.success() {
            let error_msg = if let Some(code) = status.code() {
                format!(
                    "{} exited with non-zero status code: {}",
                    YT_DLP_EXECUTABLE, code
                )
            } else {
                format!(
                    "{} terminated by signal/unknown status",
                    YT_DLP_EXECUTABLE
                )
            };
            self.logger.log_error(&error_msg);
            return Err(AppError::Execution(error_msg));
        }

        self.logger.log_info("Process completed successfully");
        Ok(())
    }

}

impl Executor {
    fn is_yt_dlp_running() -> bool {
        // Build a lowercase set of names to match against
        let target_full = YT_DLP_EXECUTABLE.to_ascii_lowercase(); // e.g., "yt-dlp.exe"
        let target_base = target_full
            .trim_end_matches(".exe")
            .trim_end_matches(".EXE")
            .to_string(); // e.g., "yt-dlp"

        let mut sys = System::new();
        // Refresh only processes (fast enough for our use-case)
        sys.refresh_processes();

        sys.processes().values().any(|proc| {
            let name = proc.name().to_ascii_lowercase();
            // Match exact executable name or base name to be safe across platforms
            name == target_full || name == target_base || name.contains("yt-dlp")
        })
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

    /// Waits for the child to exit up to a timeout; kills it on timeout and returns TimedOut.
    fn wait_with_timeout(mut self, timeout: Duration) -> std::io::Result<std::process::ExitStatus> {
        if let Some(mut child) = self.child.take() {
            let start = Instant::now();
            loop {
                match child.try_wait()? {
                    Some(status) => return Ok(status),
                    None => {
                        if start.elapsed() >= timeout {
                            // Timeout reached: try to kill and wait, then return TimedOut error
                            self.logger.log_warning("Timeout waiting for child; terminating process...");
                            let _ = child.kill();
                            let _ = child.wait();
                            return Err(std::io::Error::new(std::io::ErrorKind::TimedOut, "process timeout"));
                        }
                        sleep(Duration::from_millis(200));
                    }
                }
            }
        } else {
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
