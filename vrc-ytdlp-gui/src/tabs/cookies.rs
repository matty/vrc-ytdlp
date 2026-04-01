use std::path::PathBuf;

use iced::widget::{button, column, pick_list, row, text};
use iced::{Element, Task};

use crate::services::cookie_extractor::{self, CookieStatus, BROWSERS};
use crate::theme;
use crate::widget;

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct CookiesTabState {
    pub status: CookieStatus,
    pub browser: String,
    pub extracting: bool,
    pub error: Option<String>,
    pub success: Option<String>,
    pub ytdlp_path: PathBuf,
}

impl CookiesTabState {
    pub fn new(browser: String, ytdlp_path: PathBuf) -> Self {
        let app_dir = crate::paths::exe_dir().unwrap_or_else(|_| PathBuf::from("."));
        let status = cookie_extractor::check_cookies(&app_dir);
        Self {
            status,
            browser,
            extracting: false,
            error: None,
            success: None,
            ytdlp_path,
        }
    }
}

// ---------------------------------------------------------------------------
// Message
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum CookiesMessage {
    BrowserSelected(String),
    Extract,
    ExtractResult(Result<String, String>),
    Refresh,
}

// ---------------------------------------------------------------------------
// Update
// ---------------------------------------------------------------------------

pub fn update(state: &mut CookiesTabState, msg: CookiesMessage) -> Task<CookiesMessage> {
    match msg {
        CookiesMessage::BrowserSelected(b) => {
            state.browser = b;
            Task::none()
        }
        CookiesMessage::Extract => {
            state.extracting = true;
            state.error = None;
            state.success = None;
            let path = state.ytdlp_path.clone();
            let browser = state.browser.clone();
            Task::perform(
                async move {
                    cookie_extractor::extract_cookies(&path, &browser)
                        .await
                        .map_err(|e| e.to_string())
                },
                CookiesMessage::ExtractResult,
            )
        }
        CookiesMessage::ExtractResult(result) => {
            state.extracting = false;
            match result {
                Ok(msg) => {
                    state.success = Some(msg);
                    // Refresh status
                    let app_dir =
                        crate::paths::exe_dir().unwrap_or_else(|_| PathBuf::from("."));
                    state.status = cookie_extractor::check_cookies(&app_dir);
                }
                Err(e) => state.error = Some(e),
            }
            Task::none()
        }
        CookiesMessage::Refresh => {
            let app_dir = crate::paths::exe_dir().unwrap_or_else(|_| PathBuf::from("."));
            state.status = cookie_extractor::check_cookies(&app_dir);
            state.error = None;
            state.success = None;
            Task::none()
        }
    }
}

// ---------------------------------------------------------------------------
// View
// ---------------------------------------------------------------------------

pub fn view(state: &CookiesTabState) -> Element<CookiesMessage> {
    let dot_color = if state.status.exists {
        theme::GREEN
    } else {
        theme::YELLOW
    };
    let status_text = if state.status.exists {
        "cookies.txt found"
    } else {
        "No cookies.txt"
    };

    let age_text = state
        .status
        .age_description
        .as_deref()
        .unwrap_or("N/A");

    let browser_options: Vec<String> = BROWSERS.iter().map(|s| s.to_string()).collect();
    let selected: Option<String> = Some(state.browser.clone());

    let extract_btn = if state.extracting {
        button("Extracting...").style(button::secondary)
    } else {
        button("Extract Cookies")
            .on_press(CookiesMessage::Extract)
            .style(button::primary)
    };

    let status_card = widget::card(
        column![
            row![widget::status_dot(dot_color), text(status_text).size(16),]
                .spacing(8)
                .align_y(iced::Alignment::Center),
            text(format!("Last modified: {age_text}"))
                .size(14)
                .color(theme::GREY),
        ]
        .spacing(8),
    );

    let extract_card = widget::card(
        column![
            text("Extract cookies from browser").size(14),
            row![
                pick_list(browser_options, selected, CookiesMessage::BrowserSelected).width(160),
                extract_btn,
            ]
            .spacing(theme::SPACING)
            .align_y(iced::Alignment::Center),
        ]
        .spacing(8),
    );

    let mut content = column![
        widget::section_header("Cookies"),
        status_card,
        extract_card,
        button("Refresh Status")
            .on_press(CookiesMessage::Refresh)
            .style(button::secondary),
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
