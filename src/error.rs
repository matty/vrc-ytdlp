use std::fmt;

#[derive(Debug)]
pub enum AppError {
    Io(std::io::Error),
    Reqwest(reqwest::Error),
    Serde(serde_json::Error),
    Download(String),
    Execution(String),
    Config(String),
    FileNotFound(String),
    PermissionDenied(String),

    NetworkError(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Io(err) => write!(f, "IO error: {}", err),
            AppError::Reqwest(err) => write!(f, "HTTP request error: {}", err),
            AppError::Serde(err) => write!(f, "JSON serialization error: {}", err),
            AppError::Download(msg) => write!(f, "Download error: {}", msg),
            AppError::Execution(msg) => write!(f, "Execution error: {}", msg),
            AppError::Config(msg) => write!(f, "Configuration error: {}", msg),
            AppError::FileNotFound(msg) => write!(f, "File not found: {}", msg),
            AppError::PermissionDenied(msg) => write!(f, "Permission denied: {}", msg),

            AppError::NetworkError(msg) => write!(f, "Network error: {}", msg),
        }
    }
}

impl std::error::Error for AppError {}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        match err.kind() {
            std::io::ErrorKind::NotFound => AppError::FileNotFound(err.to_string()),
            std::io::ErrorKind::PermissionDenied => AppError::PermissionDenied(err.to_string()),
            _ => AppError::Io(err),
        }
    }
}

impl From<reqwest::Error> for AppError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_connect() || err.is_timeout() {
            AppError::NetworkError(err.to_string())
        } else {
            AppError::Reqwest(err)
        }
    }
}

impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        AppError::Serde(err)
    }
}

pub type Result<T> = std::result::Result<T, AppError>;
