use iced::widget::{button, column, container, progress_bar, row, text, Space};
use iced::{Element, Length};

use crate::services::cache_scanner;
use crate::sidebar::Tab;
use crate::theme;
use crate::widget;

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct DashboardState {
    pub server_running: bool,
    pub server_port: u16,
    pub cache_size_bytes: u64,
    pub cache_max_mb: u64,
    pub ytdlp_version: Option<String>,
    pub cookies_exist: bool,
}

impl DashboardState {
    pub fn new(port: u16, cache_max_mb: u64) -> Self {
        Self {
            server_running: false,
            server_port: port,
            cache_size_bytes: 0,
            cache_max_mb,
            ytdlp_version: None,
            cookies_exist: false,
        }
    }
}

// ---------------------------------------------------------------------------
// Message
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum DashboardMessage {
    GoToTab(Tab),
}

// ---------------------------------------------------------------------------
// View
// ---------------------------------------------------------------------------

pub fn view(state: &DashboardState) -> Element<'_, DashboardMessage> {
    let server_card = {
        let dot_color = if state.server_running {
            theme::GREEN
        } else {
            theme::RED
        };
        let status_text = if state.server_running {
            format!("Running on port {}", state.server_port)
        } else {
            "Stopped".to_string()
        };
        widget::card(
            column![
                row![
                    widget::status_dot(dot_color),
                    text("Server").size(16),
                ]
                .spacing(8)
                .align_y(iced::Alignment::Center),
                text(status_text).size(13).color(theme::GREY),
                button("Details")
                    .on_press(DashboardMessage::GoToTab(Tab::Server))
                    .style(button::secondary),
            ]
            .spacing(8)
            .width(Length::Fill),
        )
    };

    let cache_card = {
        let max_bytes = state.cache_max_mb * 1024 * 1024;
        let ratio = if max_bytes > 0 {
            state.cache_size_bytes as f32 / max_bytes as f32
        } else {
            0.0
        };
        let usage_text = format!(
            "{} / {} MB",
            cache_scanner::format_size(state.cache_size_bytes),
            state.cache_max_mb
        );
        widget::card(
            column![
                text("Cache").size(16),
                text(usage_text).size(13).color(theme::GREY),
                progress_bar(0.0..=1.0, ratio.min(1.0)),
                button("Details")
                    .on_press(DashboardMessage::GoToTab(Tab::Cache))
                    .style(button::secondary),
            ]
            .spacing(8)
            .width(Length::Fill),
        )
    };

    let ytdlp_card = {
        let version_text = state
            .ytdlp_version
            .as_deref()
            .unwrap_or("Not installed");
        let dot_color = if state.ytdlp_version.is_some() {
            theme::GREEN
        } else {
            theme::RED
        };
        widget::card(
            column![
                row![
                    widget::status_dot(dot_color),
                    text("yt-dlp").size(16),
                ]
                .spacing(8)
                .align_y(iced::Alignment::Center),
                text(version_text.to_owned()).size(13).color(theme::GREY),
                button("Details")
                    .on_press(DashboardMessage::GoToTab(Tab::Updates))
                    .style(button::secondary),
            ]
            .spacing(8)
            .width(Length::Fill),
        )
    };

    let cookies_card = {
        let (dot_color, status_text) = if state.cookies_exist {
            (theme::GREEN, "cookies.txt found")
        } else {
            (theme::YELLOW, "No cookies.txt")
        };
        widget::card(
            column![
                row![
                    widget::status_dot(dot_color),
                    text("Cookies").size(16),
                ]
                .spacing(8)
                .align_y(iced::Alignment::Center),
                text(status_text).size(13).color(theme::GREY),
                button("Details")
                    .on_press(DashboardMessage::GoToTab(Tab::Cookies))
                    .style(button::secondary),
            ]
            .spacing(8)
            .width(Length::Fill),
        )
    };

    let grid = column![
        row![
            container(server_card).width(Length::Fill),
            Space::with_width(theme::SPACING),
            container(cache_card).width(Length::Fill),
        ],
        row![
            container(ytdlp_card).width(Length::Fill),
            Space::with_width(theme::SPACING),
            container(cookies_card).width(Length::Fill),
        ],
    ]
    .spacing(theme::SPACING);

    column![
        widget::section_header("Dashboard"),
        grid,
    ]
    .spacing(theme::SPACING)
    .into()
}
