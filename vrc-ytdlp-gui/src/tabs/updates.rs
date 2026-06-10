use std::path::PathBuf;

use iced::widget::{column, container, row, text};
use iced::{Element, Length, Task};

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

pub fn view(state: &UpdatesTabState) -> Element<'_, UpdatesMessage> {
    let current = state
        .current_version
        .as_deref()
        .unwrap_or("Not installed");
    let latest = state.latest_version.as_deref().unwrap_or("Unknown");

    // --- yt-dlp card ---
    let ytdlp_dot = if state.current_version.is_some() {
        widget::status_dot(theme::STATUS_GREEN)
    } else {
        widget::status_dot(theme::STATUS_RED)
    };

    let version_badge: Element<'_, UpdatesMessage> = if state.update_available {
        widget::pill_badge("Update available", theme::STATUS_YELLOW)
    } else if state.current_version.is_some() {
        widget::pill_badge("Current", theme::ACCENT)
    } else {
        widget::pill_badge("Not installed", theme::STATUS_RED)
    };

    let check_btn = if state.checking {
        widget::secondary_button("Checking...", None)
    } else {
        widget::primary_button("Check for Updates", Some(UpdatesMessage::CheckForUpdate))
    };

    let download_btn = if state.downloading {
        widget::secondary_button("Downloading...", None)
    } else if state.update_available {
        widget::primary_button("Download Update", Some(UpdatesMessage::DownloadUpdate))
    } else {
        widget::secondary_button("Download Update", None)
    };

    let ytdlp_card = widget::card(
        column![
            row![
                ytdlp_dot,
                text("yt-dlp").size(14).color(theme::TEXT_PRIMARY),
                iced::widget::Space::new(Length::Fill, 0),
                version_badge,
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center),
            row![
                column![
                    text("CURRENT VERSION")
                        .size(9)
                        .color(theme::TEXT_LABEL),
                    text(current)
                        .size(12)
                        .color(theme::TEXT_PRIMARY),
                ]
                .spacing(2),
                column![
                    text("LATEST VERSION")
                        .size(9)
                        .color(theme::TEXT_LABEL),
                    text(latest)
                        .size(12)
                        .color(theme::TEXT_PRIMARY),
                ]
                .spacing(2),
            ]
            .spacing(theme::SPACING_LG),
            row![check_btn, download_btn]
                .spacing(theme::SPACING_SM),
        ]
        .spacing(theme::SPACING),
    );

    // --- ffmpeg card ---
    let ffmpeg_dot = if state.ffmpeg_exists {
        widget::status_dot(theme::STATUS_GREEN)
    } else {
        widget::status_dot(theme::STATUS_RED)
    };
    let ffmpeg_badge: Element<'_, UpdatesMessage> = if state.ffmpeg_exists {
        widget::pill_badge("Found", theme::STATUS_GREEN)
    } else {
        widget::pill_badge("Not found", theme::STATUS_RED)
    };
    let ffmpeg_path_str = state.ffmpeg_path.to_string_lossy().to_string();

    let ffmpeg_card = widget::card(
        column![
            row![
                ffmpeg_dot,
                text("ffmpeg").size(14).color(theme::TEXT_PRIMARY),
                iced::widget::Space::new(Length::Fill, 0),
                ffmpeg_badge,
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center),
            column![
                text("PATH").size(9).color(theme::TEXT_LABEL),
                text(ffmpeg_path_str)
                    .size(11)
                    .color(theme::TEXT_SECONDARY)
                    .font(iced::Font::MONOSPACE),
            ]
            .spacing(2),
        ]
        .spacing(theme::SPACING),
    );

    // --- Feedback messages ---
    let mut feedback_col = column![].spacing(4);
    if let Some(err) = &state.error {
        feedback_col = feedback_col.push(
            text(format!("Error: {err}"))
                .size(12)
                .color(theme::STATUS_RED),
        );
    }
    if let Some(msg) = &state.success {
        feedback_col = feedback_col.push(
            text(msg.clone())
                .size(12)
                .color(theme::STATUS_GREEN),
        );
    }

    let header_row = widget::page_header("Updates", "Manage binary versions");

    let inner = column![
        header_row,
        widget::section_divider(),
        ytdlp_card,
        ffmpeg_card,
        feedback_col,
    ]
    .spacing(theme::SPACING_LG)
    .width(Length::Fill);

    container(inner)
        .padding(iced::Padding {
            top: 24.0,
            right: 28.0,
            bottom: 24.0,
            left: 28.0,
        })
        .into()
}
