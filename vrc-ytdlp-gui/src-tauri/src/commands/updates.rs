use std::fs;
use std::path::Path;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::paths;

const GITHUB_API_URL: &str = "https://api.github.com/repos/yt-dlp/yt-dlp/releases/latest";
const ASSET_NAME: &str = if cfg!(windows) { "yt-dlp_x86.exe" } else { "yt-dlp_linux" };

#[derive(Deserialize)]
struct GitHubRelease {
    tag_name: String,
    assets: Vec<GitHubAsset>,
}

#[derive(Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct VersionInfo {
    pub current: Option<String>,
    pub latest: Option<String>,
    pub update_available: bool,
    pub ytdlp_exists: bool,
    pub ffmpeg_exists: bool,
}

#[tauri::command]
pub fn get_version_info(ytdlp_location: String, ffmpeg_location: String) -> Result<VersionInfo, String> {
    let ytdlp_path = paths::resolve_path(&ytdlp_location).map_err(|e| e.to_string())?;
    let ffmpeg_path = paths::resolve_path(&ffmpeg_location).map_err(|e| e.to_string())?;
    let version_path = paths::version_file_path(&ytdlp_path);

    let current = fs::read_to_string(&version_path)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    Ok(VersionInfo {
        current,
        latest: None,
        update_available: false,
        ytdlp_exists: ytdlp_path.exists(),
        ffmpeg_exists: ffmpeg_path.exists(),
    })
}

#[tauri::command]
pub async fn check_for_update(ytdlp_location: String) -> Result<VersionInfo, String> {
    let ytdlp_path = paths::resolve_path(&ytdlp_location).map_err(|e| e.to_string())?;
    let version_path = paths::version_file_path(&ytdlp_path);

    let current = fs::read_to_string(&version_path)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    let client = build_client().map_err(|e| e.to_string())?;
    let release = fetch_release(&client).await.map_err(|e| e.to_string())?;

    let update_available = match &current {
        Some(v) => v != &release.tag_name,
        None => true,
    };

    Ok(VersionInfo {
        current,
        latest: Some(release.tag_name),
        update_available,
        ytdlp_exists: ytdlp_path.exists(),
        ffmpeg_exists: true,
    })
}

#[tauri::command]
pub async fn download_ytdlp(ytdlp_location: String) -> Result<String, String> {
    let ytdlp_path = paths::resolve_path(&ytdlp_location).map_err(|e| e.to_string())?;
    let client = build_client().map_err(|e| e.to_string())?;
    let release = fetch_release(&client).await.map_err(|e| e.to_string())?;

    let asset = release.assets.iter()
        .find(|a| a.name == ASSET_NAME)
        .ok_or_else(|| format!("{ASSET_NAME} not found in release"))?;

    let bytes = client.get(&asset.browser_download_url).send().await
        .map_err(|e| e.to_string())?
        .error_for_status().map_err(|e| e.to_string())?
        .bytes().await.map_err(|e| e.to_string())?;

    let dir = ytdlp_path.parent().unwrap_or(Path::new("."));
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;

    let tmp = ytdlp_path.with_file_name(".yt-dlp-download.tmp");
    fs::write(&tmp, &bytes).map_err(|e| e.to_string())?;
    if ytdlp_path.exists() { let _ = fs::remove_file(&ytdlp_path); }
    fs::rename(&tmp, &ytdlp_path).map_err(|e| e.to_string())?;

    let version_path = paths::version_file_path(&ytdlp_path);
    fs::write(&version_path, &release.tag_name).map_err(|e| e.to_string())?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&ytdlp_path, fs::Permissions::from_mode(0o755));
    }

    Ok(release.tag_name)
}

fn build_client() -> anyhow::Result<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .user_agent("VRC-YtDlp-GUI")
        .timeout(Duration::from_secs(30))
        .build()?)
}

async fn fetch_release(client: &reqwest::Client) -> anyhow::Result<GitHubRelease> {
    Ok(client.get(GITHUB_API_URL).send().await?.error_for_status()?.json().await?)
}
