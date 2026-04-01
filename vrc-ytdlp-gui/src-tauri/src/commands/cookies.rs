use std::fs;
use std::time::SystemTime;

use serde::Serialize;

use crate::paths;

pub const BROWSERS: &[&str] = &["firefox", "chrome", "chromium", "edge", "brave", "opera", "vivaldi"];

#[derive(Debug, Clone, Serialize)]
pub struct CookieStatus {
    pub exists: bool,
    pub age_description: Option<String>,
}

#[tauri::command]
pub fn check_cookies() -> Result<CookieStatus, String> {
    let app_dir = paths::app_dir().map_err(|e| e.to_string())?;
    let cookie_path = app_dir.join("cookies.txt");

    if !cookie_path.exists() {
        return Ok(CookieStatus { exists: false, age_description: None });
    }

    let age = fs::metadata(&cookie_path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|mtime| SystemTime::now().duration_since(mtime).ok())
        .map(|dur| {
            let secs = dur.as_secs();
            if secs < 60 { "just now".to_string() }
            else if secs < 3600 { format!("{} minutes ago", secs / 60) }
            else if secs < 86400 { format!("{} hours ago", secs / 3600) }
            else { format!("{} days ago", secs / 86400) }
        });

    Ok(CookieStatus { exists: true, age_description: age })
}

#[tauri::command]
pub async fn extract_cookies(ytdlp_location: String, browser: String) -> Result<String, String> {
    let ytdlp_path = paths::resolve_path(&ytdlp_location).map_err(|e| e.to_string())?;
    let app_dir = paths::app_dir().map_err(|e| e.to_string())?;
    let cookie_path = app_dir.join("cookies.txt");

    let output = tokio::process::Command::new(&ytdlp_path)
        .args([
            "--cookies-from-browser", &browser,
            "--cookies", &cookie_path.to_string_lossy(),
            "--version",
        ])
        .output()
        .await
        .map_err(|e| format!("Failed to run yt-dlp: {e}"))?;

    if output.status.success() {
        Ok("Cookies extracted successfully".to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("Extraction failed: {stderr}"))
    }
}

#[tauri::command]
pub fn get_browsers() -> Vec<String> {
    BROWSERS.iter().map(|s| s.to_string()).collect()
}
