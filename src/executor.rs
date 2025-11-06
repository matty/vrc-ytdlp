use std::path::{Path, PathBuf};
use std::process as std_process;
use std::process::{Child, Command, Stdio};
use std::thread::sleep;
use std::time::{Duration, Instant};

use crate::constants::YT_DLP_EXECUTABLE;
use crate::error::Result;
use crate::logger::Logger;
use sysinfo::{ProcessesToUpdate, System};

pub struct Executor {
    exe_dir: PathBuf,
    pub logger: Logger,
}

impl Executor {
    pub fn new(exe_dir: PathBuf, logger: Logger) -> Self {
        Self { exe_dir, logger }
    }

    pub fn execute(&self, executable_path: &Path, args: &[String]) -> Result<()> {
        if Self::is_yt_dlp_running(executable_path) {
            self.logger.log_warning("Existing yt-dlp process detected; not starting a new one.");
            return Ok(());
        }

        if args.is_empty() {
            self.logger.log_warning("No arguments provided for yt-dlp");
            return Ok(());
        }

        if !executable_path.exists() {
            self.logger.log_error(&format!("Executable not found: {:?}", executable_path));
            return Ok(());
        }

        self.logger.log_info(&format!(
            "Executing {} with {} arguments",
            crate::constants::YT_DLP_EXECUTABLE,
            args.len()
        ));
        self.logger
            .log_debug(&format!("Arguments: {:?}", args));

        let temp_dir = self.exe_dir.clone();


        let mut cmd = Command::new(executable_path);
        cmd.args(args)
            .current_dir(&self.exe_dir)
            .env("TEMP", &temp_dir)
            .env("TMP", &temp_dir)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        self.logger
            .log_debug(&format!("Spawning process: {:?}", executable_path));

        let child = match cmd.spawn() {
            Ok(child) => child,
            Err(e) => {
                let msg = format!("Failed to spawn {}: {}", YT_DLP_EXECUTABLE, e);
                self.logger.log_error(&msg);
                return Ok(());
            }
        };

        self.logger
            .log_debug(&format!("Spawned with PID: {}", child.id()));

        let guard = ChildGuard::new(&self.logger, child);
        
        let result = guard.wait_with_timeout_and_capture(Duration::from_secs(30));

        match result {
            Ok((status, stdout, stderr)) => {
                // Raw process output has already been mirrored to the console by the
                // child guard. Here we log the captured contents to the log file.
                if !stdout.is_empty() {
                    self.logger
                        .log_info(&format!("Stdout: {}", stdout.trim()));
                }
                if !stderr.is_empty() {
                    if status.success() {
                        self.logger
                            .log_warning(&format!("Stderr: {}", stderr.trim()));
                    } else {
                        self.logger
                            .log_error(&format!("Stderr: {}", stderr.trim()));
                    }
                }

                if !status.success() {
                    let msg = if let Some(code) = status.code() {
                        format!("{} exited with status {}", YT_DLP_EXECUTABLE, code)
                    } else {
                        format!("{} terminated by signal/unknown status", YT_DLP_EXECUTABLE)
                    };
                    if stdout.is_empty() && stderr.is_empty() {
                        self.logger.log_warning("Process produced no output on stdout or stderr");
                    }
                    self.logger.log_error(&msg);
                    return Ok(());
                }

                self.logger.log_info("Process completed successfully");
                Ok(())
            }
            Err(e) => {
                let msg = format!("Error waiting for {}: {}", YT_DLP_EXECUTABLE, e);
                self.logger.log_error(&msg);
                Ok(())
            }
        }
    }

}

impl Executor {
    
    fn is_yt_dlp_running(target_executable_path: &Path) -> bool {
        let target_file_lc = target_executable_path
            .file_name()
            .map(|s| s.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_else(|| YT_DLP_EXECUTABLE.to_ascii_lowercase());

        let self_pid = std_process::id();
        
        let self_exe_name = std::env::current_exe()
            .ok()
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_ascii_lowercase()));

        let mut sys = System::new();
        // Refresh all processes; the second arg indicates whether to refresh user information
        sys.refresh_processes(ProcessesToUpdate::All, true);

        sys.processes().values().any(|proc| {
            if proc.pid().as_u32() == self_pid {
                return false;
            }
            
            if let Some(ref self_name) = self_exe_name {
                if let Some(exe_path) = proc.exe() {
                    if let Some(proc_file_name) = exe_path.file_name() {
                        let proc_file_lc = proc_file_name.to_string_lossy().to_ascii_lowercase();
                        if proc_file_lc == *self_name {
                            return false;
                        }
                    }
                }
                let proc_name_lc = proc.name().to_string_lossy().to_ascii_lowercase();
                if proc_name_lc == *self_name {
                    return false;
                }
            }

            if let Some(exe_path) = proc.exe() {
                if let Some(proc_file_name) = exe_path.file_name() {
                    let proc_file_lc = proc_file_name.to_string_lossy().to_ascii_lowercase();
                    if proc_file_lc == target_file_lc {
                        return true;
                    }
                }
            }

            let name_lc = proc.name().to_string_lossy().to_ascii_lowercase();
            name_lc == target_file_lc
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

    fn wait_with_timeout_and_capture(mut self, timeout: Duration) -> std::io::Result<(std::process::ExitStatus, String, String)> {
        use std::io::{Read, Write};
        use std::thread;

        if let Some(mut child) = self.child.take() {
            let stdout_handle = if let Some(mut stdout) = child.stdout.take() {
                Some(thread::spawn(move || {
                    let mut collected = Vec::new();
                    let mut buf = [0u8; 8192];
                    let mut console = std::io::stdout();

                    loop {
                        match stdout.read(&mut buf) {
                            Ok(0) => break,
                            Ok(n) => {
                                collected.extend_from_slice(&buf[..n]);
                                let _ = console.write_all(&buf[..n]);
                                let _ = console.flush();
                            }
                            Err(_) => break,
                        }
                    }

                    String::from_utf8_lossy(&collected).into_owned()
                }))
            } else { None };

            let stderr_handle = if let Some(mut stderr) = child.stderr.take() {
                Some(thread::spawn(move || {
                    let mut collected = Vec::new();
                    let mut buf = [0u8; 8192];
                    let mut console = std::io::stderr();

                    loop {
                        match stderr.read(&mut buf) {
                            Ok(0) => break,
                            Ok(n) => {
                                collected.extend_from_slice(&buf[..n]);
                                let _ = console.write_all(&buf[..n]);
                                let _ = console.flush();
                            }
                            Err(_) => break,
                        }
                    }

                    String::from_utf8_lossy(&collected).into_owned()
                }))
            } else { None };

            let start = Instant::now();
            loop {
                match child.try_wait()? {
                    Some(status) => {
                        // Child exited; collect outputs (blocking joins acceptable here).
                        let stdout = stdout_handle
                            .map(|h| h.join().unwrap_or_default())
                            .unwrap_or_default();
                        let stderr = stderr_handle
                            .map(|h| h.join().unwrap_or_default())
                            .unwrap_or_default();
                        return Ok((status, stdout, stderr));
                    }
                    None => {
                        if start.elapsed() >= timeout {
                            self.logger.log_warning("Timeout waiting for child; terminating process...");
                            let _ = child.kill();
                            let _ = child.wait()?;
                            let stdout = stdout_handle
                                .map(|h| h.join().unwrap_or_default())
                                .unwrap_or_default();
                            let stderr = stderr_handle
                                .map(|h| h.join().unwrap_or_default())
                                .unwrap_or_default();
                            return Err(std::io::Error::new(std::io::ErrorKind::TimedOut, format!("process timeout; partial stdout: {}; partial stderr: {}", stdout.len(), stderr.len())));
                        }
                        sleep(Duration::from_millis(200));
                    }
                }
            }
        } else {
            Err(std::io::Error::new(std::io::ErrorKind::Other, "child already taken"))
        }
    }
}

impl<'a> Drop for ChildGuard<'a> {
    fn drop(&mut self) {
        if let Some(child) = self.child.as_mut() {
            match child.try_wait() {
                Ok(Some(_status)) => {
                }
                Ok(None) => {
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
