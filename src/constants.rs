pub const CONFIG_FILE_NAME: &str = "config.json";
pub const VERSION_FILE_NAME: &str = "version.txt";
pub const GITHUB_API_URL: &str = "https://api.github.com/repos/yt-dlp/yt-dlp/releases/latest";
pub const YT_DLP_EXECUTABLE: &str = "yt-dlp.exe";

/// Default configuration values
pub mod defaults {
    pub const LOG_MAX_SIZE_MB: u32 = 10;
    pub const LOG_MAX_ARCHIVED: u32 = 5;
    pub const UPDATE_CHECK_DAYS: i64 = 1;
}