use iced::widget::{button, column, row, scrollable, text, text_input};
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
        let app_dir = match crate::paths::exe_dir() {
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
    let toolbar = row![
        text_input("Filter...", &state.filter).on_input(LogsMessage::FilterChanged),
        widget::labeled_toggle("Auto-scroll", state.auto_scroll, LogsMessage::ToggleAutoScroll),
        button("Reload")
            .on_press(LogsMessage::Reload)
            .style(button::secondary),
    ]
    .spacing(theme::SPACING)
    .align_y(iced::Alignment::Center);

    let filter_lower = state.filter.to_lowercase();
    let filtered: Vec<&LogLine> = state
        .lines
        .iter()
        .filter(|l| filter_lower.is_empty() || l.text.to_lowercase().contains(&filter_lower))
        .collect();

    let mut log_col = column![].spacing(2);
    for line in &filtered {
        let color = match line.level {
            LogLevel::Error => theme::RED,
            LogLevel::Warn => theme::YELLOW,
            LogLevel::Debug => theme::GREY,
            LogLevel::Info | LogLevel::Other => iced::Color::WHITE,
        };
        log_col = log_col.push(
            text(line.text.clone())
                .size(12)
                .font(iced::Font::MONOSPACE)
                .color(color),
        );
    }

    let log_scroll = scrollable(log_col).height(Length::Fill);

    let mut content = column![widget::section_header("Logs"), toolbar, log_scroll]
        .spacing(theme::SPACING)
        .height(Length::Fill);

    if let Some(err) = &state.error {
        content = content.push(text(format!("Error: {err}")).size(13).color(theme::RED));
    }

    content.into()
}
