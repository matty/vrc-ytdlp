use iced::widget::{column, container, pick_list, row, scrollable, text};
use iced::{Element, Length};

use crate::config::{self, Config, ValidationErrors};
use crate::services::cookie_extractor::BROWSERS;
use crate::theme;
use crate::widget;

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct ConfigTabState {
    pub draft: Config,
    pub saved: Config,
    pub save_error: Option<String>,
    pub save_success: bool,
    pub validation_errors: ValidationErrors,
}

impl ConfigTabState {
    pub fn new(config: Config) -> Self {
        Self {
            draft: config.clone(),
            saved: config,
            save_error: None,
            save_success: false,
            validation_errors: Vec::new(),
        }
    }

    pub fn is_dirty(&self) -> bool {
        self.draft != self.saved
    }

    /// Re-sync the saved config after external changes (e.g., wizard re-run).
    pub fn sync_saved(&mut self, config: Config) {
        self.saved = config.clone();
        self.draft = config;
        self.save_error = None;
        self.save_success = false;
        self.validation_errors = Vec::new();
    }
}

// ---------------------------------------------------------------------------
// Message
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum ConfigMessage {
    // Paths
    YtdlpLocationChanged(String),
    FfmpegLocationChanged(String),
    PluginDirsChanged(String),
    // Server
    ServerPortChanged(String),
    IdleTimeoutChanged(String),
    BgutilPotPortChanged(String),
    // Downloads
    ExecutionTimeoutChanged(String),
    AllowedArgsChanged(String),
    CustomArgsChanged(String),
    ExtractorArgsChanged(String),
    // Cookies
    CookiesToggled(bool),
    CookiesBrowserSelected(String),
    // Cache
    CacheDirChanged(String),
    CacheMaxMbChanged(String),
    CacheTtlChanged(String),
    // Updates
    UpdateCheckDaysChanged(String),
    // Actions
    Save,
    ResetToDefaults,
    RerunWizard,
}

/// Actions that the parent (main.rs) should act on.
#[derive(Debug, Clone)]
pub enum ConfigAction {
    ConfigSaved(Config),
    RerunWizard,
}

// ---------------------------------------------------------------------------
// Update
// ---------------------------------------------------------------------------

pub fn update(state: &mut ConfigTabState, msg: ConfigMessage) -> Option<ConfigAction> {
    state.save_success = false;

    match msg {
        // Paths
        ConfigMessage::YtdlpLocationChanged(v) => state.draft.ytdlp_location = v,
        ConfigMessage::FfmpegLocationChanged(v) => state.draft.ffmpeg_location = v,
        ConfigMessage::PluginDirsChanged(v) => {
            state.draft.plugin_dirs = if v.is_empty() { None } else { Some(v) };
        }
        // Server
        ConfigMessage::ServerPortChanged(v) => {
            if let Ok(p) = v.parse::<u16>() {
                state.draft.server_port = p;
            }
        }
        ConfigMessage::IdleTimeoutChanged(v) => {
            if let Ok(t) = v.parse::<u64>() {
                state.draft.server_idle_timeout_secs = t;
            }
        }
        ConfigMessage::BgutilPotPortChanged(v) => {
            if let Ok(p) = v.parse::<u16>() {
                state.draft.bgutil_pot_port = p;
            }
        }
        // Downloads
        ConfigMessage::ExecutionTimeoutChanged(v) => {
            if let Ok(t) = v.parse::<u64>() {
                state.draft.execution_timeout_secs = t;
            }
        }
        ConfigMessage::AllowedArgsChanged(v) => {
            state.draft.allowed_args = v.split_whitespace().map(String::from).collect();
        }
        ConfigMessage::CustomArgsChanged(v) => {
            state.draft.custom_args = v.split_whitespace().map(String::from).collect();
        }
        ConfigMessage::ExtractorArgsChanged(v) => {
            state.draft.extractor_args = v.split_whitespace().map(String::from).collect();
        }
        // Cookies
        ConfigMessage::CookiesToggled(v) => state.draft.cookies = v,
        ConfigMessage::CookiesBrowserSelected(v) => state.draft.cookies_browser = v,
        // Cache
        ConfigMessage::CacheDirChanged(v) => state.draft.cache_dir = v,
        ConfigMessage::CacheMaxMbChanged(v) => {
            if let Ok(m) = v.parse::<u64>() {
                state.draft.cache_max_size_mb = m;
            }
        }
        ConfigMessage::CacheTtlChanged(v) => {
            if let Ok(t) = v.parse::<u64>() {
                state.draft.cache_ttl_secs = t;
            }
        }
        // Updates
        ConfigMessage::UpdateCheckDaysChanged(v) => {
            if let Ok(d) = v.parse::<u64>() {
                state.draft.update_check_days = d;
            }
        }
        // Actions
        ConfigMessage::Save => {
            let errors = state.draft.validate();
            if !errors.is_empty() {
                state.validation_errors = errors;
                return None;
            }
            state.validation_errors = Vec::new();

            let config_path = crate::paths::config_path()
                .unwrap_or_else(|_| std::path::PathBuf::from("config.json"));
            match config::save_config(&config_path, &state.draft) {
                Ok(()) => {
                    state.saved = state.draft.clone();
                    state.save_success = true;
                    state.save_error = None;
                    return Some(ConfigAction::ConfigSaved(state.saved.clone()));
                }
                Err(e) => {
                    state.save_error = Some(e.to_string());
                }
            }
        }
        ConfigMessage::ResetToDefaults => {
            state.draft = Config::default();
            state.validation_errors = Vec::new();
        }
        ConfigMessage::RerunWizard => {
            return Some(ConfigAction::RerunWizard);
        }
    }
    None
}

// ---------------------------------------------------------------------------
// View
// ---------------------------------------------------------------------------

pub fn view(state: &ConfigTabState) -> Element<'_, ConfigMessage> {
    let browser_options: Vec<String> = BROWSERS.iter().map(|s| s.to_string()).collect();
    let selected_browser: Option<String> = Some(state.draft.cookies_browser.clone());

    // --- Header row with save/reset actions ---
    let save_msg = if state.is_dirty() {
        Some(ConfigMessage::Save)
    } else {
        None
    };
    let header_row = row![
        widget::page_header(
            "Configuration",
            "Manage paths, server, cache, and connection settings",
        ),
        iced::widget::Space::new(Length::Fill, 0),
        widget::secondary_button("Reset to Defaults", Some(ConfigMessage::ResetToDefaults)),
        widget::secondary_button("Re-run Wizard", Some(ConfigMessage::RerunWizard)),
        widget::primary_button("Save", save_msg),
    ]
    .spacing(theme::SPACING_SM)
    .align_y(iced::Alignment::Center);

    // --- Status / validation feedback ---
    let mut status_col = column![].spacing(4);
    for (field, msg) in &state.validation_errors {
        status_col = status_col.push(
            text(format!("{field}: {msg}"))
                .size(12)
                .color(theme::STATUS_RED),
        );
    }
    if let Some(err) = &state.save_error {
        status_col = status_col.push(
            text(format!("Save error: {err}"))
                .size(12)
                .color(theme::STATUS_RED),
        );
    }
    if state.save_success {
        status_col = status_col.push(
            text("Configuration saved.")
                .size(12)
                .color(theme::STATUS_GREEN),
        );
    }

    // --- PATHS section ---
    let paths_section = column![
        widget::section_header("PATHS"),
        widget::labeled_input(
            "yt-dlp location",
            &state.draft.ytdlp_location,
            "./yt-dlp",
            ConfigMessage::YtdlpLocationChanged,
        ),
        widget::labeled_input(
            "ffmpeg location",
            &state.draft.ffmpeg_location,
            "./ffmpeg",
            ConfigMessage::FfmpegLocationChanged,
        ),
        widget::labeled_input(
            "Plugin directories",
            state.draft.plugin_dirs.as_deref().unwrap_or(""),
            "(optional)",
            ConfigMessage::PluginDirsChanged,
        ),
    ]
    .spacing(theme::SPACING_SM);

    // --- SERVER section (3-column grid) ---
    let server_section = column![
        widget::section_header("SERVER"),
        row![
            widget::labeled_input(
                "Server port",
                &state.draft.server_port.to_string(),
                "9851",
                ConfigMessage::ServerPortChanged,
            ),
            widget::labeled_input(
                "Idle timeout (secs)",
                &state.draft.server_idle_timeout_secs.to_string(),
                "300",
                ConfigMessage::IdleTimeoutChanged,
            ),
            widget::labeled_input(
                "bgutil-pot port",
                &state.draft.bgutil_pot_port.to_string(),
                "4416",
                ConfigMessage::BgutilPotPortChanged,
            ),
        ]
        .spacing(theme::SPACING),
    ]
    .spacing(theme::SPACING_SM);

    // --- DOWNLOADS section ---
    let download_section = column![
        widget::section_header("DOWNLOADS"),
        widget::labeled_input(
            "Execution timeout (secs)",
            &state.draft.execution_timeout_secs.to_string(),
            "120",
            ConfigMessage::ExecutionTimeoutChanged,
        ),
        widget::labeled_input(
            "Allowed args (space-separated)",
            &state.draft.allowed_args.join(" "),
            "--get-url",
            ConfigMessage::AllowedArgsChanged,
        ),
        widget::labeled_input(
            "Custom args (space-separated)",
            &state.draft.custom_args.join(" "),
            "--no-check-certificate ...",
            ConfigMessage::CustomArgsChanged,
        ),
        widget::labeled_input(
            "Extractor args (space-separated)",
            &state.draft.extractor_args.join(" "),
            "(optional)",
            ConfigMessage::ExtractorArgsChanged,
        ),
    ]
    .spacing(theme::SPACING_SM);

    // --- COOKIES section ---
    let browser_label = column![
        text("BROWSER FOR COOKIES".to_uppercase())
            .size(10)
            .color(theme::TEXT_LABEL),
        widget::input_container(
            pick_list(
                browser_options,
                selected_browser,
                ConfigMessage::CookiesBrowserSelected,
            )
            .width(Length::Fill),
        ),
    ]
    .spacing(5)
    .width(Length::Fill);

    let cookies_section = column![
        widget::section_header("COOKIES"),
        widget::labeled_toggle(
            "Enable cookies",
            "Authenticate with browser cookies",
            state.draft.cookies,
            ConfigMessage::CookiesToggled,
        ),
        browser_label,
    ]
    .spacing(theme::SPACING_SM);

    // --- CACHE section (3-column grid) ---
    let cache_section = column![
        widget::section_header("CACHE"),
        widget::labeled_input(
            "Cache directory",
            &state.draft.cache_dir,
            "./cache",
            ConfigMessage::CacheDirChanged,
        ),
        row![
            widget::labeled_input(
                "Max cache size (MB)",
                &state.draft.cache_max_size_mb.to_string(),
                "2048",
                ConfigMessage::CacheMaxMbChanged,
            ),
            widget::labeled_input(
                "Cache TTL (secs)",
                &state.draft.cache_ttl_secs.to_string(),
                "86400",
                ConfigMessage::CacheTtlChanged,
            ),
        ]
        .spacing(theme::SPACING),
    ]
    .spacing(theme::SPACING_SM);

    // --- UPDATES section ---
    let update_section = column![
        widget::section_header("UPDATES"),
        widget::labeled_input(
            "Update check interval (days)",
            &state.draft.update_check_days.to_string(),
            "1",
            ConfigMessage::UpdateCheckDaysChanged,
        ),
    ]
    .spacing(theme::SPACING_SM);

    // --- Assemble scrollable content ---
    let content = container(
        column![
            header_row,
            widget::section_divider(),
            paths_section,
            widget::section_divider(),
            server_section,
            widget::section_divider(),
            download_section,
            widget::section_divider(),
            cookies_section,
            widget::section_divider(),
            cache_section,
            widget::section_divider(),
            update_section,
            status_col,
        ]
        .spacing(theme::SPACING_LG)
        .width(Length::Fill),
    )
    .padding(iced::Padding {
        top: 24.0,
        right: 28.0,
        bottom: 24.0,
        left: 28.0,
    });

    scrollable(content).height(Length::Fill).into()
}
