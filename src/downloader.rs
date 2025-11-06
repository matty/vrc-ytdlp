use std::fs;
use std::path::{Path, PathBuf};

use chrono::{Duration, Utc};

use crate::constants::{GITHUB_API_URL, VERSION_FILE_NAME, YT_DLP_EXECUTABLE};
use crate::error::{AppError, Result};
use crate::logger::Logger;
use crate::models::{GitHubRelease, VersionInfo};

/// Handles downloading and updating yt-dlp
pub struct Downloader {
    exe_path: PathBuf,
    exe_dir: PathBuf,
    logger: Logger,
}

impl Downloader {
    /// Creates a new downloader instance
    pub fn new(exe_path: PathBuf, logger: Logger) -> Self {
        let exe_dir = exe_path.parent().unwrap_or_else(|| Path::new(".")).to_path_buf();

        // Ensure the directory exists
        if let Err(e) = fs::create_dir_all(&exe_dir) {
            logger.log_error(&format!("Failed to create yt-dlp directory: {}", e));
        }

        Self { exe_path, exe_dir, logger }
    }

    /// Gets the path to the yt-dlp executable
    pub fn get_executable_path(&self) -> PathBuf {
        self.exe_path.clone()
    }

    /// Checks if yt-dlp executable exists
    pub fn executable_exists(&self) -> bool {
        self.exe_path.exists()
    }

    /// Downloads the latest version of yt-dlp
    pub async fn download_latest(&self) -> Result<()> {
        self.logger.log_info("Starting yt-dlp download...");

        let release = self.get_latest_release().await?;
        let asset = self.find_windows_executable(&release)?;

        self.logger.log_info(&format!("Downloading from: {}", asset.browser_download_url));

        let bytes = self.download_file(&asset.browser_download_url).await?;
        self.save_executable(&bytes)?;
        self.save_version_info(&release.tag_name)?;

        self.logger.log_info(&format!("Successfully downloaded yt-dlp version: {}", release.tag_name));

        Ok(())
    }

    /// Checks for updates and downloads if necessary
    pub async fn check_and_update(&self) -> Result<()> {
        let version_path = self.exe_dir.join(VERSION_FILE_NAME);

        let mut version_info = self.load_version_info(&version_path)?;

        if !self.should_check_for_updates(&version_info) {
            return Ok(());
        }

        self.logger.log_info("Checking for yt-dlp updates...");

        let latest_version = self.get_latest_version().await?;
        version_info.last_check = Some(Utc::now());

        if version_info.version != latest_version {
            self.logger.log_info(&format!(
                "New version available: {} (current: {})",
                latest_version,
                version_info.version
            ));

            self.download_latest().await?;
        } else {
            self.logger.log_info(&format!("yt-dlp is up to date: {}", version_info.version));
            self.save_version_info_to_file(&version_info, &version_path)?;
        }

        Ok(())
    }

    /// Gets the latest version tag from GitHub
    async fn get_latest_version(&self) -> Result<String> {
        let release = self.get_latest_release().await?;
        Ok(release.tag_name)
    }

    /// Fetches the latest release information from GitHub API
    async fn get_latest_release(&self) -> Result<GitHubRelease> {
        let client = reqwest::Client::new();
        let response = client
            .get(GITHUB_API_URL)
            .header("User-Agent", "yt-dlp-proxy")
            .send()
            .await?;

        let release: GitHubRelease = response.json().await?;
        Ok(release)
    }

    /// Finds the Windows executable in the release assets
    fn find_windows_executable<'a>(&self, release: &'a GitHubRelease) -> Result<&'a crate::models::GitHubAsset> {
        release.assets.iter()
            .find(|asset| asset.name == YT_DLP_EXECUTABLE)
            .ok_or_else(|| AppError::Download(format!("Could not find {} in release assets", YT_DLP_EXECUTABLE)))
    }

    /// Downloads a file from the given URL
    async fn download_file(&self, url: &str) -> Result<bytes::Bytes> {
        let client = reqwest::Client::new();
        let response = client.get(url).send().await?;
        let bytes = response.bytes().await?;
        Ok(bytes)
    }

    /// Saves the executable to disk
    fn save_executable(&self, bytes: &[u8]) -> Result<()> {
        fs::write(&self.exe_path, bytes)?;
        Ok(())
    }

    /// Saves version information to disk
    fn save_version_info(&self, version: &str) -> Result<()> {
        let version_info = VersionInfo {
            version: version.to_string(),
            last_check: Some(Utc::now()),
        };

        let version_path = self.exe_dir.join(VERSION_FILE_NAME);
        self.save_version_info_to_file(&version_info, &version_path)
    }

    /// Saves version info to a specific file
    fn save_version_info_to_file(&self, version_info: &VersionInfo, path: &Path) -> Result<()> {
        let version_json = serde_json::to_string(version_info)?;
        fs::write(path, version_json)?;
        Ok(())
    }

    /// Loads version information from disk
    fn load_version_info(&self, version_path: &Path) -> Result<VersionInfo> {
        if version_path.exists() {
            let content = fs::read_to_string(version_path)?;
            let version_info = serde_json::from_str::<VersionInfo>(&content)
                .unwrap_or_default();
            Ok(version_info)
        } else {
            Ok(VersionInfo::default())
        }
    }

    /// Determines if we should check for updates (once per day)
    fn should_check_for_updates(&self, version_info: &VersionInfo) -> bool {
        if let Some(last_check) = version_info.last_check {
            Utc::now() - last_check > Duration::days(crate::constants::defaults::UPDATE_CHECK_DAYS)
        } else {
            true
        }
    }
}
