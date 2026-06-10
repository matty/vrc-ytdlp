use iced::widget::{column, container, row, scrollable, text, text_input};
use iced::{Element, Length};

use crate::services::log_watcher::{LogLevel, LogLine, LogTailer};
use crate::theme;
use crate::widget;

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct LogsTabState {
    pub lines: Vec<LogLine>,
    pub filter: String,
    pub auto_scroll: bool,
    pub tailer: Option<LogTailer>,
    pub error: Option<String>,
}

impl LogsTabState {
    pub fn new() -> Self {
        Self {
            lines: Vec::new(),
            filter: String::new(),
            auto_scroll: true,
            tailer: None,
            error: None,
        }
    }

    /// Initialise the tailer from the app directory. Called when the Logs tab
    /// becomes active or on reload.
    pub fn init_tailer(&mut self) {
        let app_dir = match crate::paths::app_dir() {
            Ok(d) => d,
            Err(e) => {
                self.error = Some(e.to_string());
                return;
            }
        };

        match crate::services::log_watcher::find_latest_log(&app_dir) {
            Some(path) => {
                let mut tailer = LogTailer::from_start(path);
                self.lines = tailer.read_new_lines();
                self.tailer = Some(tailer);
                self.error = None;
            }
            None => {
                self.error = Some("No log file found".to_string());
                self.tailer = None;
            }
        }
    }

    /// Read any new lines from the tailer (called on poll tick).
    pub fn poll(&mut self) {
        if let Some(ref mut tailer) = self.tailer {
            let new_lines = tailer.read_new_lines();
            if !new_lines.is_empty() {
                self.lines.extend(new_lines);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Message
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum LogsMessage {
    FilterChanged(String),
    ToggleAutoScroll(bool),
    Reload,
}

// ---------------------------------------------------------------------------
// Update
// ---------------------------------------------------------------------------

pub fn update(state: &mut LogsTabState, msg: LogsMessage) {
    match msg {
        LogsMessage::FilterChanged(f) => state.filter = f,
        LogsMessage::ToggleAutoScroll(v) => state.auto_scroll = v,
        LogsMessage::Reload => {
            state.lines.clear();
            state.init_tailer();
        }
    }
}

// ---------------------------------------------------------------------------
// View
// ---------------------------------------------------------------------------

pub fn view(state: &LogsTabState) -> Element<'_, LogsMessage> {
    // --- Toolbar ---
    let filter_input = container(
        text_input("Filter...", &state.filter)
            .on_input(LogsMessage::FilterChanged)
            .size(12)
            .padding(iced::Padding {
                top: 6.0,
                right: 10.0,
                bottom: 6.0,
                left: 10.0,
            }),
    )
    .style(|_: &iced::Theme| iced::widget::container::Style {
        background: Some(iced::Background::Color(theme::BG_INPUT)),
        border: iced::Border {
            width: 1.0,
            radius: theme::INPUT_RADIUS.into(),
            color: theme::BORDER_INPUT,
        },
        ..Default::default()
    })
    .width(Length::Fill);

    let toolbar = row![
        filter_input,
        widget::labeled_toggle(
            "Auto-scroll",
            "Follow new log entries",
            state.auto_scroll,
            LogsMessage::ToggleAutoScroll,
        ),
        widget::secondary_button("Reload", Some(LogsMessage::Reload)),
    ]
    .spacing(theme::SPACING)
    .align_y(iced::Alignment::Center);

    // --- Log lines ---
    let filter_lower = state.filter.to_lowercase();
    let filtered: Vec<&LogLine> = state
        .lines
        .iter()
        .filter(|l| filter_lower.is_empty() || l.text.to_lowercase().contains(&filter_lower))
        .collect();

    let mut log_col = column![].spacing(1);
    for line in &filtered {
        let color = match line.level {
            LogLevel::Error => theme::STATUS_RED,
            LogLevel::Warn => theme::STATUS_YELLOW,
            LogLevel::Debug => theme::TEXT_SECONDARY,
            LogLevel::Info | LogLevel::Other => theme::TEXT_PRIMARY,
        };
        log_col = log_col.push(
            text(line.text.clone())
                .size(11)
                .font(iced::Font::MONOSPACE)
                .color(color),
        );
    }

    // Error overlay at bottom
    let error_el: Element<'_, LogsMessage> = if let Some(err) = &state.error {
        text(format!("Error: {err}"))
            .size(12)
            .color(theme::STATUS_RED)
            .into()
    } else {
        iced::widget::Space::new(0, 0).into()
    };

    let log_area = container(
        scrollable(
            container(log_col)
                .padding(iced::Padding {
                    top: 10.0,
                    right: 14.0,
                    bottom: 10.0,
                    left: 14.0,
                })
                .width(Length::Fill),
        )
        .height(Length::Fill),
    )
    .style(|_: &iced::Theme| iced::widget::container::Style {
        background: Some(iced::Background::Color(theme::BG_INPUT)),
        border: iced::Border {
            width: 1.0,
            radius: theme::INPUT_RADIUS.into(),
            color: theme::BORDER_INPUT,
        },
        ..Default::default()
    })
    .width(Length::Fill)
    .height(Length::Fill);

    let header_row = row![
        widget::page_header("Logs", "View application log output"),
    ]
    .align_y(iced::Alignment::Center);

    let inner = column![
        header_row,
        widget::section_divider(),
        toolbar,
        log_area,
        error_el,
    ]
    .spacing(theme::SPACING_LG)
    .width(Length::Fill)
    .height(Length::Fill);

    container(inner)
        .padding(iced::Padding {
            top: 24.0,
            right: 28.0,
            bottom: 24.0,
            left: 28.0,
        })
        .height(Length::Fill)
        .into()
}
