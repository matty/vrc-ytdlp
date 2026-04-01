use std::path::PathBuf;

use iced::widget::{button, column, container, pick_list, row, text, text_input, toggler};
use iced::{Element, Length, Task};

use crate::config::Config;
use crate::paths;
use crate::services::cookie_extractor::BROWSERS;
use crate::services::downloader;
use crate::theme;

// ---------------------------------------------------------------------------
// Step
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WizardStep {
    Welcome,
    Binaries,
    BasicConfig,
    Done,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct WizardState {
    pub step: WizardStep,

    // Binary detection
    pub ytdlp_found: bool,
    pub ffmpeg_found: bool,
    pub ffprobe_found: bool,
    pub ytdlp_downloading: bool,
    pub ytdlp_download_error: Option<String>,

    // Basic config fields
    pub server_port: String,
    pub cookies_enabled: bool,
    pub cookies_browser: String,
    pub cache_max_mb: String,

    // Resolved paths
    pub ytdlp_path: PathBuf,
    pub ffmpeg_path: PathBuf,
}

impl WizardState {
    pub fn new() -> Self {
        let exe_dir = paths::exe_dir().unwrap_or_else(|_| PathBuf::from("."));

        let (ytdlp_name, ffmpeg_name, ffprobe_name) = if cfg!(windows) {
            ("yt-dlp.exe", "ffmpeg.exe", "ffprobe.exe")
        } else {
            ("yt-dlp", "ffmpeg", "ffprobe")
        };

        let ytdlp_path = exe_dir.join(ytdlp_name);
        let ffmpeg_path = exe_dir.join(ffmpeg_name);

        let ytdlp_found = ytdlp_path.exists();
        let ffmpeg_found = ffmpeg_path.exists();
        let ffprobe_found = exe_dir.join(ffprobe_name).exists();

        Self {
            step: WizardStep::Welcome,
            ytdlp_found,
            ffmpeg_found,
            ffprobe_found,
            ytdlp_downloading: false,
            ytdlp_download_error: None,
            server_port: "9851".into(),
            cookies_enabled: false,
            cookies_browser: "firefox".into(),
            cache_max_mb: "2048".into(),
            ytdlp_path,
            ffmpeg_path,
        }
    }
}

// ---------------------------------------------------------------------------
// Messages
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum WizardMessage {
    Next,
    Back,
    DownloadYtdlp,
    YtdlpDownloaded(Result<String, String>),
    PortChanged(String),
    CookiesToggled(bool),
    BrowserSelected(String),
    CacheMaxChanged(String),
    FinishWizard,
}

// ---------------------------------------------------------------------------
// Update
// ---------------------------------------------------------------------------

pub fn update(state: &mut WizardState, msg: WizardMessage) -> (Option<Config>, Task<WizardMessage>) {
    match msg {
        WizardMessage::Next => {
            state.step = match state.step {
                WizardStep::Welcome => WizardStep::Binaries,
                WizardStep::Binaries => WizardStep::BasicConfig,
                WizardStep::BasicConfig => WizardStep::Done,
                WizardStep::Done => WizardStep::Done,
            };
            (None, Task::none())
        }

        WizardMessage::Back => {
            state.step = match state.step {
                WizardStep::Welcome => WizardStep::Welcome,
                WizardStep::Binaries => WizardStep::Welcome,
                WizardStep::BasicConfig => WizardStep::Binaries,
                WizardStep::Done => WizardStep::BasicConfig,
            };
            (None, Task::none())
        }

        WizardMessage::DownloadYtdlp => {
            state.ytdlp_downloading = true;
            state.ytdlp_download_error = None;
            let ytdlp_path = state.ytdlp_path.clone();
            let task = Task::perform(
                async move { downloader::download_latest(&ytdlp_path).await.map_err(|e| e.to_string()) },
                WizardMessage::YtdlpDownloaded,
            );
            (None, task)
        }

        WizardMessage::YtdlpDownloaded(result) => {
            state.ytdlp_downloading = false;
            match result {
                Ok(_version) => {
                    state.ytdlp_found = true;
                    state.ytdlp_download_error = None;
                }
                Err(e) => {
                    state.ytdlp_download_error = Some(e);
                }
            }
            (None, Task::none())
        }

        WizardMessage::PortChanged(val) => {
            state.server_port = val;
            (None, Task::none())
        }

        WizardMessage::CookiesToggled(val) => {
            state.cookies_enabled = val;
            (None, Task::none())
        }

        WizardMessage::BrowserSelected(val) => {
            state.cookies_browser = val;
            (None, Task::none())
        }

        WizardMessage::CacheMaxChanged(val) => {
            state.cache_max_mb = val;
            (None, Task::none())
        }

        WizardMessage::FinishWizard => {
            let port: u16 = state.server_port.parse().unwrap_or(9851);
            let cache_mb: u64 = state.cache_max_mb.parse().unwrap_or(2048);

            let mut config = Config::default();
            config.server_port = port;
            config.cookies = state.cookies_enabled;
            config.cookies_browser = state.cookies_browser.clone();
            config.cache_max_size_mb = cache_mb;
            config.ytdlp_location = state.ytdlp_path.to_string_lossy().into_owned();
            config.ffmpeg_location = state.ffmpeg_path.to_string_lossy().into_owned();

            (Some(config), Task::none())
        }
    }
}

// ---------------------------------------------------------------------------
// View helpers
// ---------------------------------------------------------------------------

fn status_dot<'a>(found: bool, label: &'a str) -> Element<'a, WizardMessage> {
    let (color, symbol) = if found {
        (theme::GREEN, "●")
    } else {
        (theme::RED, "●")
    };

    row![
        text(symbol).color(color),
        text(format!("  {label}"))
    ]
    .into()
}

// ---------------------------------------------------------------------------
// View
// ---------------------------------------------------------------------------

pub fn view(state: &WizardState) -> Element<'_, WizardMessage> {
    let content: Element<WizardMessage> = match state.step {
        WizardStep::Welcome => view_welcome(),
        WizardStep::Binaries => view_binaries(state),
        WizardStep::BasicConfig => view_basic_config(state),
        WizardStep::Done => view_done(state),
    };

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
}

fn view_welcome<'a>() -> Element<'a, WizardMessage> {
    column![
        text("Welcome to VRC-YtDlp").size(28),
        text("This wizard will help you configure the application for first use.").size(14),
        text("Click \"Get Started\" to begin.").size(14),
        button("Get Started").on_press(WizardMessage::Next),
    ]
    .spacing(theme::SPACING)
    .padding(theme::PADDING)
    .max_width(500)
    .into()
}

fn view_binaries(state: &WizardState) -> Element<'_, WizardMessage> {
    let mut col = column![
        text("Binary Detection").size(24),
        text("The following binaries are required:").size(14),
        status_dot(state.ytdlp_found, "yt-dlp"),
        status_dot(state.ffmpeg_found, "ffmpeg"),
        status_dot(state.ffprobe_found, "ffprobe"),
    ]
    .spacing(theme::SPACING)
    .padding(theme::PADDING)
    .max_width(500);

    if !state.ytdlp_found {
        if state.ytdlp_downloading {
            col = col.push(text("Downloading yt-dlp..."));
        } else {
            col = col.push(
                button("Download yt-dlp").on_press(WizardMessage::DownloadYtdlp),
            );
        }
        if let Some(err) = &state.ytdlp_download_error {
            col = col.push(text(format!("Error: {err}")).color(theme::RED));
        }
    }

    if !state.ffmpeg_found || !state.ffprobe_found {
        col = col.push(
            text("ffmpeg/ffprobe not found. Please install them and ensure they are in your PATH or next to this executable.")
                .color(theme::YELLOW)
                .size(12),
        );
    }

    col = col.push(
        row![
            button("Back").on_press(WizardMessage::Back),
            button("Next").on_press(WizardMessage::Next),
        ]
        .spacing(theme::SPACING),
    );

    col.into()
}

fn view_basic_config(state: &WizardState) -> Element<'_, WizardMessage> {
    let browser_options: Vec<String> = BROWSERS.iter().map(|s| s.to_string()).collect();
    let selected_browser: Option<String> = Some(state.cookies_browser.clone());

    let mut col = column![
        text("Basic Configuration").size(24),
        text("Server Port:").size(14),
        text_input("9851", &state.server_port)
            .on_input(WizardMessage::PortChanged)
            .width(120),
        text("Max Cache Size (MB):").size(14),
        text_input("2048", &state.cache_max_mb)
            .on_input(WizardMessage::CacheMaxChanged)
            .width(120),
        row![
            text("Enable Cookies:").size(14),
            toggler(state.cookies_enabled).on_toggle(WizardMessage::CookiesToggled),
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center),
    ]
    .spacing(theme::SPACING)
    .padding(theme::PADDING)
    .max_width(500);

    if state.cookies_enabled {
        col = col.push(text("Browser for cookies:").size(14));
        col = col.push(
            pick_list(browser_options, selected_browser, WizardMessage::BrowserSelected).width(160),
        );
    }

    col = col.push(
        row![
            button("Back").on_press(WizardMessage::Back),
            button("Next").on_press(WizardMessage::Next),
        ]
        .spacing(theme::SPACING),
    );

    col.into()
}

fn view_done(_state: &WizardState) -> Element<'_, WizardMessage> {
    column![
        text("All Done!").size(28),
        text("Your configuration has been saved. Click \"Finish\" to start using VRC-YtDlp.").size(14),
        row![
            button("Back").on_press(WizardMessage::Back),
            button("Finish").on_press(WizardMessage::FinishWizard),
        ]
        .spacing(theme::SPACING),
    ]
    .spacing(theme::SPACING)
    .padding(theme::PADDING)
    .max_width(500)
    .into()
}
