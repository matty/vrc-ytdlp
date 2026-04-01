use std::path::PathBuf;

use iced::widget::{button, column, progress_bar, row, scrollable, text};
use iced::{Element, Length, Task};

use crate::services::cache_scanner::{self, CacheSummary};
use crate::theme;
use crate::widget;

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct CacheTabState {
    pub summary: Option<CacheSummary>,
    pub cache_dir: PathBuf,
    pub max_size_mb: u64,
    pub scanning: bool,
    pub error: Option<String>,
    pub clear_confirm: bool,
}

impl CacheTabState {
    pub fn new(cache_dir: PathBuf, max_size_mb: u64) -> Self {
        Self {
            summary: None,
            cache_dir,
            max_size_mb,
            scanning: false,
            error: None,
            clear_confirm: false,
        }
    }
}

// ---------------------------------------------------------------------------
// Message
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum CacheMessage {
    Scan,
    ScanResult(Result<CacheSummary, String>),
    DeleteEntry(PathBuf),
    DeleteResult(Result<PathBuf, String>),
    ClearAll,
    ConfirmClear,
    CancelClear,
    ClearResult(Result<(), String>),
}

// ---------------------------------------------------------------------------
// Update
// ---------------------------------------------------------------------------

pub fn update(state: &mut CacheTabState, msg: CacheMessage) -> Task<CacheMessage> {
    match msg {
        CacheMessage::Scan => {
            state.scanning = true;
            state.error = None;
            let dir = state.cache_dir.clone();
            Task::perform(
                async move {
                    tokio::task::spawn_blocking(move || {
                        cache_scanner::scan_cache(&dir).map_err(|e| e.to_string())
                    })
                    .await
                    .unwrap_or_else(|e| Err(e.to_string()))
                },
                CacheMessage::ScanResult,
            )
        }
        CacheMessage::ScanResult(result) => {
            state.scanning = false;
            match result {
                Ok(summary) => {
                    state.summary = Some(summary);
                    state.error = None;
                }
                Err(e) => state.error = Some(e),
            }
            Task::none()
        }
        CacheMessage::DeleteEntry(path) => {
            let p = path.clone();
            Task::perform(
                async move {
                    tokio::task::spawn_blocking(move || {
                        cache_scanner::delete_entry(&p).map_err(|e| e.to_string())
                    })
                    .await
                    .unwrap_or_else(|e| Err(e.to_string()))
                },
                move |result| CacheMessage::DeleteResult(result.map(|()| path.clone())),
            )
        }
        CacheMessage::DeleteResult(result) => {
            match result {
                Ok(deleted_path) => {
                    // Remove from summary in-place
                    if let Some(ref mut summary) = state.summary {
                        if let Some(idx) =
                            summary.entries.iter().position(|e| e.path == deleted_path)
                        {
                            let removed = summary.entries.remove(idx);
                            summary.total_size_bytes =
                                summary.total_size_bytes.saturating_sub(removed.size_bytes);
                            summary.entry_count = summary.entries.len();
                        }
                    }
                }
                Err(e) => state.error = Some(e),
            }
            Task::none()
        }
        CacheMessage::ClearAll => {
            state.clear_confirm = true;
            Task::none()
        }
        CacheMessage::CancelClear => {
            state.clear_confirm = false;
            Task::none()
        }
        CacheMessage::ConfirmClear => {
            state.clear_confirm = false;
            let dir = state.cache_dir.clone();
            Task::perform(
                async move {
                    tokio::task::spawn_blocking(move || {
                        cache_scanner::clear_cache(&dir).map_err(|e| e.to_string())
                    })
                    .await
                    .unwrap_or_else(|e| Err(e.to_string()))
                },
                CacheMessage::ClearResult,
            )
        }
        CacheMessage::ClearResult(result) => {
            match result {
                Ok(()) => {
                    state.summary = Some(CacheSummary {
                        total_size_bytes: 0,
                        entry_count: 0,
                        entries: vec![],
                    });
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

pub fn view(state: &CacheTabState) -> Element<CacheMessage> {
    let max_bytes = state.max_size_mb * 1024 * 1024;
    let total = state
        .summary
        .as_ref()
        .map(|s| s.total_size_bytes)
        .unwrap_or(0);
    let ratio = if max_bytes > 0 {
        (total as f32 / max_bytes as f32).min(1.0)
    } else {
        0.0
    };

    let usage_text = format!(
        "{} / {} MB used",
        cache_scanner::format_size(total),
        state.max_size_mb
    );

    let scan_btn = if state.scanning {
        button("Scanning...").style(button::secondary)
    } else {
        button("Refresh")
            .on_press(CacheMessage::Scan)
            .style(button::primary)
    };

    let clear_btn = if state.clear_confirm {
        row![
            button("Yes, clear all")
                .on_press(CacheMessage::ConfirmClear)
                .style(button::danger),
            button("Cancel")
                .on_press(CacheMessage::CancelClear)
                .style(button::secondary),
        ]
        .spacing(8)
    } else {
        row![button("Clear All")
            .on_press(CacheMessage::ClearAll)
            .style(button::danger),]
    };

    let header = column![
        widget::section_header("Cache Management"),
        row![scan_btn, clear_btn].spacing(theme::SPACING),
        text(usage_text).size(14),
        progress_bar(0.0..=1.0, ratio),
    ]
    .spacing(theme::SPACING);

    let mut content = column![header].spacing(theme::SPACING);

    if let Some(err) = &state.error {
        content = content.push(text(format!("Error: {err}")).size(13).color(theme::RED));
    }

    if let Some(ref summary) = state.summary {
        let count_text = format!("{} file(s)", summary.entry_count);
        content = content.push(text(count_text).size(13).color(theme::GREY));

        let mut file_list = column![].spacing(4);
        for entry in &summary.entries {
            let size = cache_scanner::format_size(entry.size_bytes);
            let label = format!("{} ({})", entry.file_name, size);
            let entry_row = row![
                text(label).size(13).width(Length::Fill),
                button("Delete")
                    .on_press(CacheMessage::DeleteEntry(entry.path.clone()))
                    .style(button::danger),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center);
            file_list = file_list.push(widget::card(entry_row));
        }
        content = content.push(scrollable(file_list).height(Length::Fill));
    }

    content.into()
}
