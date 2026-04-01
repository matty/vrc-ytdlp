use iced::widget::{button, column, row, text};
use iced::{Element, Task};

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
    let dot_color = if state.running {
        theme::GREEN
    } else {
        theme::RED
    };
    let status_label = if state.running { "Running" } else { "Stopped" };

    let status_row = row![
        widget::status_dot(dot_color),
        text(format!("Server: {status_label}")).size(16),
    ]
    .spacing(8)
    .align_y(iced::Alignment::Center);

    let info = column![
        text(format!("Port: {}", state.port)).size(14).color(theme::GREY),
        text(format!("Idle timeout: {} secs", state.idle_timeout))
            .size(14)
            .color(theme::GREY),
    ]
    .spacing(4);

    let pid_text = match state.pid {
        Some(pid) => format!("PID: {pid}"),
        None => "PID: N/A".to_string(),
    };

    let action_btn = if state.running {
        button("Stop Server")
            .on_press(ServerMessage::Stop)
            .style(button::danger)
    } else {
        button("Start Server")
            .on_press(ServerMessage::Start)
            .style(button::primary)
    };

    let mut content = column![
        widget::section_header("Server Control"),
        widget::card(
            column![status_row, info, text(pid_text).size(14).color(theme::GREY), action_btn,]
                .spacing(theme::SPACING),
        ),
    ]
    .spacing(theme::SPACING);

    if let Some(err) = &state.start_error {
        content = content.push(text(format!("Start error: {err}")).size(13).color(theme::RED));
    }
    if let Some(err) = &state.stop_error {
        content = content.push(text(format!("Stop error: {err}")).size(13).color(theme::RED));
    }

    content.into()
}
