use iced::widget::{column, container, row, text, Space};
use iced::{Element, Length, Task};

use crate::services::server_manager;
use crate::theme;
use crate::widget;

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct ServerTabState {
    pub running: bool,
    pub port: u16,
    pub idle_timeout: u64,
    pub pid: Option<u32>,
    pub start_error: Option<String>,
    pub stop_error: Option<String>,
}

impl ServerTabState {
    pub fn new(port: u16, idle_timeout: u64) -> Self {
        let pid = server_manager::read_pid();
        Self {
            running: false,
            port,
            idle_timeout,
            pid,
            start_error: None,
            stop_error: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Message
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum ServerMessage {
    Start,
    Stop,
    StartResult(Result<u32, String>),
    StopResult(Result<(), String>),
}

// ---------------------------------------------------------------------------
// Update
// ---------------------------------------------------------------------------

pub fn update(state: &mut ServerTabState, msg: ServerMessage) -> Task<ServerMessage> {
    match msg {
        ServerMessage::Start => {
            state.start_error = None;
            let port = state.port;
            let idle_timeout = state.idle_timeout;
            Task::perform(
                async move {
                    tokio::task::spawn_blocking(move || {
                        server_manager::start_server(port, idle_timeout)
                            .map_err(|e| e.to_string())
                    })
                    .await
                    .unwrap_or_else(|e| Err(e.to_string()))
                },
                ServerMessage::StartResult,
            )
        }
        ServerMessage::Stop => {
            state.stop_error = None;
            Task::perform(
                async move {
                    tokio::task::spawn_blocking(|| {
                        server_manager::stop_server().map_err(|e| e.to_string())
                    })
                    .await
                    .unwrap_or_else(|e| Err(e.to_string()))
                },
                ServerMessage::StopResult,
            )
        }
        ServerMessage::StartResult(result) => {
            match result {
                Ok(pid) => {
                    state.pid = Some(pid);
                    state.running = true;
                    state.start_error = None;
                }
                Err(e) => {
                    state.start_error = Some(e);
                }
            }
            Task::none()
        }
        ServerMessage::StopResult(result) => {
            match result {
                Ok(()) => {
                    state.running = false;
                    state.pid = None;
                    state.stop_error = None;
                }
                Err(e) => {
                    state.stop_error = Some(e);
                }
            }
            Task::none()
        }
    }
}

// ---------------------------------------------------------------------------
// View
// ---------------------------------------------------------------------------

pub fn view(state: &ServerTabState) -> Element<'_, ServerMessage> {
    // Main status card
    let dot_color = if state.running {
        theme::STATUS_GREEN
    } else {
        theme::STATUS_RED
    };

    let status_label = if state.running {
        format!("Running on port {}", state.port)
    } else {
        "Stopped".to_string()
    };

    let pid_label = match state.pid {
        Some(pid) => format!("PID {pid}"),
        None => "PID: N/A".to_string(),
    };

    let idle_label = format!("Idle timeout: {} secs", state.idle_timeout);

    let action_btn = if state.running {
        widget::danger_button("Stop Server", Some(ServerMessage::Stop))
    } else {
        widget::primary_button("Start Server", Some(ServerMessage::Start))
    };

    let status_card = widget::card(
        column![
            text("STATUS").size(10).color(theme::TEXT_SECTION),
            row![
                widget::status_dot(dot_color),
                text(status_label).size(14).color(theme::TEXT_PRIMARY),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center),
            container(
                column![
                    text(pid_label).size(11).color(theme::TEXT_LABEL),
                    text(idle_label).size(11).color(theme::TEXT_LABEL),
                ]
                .spacing(3)
            )
            .padding(iced::Padding { top: 4.0, right: 0.0, bottom: 4.0, left: 0.0 }),
            Space::with_height(4),
            action_btn,
        ]
        .spacing(8)
        .width(Length::Fill),
    );

    let mut content = column![
        widget::page_header("Server", "Control the media proxy server"),
        Space::with_height(theme::SPACING_LG),
        status_card,
    ]
    .spacing(theme::SPACING);

    if let Some(err) = &state.start_error {
        content = content.push(
            text(format!("Start error: {err}"))
                .size(12)
                .color(theme::STATUS_RED),
        );
    }
    if let Some(err) = &state.stop_error {
        content = content.push(
            text(format!("Stop error: {err}"))
                .size(12)
                .color(theme::STATUS_RED),
        );
    }

    content.into()
}
