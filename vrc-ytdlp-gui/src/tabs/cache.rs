use std::path::PathBuf;

use iced::widget::{column, container, progress_bar, row, scrollable, text};
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

pub fn view(state: &CacheTabState) -> Element<'_, CacheMessage> {
    let max_bytes = state.max_size_mb * 1024 * 1024;
    let total = state
        .summary
        .as_ref()
        .map(|s| s.total_size_bytes)
        .unwrap_or(0);
    let entry_count = state
        .summary
        .as_ref()
        .map(|s| s.entry_count)
        .unwrap_or(0);
    let ratio = if max_bytes > 0 {
        (total as f32 / max_bytes as f32).min(1.0)
    } else {
        0.0
    };

    let used_str = cache_scanner::format_size(total);
    let pct = (ratio * 100.0).round() as u32;

    // --- Toolbar buttons ---
    let refresh_btn = if state.scanning {
        widget::primary_button("Scanning...", None)
    } else {
        widget::primary_button("Refresh", Some(CacheMessage::Scan))
    };

    let clear_area: Element<'_, CacheMessage> = if state.clear_confirm {
        row![
            text("Are you sure?").size(12).color(theme::STATUS_RED),
            widget::danger_button("Yes, clear all", Some(CacheMessage::ConfirmClear)),
            widget::secondary_button("Cancel", Some(CacheMessage::CancelClear)),
        ]
        .spacing(theme::SPACING_SM)
        .align_y(iced::Alignment::Center)
        .into()
    } else {
        widget::danger_button("Clear All", Some(CacheMessage::ClearAll))
    };

    // --- Usage summary card ---
    let usage_card = widget::card(
        column![
            row![
                text(used_str.clone())
                    .size(22)
                    .color(theme::TEXT_PRIMARY),
                iced::widget::Space::new(Length::Fill, 0),
                text(format!("{pct}% of {} MB · {entry_count} files", state.max_size_mb))
                    .size(11)
                    .color(theme::TEXT_LABEL),
            ]
            .align_y(iced::Alignment::End),
            progress_bar(0.0..=1.0, ratio),
        ]
        .spacing(10),
    );

    // --- Header with buttons ---
    let header_row = row![
        widget::page_header("Cache", "Manage cached video files"),
        iced::widget::Space::new(Length::Fill, 0),
        refresh_btn,
        clear_area,
    ]
    .spacing(theme::SPACING_SM)
    .align_y(iced::Alignment::Center);

    // --- Error row ---
    let error_el: Element<'_, CacheMessage> = if let Some(err) = &state.error {
        text(format!("Error: {err}"))
            .size(12)
            .color(theme::STATUS_RED)
            .into()
    } else {
        iced::widget::Space::new(0, 0).into()
    };

    // --- File list ---
    let mut file_list = column![].spacing(6);
    if let Some(ref summary) = state.summary {
        for entry in &summary.entries {
            let size_str = cache_scanner::format_size(entry.size_bytes);
            let entry_row = row![
                column![
                    text(entry.file_name.clone())
                        .size(12)
                        .color(theme::TEXT_PRIMARY),
                    text(size_str)
                        .size(10)
                        .color(theme::TEXT_LABEL),
                ]
                .spacing(2)
                .width(Length::Fill),
                widget::danger_button(
                    "Delete",
                    Some(CacheMessage::DeleteEntry(entry.path.clone())),
                ),
            ]
            .spacing(theme::SPACING)
            .align_y(iced::Alignment::Center);
            file_list = file_list.push(widget::card(entry_row));
        }
    }

    let inner = column![
        header_row,
        widget::section_divider(),
        usage_card,
        error_el,
        scrollable(file_list).height(Length::Fill),
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
