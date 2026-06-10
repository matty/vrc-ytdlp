use std::fs;
use std::path::Path;
use std::time::Duration;

use anyhow::{Context, Result};
use serde::Deserialize;

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

#[derive(Debug, Clone)]
pub struct VersionInfo {
    pub current: Option<String>,
    pub latest: Option<String>,
    pub update_available: bool,
}

pub fn current_version(ytdlp_path: &Path) -> Option<String> {
    let version_path = paths::version_file_path(ytdlp_path);
    fs::read_to_string(version_path).ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

pub async fn check_for_update(ytdlp_path: &Path) -> Result<VersionInfo> {
    let current = current_version(ytdlp_path);
    let client = build_client()?;
    let release = fetch_release(&client).await?;
    let update_available = match &current {
        Some(v) => v != &release.tag_name,
        None => true,
    };
    Ok(VersionInfo { current, latest: Some(release.tag_name), update_available })
}

pub async fn download_latest(ytdlp_path: &Path) -> Result<String> {
    let client = build_client()?;
    let release = fetch_release(&client).await?;
    let asset = release.assets.iter()
        .find(|a| a.name == ASSET_NAME)
        .context(format!("{ASSET_NAME} not found in release assets"))?;

    let bytes = client.get(&asset.browser_download_url).send().await?
        .error_for_status().context("downloading yt-dlp")?
        .bytes().await?;

    let dir = ytdlp_path.parent().unwrap_or(Path::new("."));
    fs::create_dir_all(dir).context("creating yt-dlp directory")?;

    let tmp_path = ytdlp_path.with_file_name(".yt-dlp-download.tmp");
    fs::write(&tmp_path, &bytes).context("writing temp file")?;
    if ytdlp_path.exists() { let _ = fs::remove_file(ytdlp_path); }
    fs::rename(&tmp_path, ytdlp_path).context("renaming to target")?;

    let version_path = paths::version_file_path(ytdlp_path);
    fs::write(&version_path, &release.tag_name).context("writing version file")?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(ytdlp_path, fs::Permissions::from_mode(0o755));
    }

    Ok(release.tag_name)
}

pub fn binary_exists(path: &Path) -> bool {
    path.exists() && path.is_file()
}

fn build_client() -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .user_agent("VRC-YtDlp-GUI")
        .timeout(Duration::from_secs(30))
        .build()
        .context("building HTTP client")
}

async fn fetch_release(client: &reqwest::Client) -> Result<GitHubRelease> {
    client.get(GITHUB_API_URL).send().await?
        .error_for_status().context("fetching latest release")?
        .json().await.context("parsing release JSON")
}
