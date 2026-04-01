use std::fs;
use std::path::Path;
use std::time::SystemTime;

use anyhow::{Context, Result};
use crate::paths;

#[derive(Debug, Clone)]
pub struct CookieStatus {
    pub exists: bool,
    pub age_description: Option<String>,
}

pub fn check_cookies(app_dir: &Path) -> CookieStatus {
    let cookie_path = app_dir.join("cookies.txt");
    if !cookie_path.exists() {
        return CookieStatus { exists: false, age_description: None };
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
    CookieStatus { exists: true, age_description: age }
}

pub async fn extract_cookies(ytdlp_path: &Path, browser: &str) -> Result<String> {
    let app_dir = paths::exe_dir()?;
    let cookie_path = app_dir.join("cookies.txt");
    let output = tokio::process::Command::new(ytdlp_path)
        .args(["--cookies-from-browser", browser, "--cookies", &cookie_path.to_string_lossy(), "--version"])
        .output().await
        .context("running yt-dlp for cookie extraction")?;
    if output.status.success() {
        Ok("Cookies extracted successfully".to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Cookie extraction failed: {stderr}")
    }
}

pub const BROWSERS: &[&str] = &["firefox", "chrome", "chromium", "edge", "brave", "opera", "vivaldi"];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_missing_cookies() {
        let dir = std::env::temp_dir().join("vrc-gui-test-no-cookies");
        let _ = fs::create_dir_all(&dir);
        let _ = fs::remove_file(dir.join("cookies.txt"));
        let status = check_cookies(&dir);
        assert!(!status.exists);
        assert!(status.age_description.is_none());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn check_existing_cookies() {
        let dir = std::env::temp_dir().join("vrc-gui-test-has-cookies");
        let _ = fs::create_dir_all(&dir);
        fs::write(dir.join("cookies.txt"), "cookie data").unwrap();
        let status = check_cookies(&dir);
        assert!(status.exists);
        assert!(status.age_description.is_some());
        let _ = fs::remove_dir_all(&dir);
    }
}
