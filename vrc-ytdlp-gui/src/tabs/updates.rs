use std::path::PathBuf;

use iced::widget::{button, column, row, text};
use iced::{Element, Task};

use crate::services::downloader;
use crate::theme;
use crate::widget;

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct UpdatesTabState {
    pub ytdlp_path: PathBuf,
    pub ffmpeg_path: PathBuf,
    pub current_version: Option<String>,
    pub latest_version: Option<String>,
    pub update_available: bool,
    pub checking: bool,
    pub downloading: bool,
    pub ffmpeg_exists: bool,
    pub error: Option<String>,
    pub success: Option<String>,
}

impl UpdatesTabState {
    pub fn new(ytdlp_path: PathBuf, ffmpeg_path: PathBuf) -> Self {
        let current_version = downloader::current_version(&ytdlp_path);
        let ffmpeg_exists = downloader::binary_exists(&ffmpeg_path);
        Self {
            ytdlp_path,
            ffmpeg_path,
            current_version,
            latest_version: None,
            update_available: false,
            checking: false,
            downloading: false,
            ffmpeg_exists,
            error: None,
            success: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Message
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum UpdatesMessage {
    CheckForUpdate,
    CheckResult(Result<(Option<String>, Option<String>, bool), String>),
    DownloadUpdate,
    DownloadResult(Result<String, String>),
}

// ---------------------------------------------------------------------------
// Update
// ---------------------------------------------------------------------------

pub fn update(state: &mut UpdatesTabState, msg: UpdatesMessage) -> Task<UpdatesMessage> {
    match msg {
        UpdatesMessage::CheckForUpdate => {
            state.checking = true;
            state.error = None;
            state.success = None;
            let path = state.ytdlp_path.clone();
            Task::perform(
                async move {
                    downloader::check_for_update(&path)
                        .await
                        .map(|v| (v.current, v.latest, v.update_available))
                        .map_err(|e| e.to_string())
                },
                UpdatesMessage::CheckResult,
            )
        }
        UpdatesMessage::CheckResult(result) => {
            state.checking = false;
            match result {
                Ok((current, latest, available)) => {
                    state.current_version = current;
                    state.latest_version = latest;
                    state.update_available = available;
                    if !available {
                        state.success = Some("Already up to date.".to_string());
                    }
                }
                Err(e) => state.error = Some(e),
            }
            Task::none()
        }
        UpdatesMessage::DownloadUpdate => {
            state.downloading = true;
            state.error = None;
            state.success = None;
            let path = state.ytdlp_path.clone();
            Task::perform(
                async move {
                    downloader::download_latest(&path)
                        .await
                        .map_err(|e| e.to_string())
                },
                UpdatesMessage::DownloadResult,
            )
        }
        UpdatesMessage::DownloadResult(result) => {
            state.downloading = false;
            match result {
                Ok(version) => {
                    state.current_version = Some(version.clone());
                    state.latest_version = Some(version);
                    state.update_available = false;
                    state.success = Some("Updated successfully.".to_string());
                }
                Err(e) => state.error = Some(e),
            }
            Task::none()
        }
    }
}

// ---------------------------------------------------------------------------
// View
// ---------------------------------------------------------------------------

pub fn view(state: &UpdatesTabState) -> Element<UpdatesMessage> {
    let current = state
        .current_version
        .as_deref()
        .unwrap_or("Not installed");
    let latest = state.latest_version.as_deref().unwrap_or("Unknown");

    let ytdlp_dot = if state.current_version.is_some() {
        widget::status_dot(theme::GREEN)
    } else {
        widget::status_dot(theme::RED)
    };

    let check_btn = if state.checking {
        button("Checking...").style(button::secondary)
    } else {
        button("Check for Updates")
            .on_press(UpdatesMessage::CheckForUpdate)
            .style(button::primary)
    };

    let download_btn = if state.downloading {
        button("Downloading...").style(button::secondary)
    } else if state.update_available {
        button("Download Update")
            .on_press(UpdatesMessage::DownloadUpdate)
            .style(button::primary)
    } else {
        button("Download Update").style(button::secondary)
    };

    let ytdlp_section = widget::card(
        column![
            row![ytdlp_dot, text("yt-dlp").size(16),]
                .spacing(8)
                .align_y(iced::Alignment::Center),
            text(format!("Current: {current}"))
                .size(14)
                .color(theme::GREY),
            text(format!("Latest:  {latest}"))
                .size(14)
                .color(theme::GREY),
            row![check_btn, download_btn].spacing(theme::SPACING),
        ]
        .spacing(8),
    );

    // ffmpeg section
    let ffmpeg_dot = if state.ffmpeg_exists {
        widget::status_dot(theme::GREEN)
    } else {
        widget::status_dot(theme::RED)
    };
    let ffmpeg_status = if state.ffmpeg_exists {
        "Installed"
    } else {
        "Not found"
    };
    let ffmpeg_section = widget::card(
        column![
            row![ffmpeg_dot, text("ffmpeg").size(16),]
                .spacing(8)
                .align_y(iced::Alignment::Center),
            text(ffmpeg_status).size(14).color(theme::GREY),
        ]
        .spacing(8),
    );

    let mut content = column![
        widget::section_header("Updates"),
        ytdlp_section,
        ffmpeg_section,
    ]
    .spacing(theme::SPACING);

    if let Some(err) = &state.error {
        content = content.push(text(format!("Error: {err}")).size(13).color(theme::RED));
    }
    if let Some(msg) = &state.success {
        content = content.push(text(msg.clone()).size(13).color(theme::GREEN));
    }

    content.into()
}
