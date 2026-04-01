use std::fs;
use std::process::Stdio;
use std::time::Duration;

use crate::paths;

#[derive(serde::Serialize)]
pub struct ServerStatus {
    pub running: bool,
    pub pid: Option<u32>,
    pub port: u16,
}

#[tauri::command]
pub async fn check_server_health(port: u16) -> bool {
    let url = format!("http://127.0.0.1:{port}/health");
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
    {
        Ok(c) => c,
        Err(_) => return false,
    };
    client.get(&url).send().await.is_ok()
}

#[tauri::command]
pub fn start_server(port: u16, idle_timeout: u64) -> Result<u32, String> {
    let app_dir = paths::app_dir().map_err(|e| e.to_string())?;
    let exe_name = if cfg!(windows) { "vrc-ytdlp.exe" } else { "vrc-ytdlp" };
    let exe_path = app_dir.join(exe_name);

    if !exe_path.exists() {
        return Err(format!("vrc-ytdlp not found at {}", exe_path.display()));
    }

    let child = std::process::Command::new(&exe_path)
        .args([
            "--serve",
            "--port", &port.to_string(),
            "--idle-timeout", &idle_timeout.to_string(),
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("Failed to start server: {e}"))?;

    let pid = child.id();
    if let Ok(pid_path) = paths::pid_file_path() {
        let _ = fs::write(&pid_path, pid.to_string());
    }

    Ok(pid)
}

#[tauri::command]
pub fn stop_server() -> Result<(), String> {
    let pid_path = paths::pid_file_path().map_err(|e| e.to_string())?;
    let pid_str = fs::read_to_string(&pid_path).map_err(|e| format!("No PID file: {e}"))?;
    let pid: u32 = pid_str.trim().parse().map_err(|e| format!("Invalid PID: {e}"))?;

    kill_process(pid).map_err(|e| e.to_string())?;
    let _ = fs::remove_file(&pid_path);
    Ok(())
}

#[tauri::command]
pub fn get_server_pid() -> Option<u32> {
    let pid_path = paths::pid_file_path().ok()?;
    let pid_str = fs::read_to_string(&pid_path).ok()?;
    pid_str.trim().parse().ok()
}

#[cfg(windows)]
fn kill_process(pid: u32) -> anyhow::Result<()> {
    std::process::Command::new("taskkill")
        .args(["/F", "/PID", &pid.to_string()])
        .output()?;
    Ok(())
}

#[cfg(not(windows))]
fn kill_process(pid: u32) -> anyhow::Result<()> {
    unsafe { libc::kill(pid as i32, libc::SIGTERM); }
    Ok(())
}
