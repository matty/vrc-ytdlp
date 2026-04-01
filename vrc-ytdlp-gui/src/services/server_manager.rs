use std::fs;
use std::process::Stdio;
use std::time::Duration;

use anyhow::{Context, Result};

use crate::paths;

pub async fn check_health(port: u16) -> bool {
    let url = format!("http://127.0.0.1:{port}/health");
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build();
    let client = match client {
        Ok(c) => c,
        Err(_) => return false,
    };
    client.get(&url).send().await.is_ok()
}

pub fn start_server(port: u16, idle_timeout: u64) -> Result<u32> {
    let app_dir = paths::exe_dir()?;
    let exe_name = if cfg!(windows) { "vrc-ytdlp.exe" } else { "vrc-ytdlp" };
    let exe_path = app_dir.join(exe_name);

    if !exe_path.exists() {
        anyhow::bail!("vrc-ytdlp binary not found at {}", exe_path.display());
    }

    let child = std::process::Command::new(&exe_path)
        .args(["--serve", "--port", &port.to_string(), "--idle-timeout", &idle_timeout.to_string()])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .context("spawning vrc-ytdlp server")?;

    let pid = child.id();
    let pid_path = paths::pid_file_path()?;
    let _ = fs::write(&pid_path, pid.to_string());

    Ok(pid)
}

pub fn stop_server() -> Result<()> {
    let pid_path = paths::pid_file_path()?;
    let pid_str = fs::read_to_string(&pid_path).context("reading server.pid")?;
    let pid: u32 = pid_str.trim().parse().context("parsing PID")?;
    kill_process(pid)?;
    let _ = fs::remove_file(&pid_path);
    Ok(())
}

pub fn read_pid() -> Option<u32> {
    let pid_path = paths::pid_file_path().ok()?;
    let pid_str = fs::read_to_string(&pid_path).ok()?;
    pid_str.trim().parse().ok()
}

#[cfg(windows)]
fn kill_process(pid: u32) -> Result<()> {
    use std::process::Command;
    Command::new("taskkill")
        .args(["/F", "/PID", &pid.to_string()])
        .output()
        .context("killing server process")?;
    Ok(())
}

#[cfg(not(windows))]
fn kill_process(pid: u32) -> Result<()> {
    unsafe { libc::kill(pid as i32, libc::SIGTERM); }
    Ok(())
}
