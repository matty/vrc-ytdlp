use std::path::PathBuf;

use iced::widget::{column, container, pick_list, row, text};
use iced::{Element, Length, Task};

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
        let app_dir = crate::paths::app_dir().unwrap_or_else(|_| PathBuf::from("."));
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
                        crate::paths::app_dir().unwrap_or_else(|_| PathBuf::from("."));
                    state.status = cookie_extractor::check_cookies(&app_dir);
                }
                Err(e) => state.error = Some(e),
            }
            Task::none()
        }
        CookiesMessage::Refresh => {
            let app_dir = crate::paths::app_dir().unwrap_or_else(|_| PathBuf::from("."));
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

pub fn view(state: &CookiesTabState) -> Element<'_, CookiesMessage> {
    let dot_color = if state.status.exists {
        theme::STATUS_GREEN
    } else {
        theme::STATUS_YELLOW
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

    // --- Status card ---
    let status_badge: Element<'_, CookiesMessage> = if state.status.exists {
        widget::pill_badge("Found", theme::STATUS_GREEN)
    } else {
        widget::pill_badge("Not found", theme::STATUS_YELLOW)
    };

    let status_card = widget::card(
        column![
            row![
                widget::status_dot(dot_color),
                text(status_text)
                    .size(13)
                    .color(theme::TEXT_PRIMARY),
                iced::widget::Space::new(Length::Fill, 0),
                status_badge,
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center),
            column![
                text("LAST MODIFIED").size(9).color(theme::TEXT_LABEL),
                text(age_text).size(12).color(theme::TEXT_SECONDARY),
            ]
            .spacing(2),
        ]
        .spacing(theme::SPACING),
    );

    // --- Extract section ---
    let browser_options: Vec<String> = BROWSERS.iter().map(|s| s.to_string()).collect();
    let selected: Option<String> = Some(state.browser.clone());

    let extract_btn = if state.extracting {
        widget::primary_button("Extracting...", None)
    } else {
        widget::primary_button("Extract Cookies", Some(CookiesMessage::Extract))
    };

    let browser_picker = column![
        text("BROWSER").size(9).color(theme::TEXT_LABEL),
        widget::input_container(
            pick_list(browser_options, selected, CookiesMessage::BrowserSelected)
                .width(Length::Fill),
        ),
    ]
    .spacing(5)
    .width(160);

    let extract_card = widget::card(
        column![
            text("Extract cookies from browser")
                .size(13)
                .color(theme::TEXT_PRIMARY),
            row![browser_picker, extract_btn]
                .spacing(theme::SPACING)
                .align_y(iced::Alignment::End),
        ]
        .spacing(theme::SPACING),
    );

    // --- Feedback ---
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

    // --- Header row ---
    let header_row = row![
        widget::page_header("Cookies", "Manage browser cookie authentication"),
        iced::widget::Space::new(Length::Fill, 0),
        widget::secondary_button("Refresh Status", Some(CookiesMessage::Refresh)),
    ]
    .spacing(theme::SPACING_SM)
    .align_y(iced::Alignment::Center);

    let inner = column![
        header_row,
        widget::section_divider(),
        status_card,
        extract_card,
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
