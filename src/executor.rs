use std::path::Path;
use std::process::{Command, Stdio};
use std::thread::sleep;
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};

pub fn run_ytdlp(exe_path: &Path, args: &[String], timeout: Duration) -> Result<()> {
    let work_dir = exe_path.parent().unwrap_or(Path::new("."));

    let mut child = Command::new(exe_path)
        .args(args)
        .current_dir(work_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .context("spawning yt-dlp")?;

    let _job = JobObject::attach(&child).context("creating job object for yt-dlp")?;

    let start = Instant::now();
    loop {
        match child.try_wait()? {
            Some(status) => {
                if status.success() {
                    tracing::info!("yt-dlp completed successfully");
                    return Ok(());
                }
                bail!(
                    "yt-dlp exited with {}",
                    status
                        .code()
                        .map(|c| c.to_string())
                        .unwrap_or_else(|| "unknown status".into())
                );
            }
            None => {
                if start.elapsed() >= timeout {
                    tracing::warn!("yt-dlp timed out, terminating");
                    bail!("yt-dlp timed out after {}s", timeout.as_secs());
                }
                sleep(Duration::from_millis(200));
            }
        }
    }
}

// --- Windows Job Object RAII wrapper ---

#[cfg(windows)]
struct JobObject(windows_sys::Win32::Foundation::HANDLE);

#[cfg(windows)]
impl JobObject {
    fn attach(child: &std::process::Child) -> std::io::Result<Self> {
        use std::os::windows::io::AsRawHandle;
        use windows_sys::Win32::Foundation::CloseHandle;
        use windows_sys::Win32::System::JobObjects::{
            AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
            SetInformationJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
            JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
        };

        unsafe {
            let job = CreateJobObjectW(std::ptr::null(), std::ptr::null());
            if job.is_null() {
                return Err(std::io::Error::last_os_error());
            }

            let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
            info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;

            if SetInformationJobObject(
                job,
                JobObjectExtendedLimitInformation,
                &info as *const _ as *const _,
                std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            ) == 0
            {
                CloseHandle(job);
                return Err(std::io::Error::last_os_error());
            }

            if AssignProcessToJobObject(job, child.as_raw_handle() as _) == 0 {
                CloseHandle(job);
                return Err(std::io::Error::last_os_error());
            }

            Ok(Self(job))
        }
    }
}

#[cfg(windows)]
impl Drop for JobObject {
    fn drop(&mut self) {
        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(self.0);
        }
    }
}

#[cfg(not(windows))]
struct JobObject;

#[cfg(not(windows))]
impl JobObject {
    fn attach(_child: &std::process::Child) -> std::io::Result<Self> {
        Ok(Self)
    }
}
