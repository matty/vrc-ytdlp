use iced::widget::{column, container, progress_bar, row, text, Space};
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
    // -- Server card --
    let server_card = {
        let dot_color = if state.server_running {
            theme::STATUS_GREEN
        } else {
            theme::STATUS_RED
        };
        let status_label = if state.server_running { "Running" } else { "Stopped" };
        let port_detail = format!("Port {}", state.server_port);

        widget::card(
            column![
                text("SERVER").size(10).color(theme::TEXT_SECTION),
                row![
                    widget::status_dot(dot_color),
                    text(status_label).size(14).color(theme::TEXT_PRIMARY),
                ]
                .spacing(6)
                .align_y(iced::Alignment::Center),
                text(port_detail).size(10).color(theme::TEXT_MUTED),
                Space::with_height(4),
                widget::secondary_button(
                    "Details",
                    Some(DashboardMessage::GoToTab(Tab::Server)),
                ),
            ]
            .spacing(6)
            .width(Length::Fill),
        )
    };

    // -- Cache card --
    let cache_card = {
        let max_bytes = state.cache_max_mb * 1024 * 1024;
        let ratio = if max_bytes > 0 {
            (state.cache_size_bytes as f32 / max_bytes as f32).min(1.0)
        } else {
            0.0
        };
        let size_label = cache_scanner::format_size(state.cache_size_bytes);
        let detail = format!("{}% of {} GB", (ratio * 100.0) as u32, state.cache_max_mb / 1024);

        widget::card(
            column![
                text("CACHE").size(10).color(theme::TEXT_SECTION),
                text(size_label).size(14).color(theme::TEXT_PRIMARY),
                container(progress_bar(0.0..=1.0, ratio))
                    .height(4)
                    .width(Length::Fill),
                text(detail).size(10).color(theme::TEXT_MUTED),
                Space::with_height(4),
                widget::secondary_button(
                    "Details",
                    Some(DashboardMessage::GoToTab(Tab::Cache)),
                ),
            ]
            .spacing(6)
            .width(Length::Fill),
        )
    };

    // -- yt-dlp card --
    let ytdlp_card = {
        let version_text = state
            .ytdlp_version
            .as_deref()
            .unwrap_or("Not installed");
        let badge_label = if state.ytdlp_version.is_some() {
            "Current"
        } else {
            "Not installed"
        };
        let badge_color = if state.ytdlp_version.is_some() {
            theme::STATUS_GREEN
        } else {
            theme::STATUS_RED
        };

        widget::card(
            column![
                text("YT-DLP").size(10).color(theme::TEXT_SECTION),
                text(version_text.to_owned()).size(14).color(theme::TEXT_PRIMARY),
                widget::pill_badge(badge_label, badge_color),
                Space::with_height(4),
                widget::secondary_button(
                    "Details",
                    Some(DashboardMessage::GoToTab(Tab::Updates)),
                ),
            ]
            .spacing(6)
            .width(Length::Fill),
        )
    };

    // -- Cookies card --
    let cookies_card = {
        let (dot_color, status_label, detail_label) = if state.cookies_exist {
            (theme::STATUS_GREEN, "Available", "cookies.txt found")
        } else {
            (theme::STATUS_RED, "Missing", "No cookies.txt")
        };

        widget::card(
            column![
                text("COOKIES").size(10).color(theme::TEXT_SECTION),
                row![
                    widget::status_dot(dot_color),
                    text(status_label).size(14).color(theme::TEXT_PRIMARY),
                ]
                .spacing(6)
                .align_y(iced::Alignment::Center),
                text(detail_label).size(10).color(theme::TEXT_MUTED),
                Space::with_height(4),
                widget::secondary_button(
                    "Details",
                    Some(DashboardMessage::GoToTab(Tab::Cookies)),
                ),
            ]
            .spacing(6)
            .width(Length::Fill),
        )
    };

    // 2x2 grid
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
        widget::page_header("Dashboard", "System overview at a glance"),
        Space::with_height(theme::SPACING_LG),
        grid,
    ]
    .spacing(theme::SPACING)
    .into()
}
