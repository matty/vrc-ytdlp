mod config;
mod paths;
mod services;
mod sidebar;
mod tabs;
mod theme;
mod widget;
mod wizard;

use std::path::PathBuf;
use std::sync::{mpsc, OnceLock};
use std::time::Duration;

use iced::widget::{container, row};
use iced::{Element, Length, Subscription, Task, Theme};

use sidebar::Tab;

// ---------------------------------------------------------------------------
// System tray
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
enum TrayEvent {
    ShowWindow,
    ToggleServer,
    Quit,
}

/// Global channel for tray menu events. Set up once on app start.
static TRAY_RECEIVER: OnceLock<std::sync::Mutex<mpsc::Receiver<TrayEvent>>> = OnceLock::new();

fn setup_tray() -> Option<tray_icon::TrayIcon> {
    use tray_icon::menu::{Menu, MenuItem};
    use tray_icon::{Icon, TrayIconBuilder};

    let menu = Menu::new();
    let show_item = MenuItem::new("Show Window", true, None);
    let server_item = MenuItem::new("Start/Stop Server", true, None);
    let quit_item = MenuItem::new("Quit", true, None);

    let show_id = show_item.id().clone();
    let server_id = server_item.id().clone();
    let quit_id = quit_item.id().clone();

    let _ = menu.append(&show_item);
    let _ = menu.append(&server_item);
    let _ = menu.append(&quit_item);

    // Create a simple 16x16 RGBA icon (solid accent color)
    let icon_rgba = vec![0x5A, 0x8D, 0xF2, 0xFF].repeat(16 * 16);
    let icon = match Icon::from_rgba(icon_rgba, 16, 16) {
        Ok(i) => i,
        Err(_) => return None,
    };

    let tray = TrayIconBuilder::new()
        .with_tooltip("vrc-ytdlp")
        .with_icon(icon)
        .with_menu(Box::new(menu))
        .build()
        .ok()?;

    // Set up event forwarding through a channel
    let (tx, rx) = mpsc::channel();
    TRAY_RECEIVER.get_or_init(|| std::sync::Mutex::new(rx));

    // Spawn a thread to forward tray_icon MenuEvent to our channel
    std::thread::spawn(move || {
        let menu_channel = tray_icon::menu::MenuEvent::receiver();
        loop {
            if let Ok(event) = menu_channel.recv() {
                let tray_event = if event.id == show_id {
                    TrayEvent::ShowWindow
                } else if event.id == server_id {
                    TrayEvent::ToggleServer
                } else if event.id == quit_id {
                    TrayEvent::Quit
                } else {
                    continue;
                };
                if tx.send(tray_event).is_err() {
                    break;
                }
            }
        }
    });

    Some(tray)
}

fn poll_tray_events() -> Option<TrayEvent> {
    TRAY_RECEIVER
        .get()
        .and_then(|mtx| mtx.lock().ok())
        .and_then(|rx| rx.try_recv().ok())
}

// ---------------------------------------------------------------------------
// Phase
// ---------------------------------------------------------------------------

enum Phase {
    Wizard(wizard::WizardState),
    Main,
}

// ---------------------------------------------------------------------------
// App
// ---------------------------------------------------------------------------

struct App {
    phase: Phase,
    active_tab: Tab,
    config: config::Config,

    // Tab states
    dashboard: tabs::dashboard::DashboardState,
    config_tab: tabs::config_tab::ConfigTabState,
    server_tab: tabs::server::ServerTabState,
    cache_tab: tabs::cache::CacheTabState,
    logs_tab: tabs::logs::LogsTabState,
    updates_tab: tabs::updates::UpdatesTabState,
    cookies_tab: tabs::cookies::CookiesTabState,

    // System tray (keep alive)
    _tray: Option<tray_icon::TrayIcon>,
}

impl Default for App {
    fn default() -> Self {
        let config_path =
            paths::config_path().unwrap_or_else(|_| std::path::PathBuf::from("config.json"));

        let (phase, cfg) = if config::config_exists(&config_path) {
            let cfg = config::load_config(&config_path).unwrap_or_default();
            (Phase::Main, cfg)
        } else {
            (
                Phase::Wizard(wizard::WizardState::new()),
                config::Config::default(),
            )
        };

        let ytdlp_path = paths::resolve_path(&cfg.ytdlp_location)
            .unwrap_or_else(|_| PathBuf::from(&cfg.ytdlp_location));
        let ffmpeg_path = paths::resolve_path(&cfg.ffmpeg_location)
            .unwrap_or_else(|_| PathBuf::from(&cfg.ffmpeg_location));
        let cache_dir = paths::resolve_path(&cfg.cache_dir)
            .unwrap_or_else(|_| PathBuf::from(&cfg.cache_dir));

        let tray = setup_tray();

        Self {
            phase,
            active_tab: Tab::Dashboard,
            dashboard: tabs::dashboard::DashboardState::new(
                cfg.server_port,
                cfg.cache_max_size_mb,
            ),
            config_tab: tabs::config_tab::ConfigTabState::new(cfg.clone()),
            server_tab: tabs::server::ServerTabState::new(
                cfg.server_port,
                cfg.server_idle_timeout_secs,
            ),
            cache_tab: tabs::cache::CacheTabState::new(cache_dir, cfg.cache_max_size_mb),
            logs_tab: tabs::logs::LogsTabState::new(),
            updates_tab: tabs::updates::UpdatesTabState::new(
                ytdlp_path,
                ffmpeg_path,
            ),
            cookies_tab: tabs::cookies::CookiesTabState::new(
                cfg.cookies_browser.clone(),
                paths::resolve_path(&cfg.ytdlp_location)
                    .unwrap_or_else(|_| PathBuf::from(&cfg.ytdlp_location)),
            ),
            config: cfg,
            _tray: tray,
        }
    }
}

// ---------------------------------------------------------------------------
// Messages
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
enum Message {
    TabSelected(Tab),
    Wizard(wizard::WizardMessage),

    // Tab messages
    Dashboard(tabs::dashboard::DashboardMessage),
    ConfigTab(tabs::config_tab::ConfigMessage),
    ServerTab(tabs::server::ServerMessage),
    CacheTab(tabs::cache::CacheMessage),
    LogsTab(tabs::logs::LogsMessage),
    UpdatesTab(tabs::updates::UpdatesMessage),
    CookiesTab(tabs::cookies::CookiesMessage),

    // Subscriptions
    HealthTick,
    HealthResult(bool),
    LogPollTick,
    TrayPollTick,
    Tray(TrayEvent),

    Noop,
}

// ---------------------------------------------------------------------------
// Update / View / Subscription
// ---------------------------------------------------------------------------

impl App {
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::TabSelected(tab) => {
                let prev = self.active_tab;
                self.active_tab = tab;

                // Initialise tailer when switching to Logs tab
                if tab == Tab::Logs && prev != Tab::Logs {
                    self.logs_tab.init_tailer();
                }

                // Trigger a cache scan when switching to Cache tab
                if tab == Tab::Cache && self.cache_tab.summary.is_none() {
                    return tabs::cache::update(&mut self.cache_tab, tabs::cache::CacheMessage::Scan)
                        .map(Message::CacheTab);
                }

                Task::none()
            }

            // ----- Wizard -----
            Message::Wizard(wizard_msg) => {
                if let Phase::Wizard(ref mut state) = self.phase {
                    let (maybe_config, task) = wizard::update(state, wizard_msg);
                    if let Some(cfg) = maybe_config {
                        let config_path = paths::config_path()
                            .unwrap_or_else(|_| std::path::PathBuf::from("config.json"));
                        let _ = config::save_config(&config_path, &cfg);
                        self.apply_config(cfg);
                        self.phase = Phase::Main;
                        return Task::none();
                    }
                    task.map(Message::Wizard)
                } else {
                    Task::none()
                }
            }

            // ----- Dashboard -----
            Message::Dashboard(tabs::dashboard::DashboardMessage::GoToTab(tab)) => {
                self.active_tab = tab;
                Task::none()
            }

            // ----- Config Tab -----
            Message::ConfigTab(msg) => {
                if let Some(action) = tabs::config_tab::update(&mut self.config_tab, msg) {
                    match action {
                        tabs::config_tab::ConfigAction::ConfigSaved(cfg) => {
                            self.apply_config(cfg);
                        }
                        tabs::config_tab::ConfigAction::RerunWizard => {
                            self.phase = Phase::Wizard(wizard::WizardState::new());
                        }
                    }
                }
                Task::none()
            }

            // ----- Server Tab -----
            Message::ServerTab(msg) => {
                let task = tabs::server::update(&mut self.server_tab, msg);
                // Sync running status to dashboard
                self.dashboard.server_running = self.server_tab.running;
                task.map(Message::ServerTab)
            }

            // ----- Cache Tab -----
            Message::CacheTab(msg) => {
                let task = tabs::cache::update(&mut self.cache_tab, msg);
                // Sync cache size to dashboard
                if let Some(ref summary) = self.cache_tab.summary {
                    self.dashboard.cache_size_bytes = summary.total_size_bytes;
                }
                task.map(Message::CacheTab)
            }

            // ----- Logs Tab -----
            Message::LogsTab(msg) => {
                tabs::logs::update(&mut self.logs_tab, msg);
                Task::none()
            }

            // ----- Updates Tab -----
            Message::UpdatesTab(msg) => {
                let task = tabs::updates::update(&mut self.updates_tab, msg);
                // Sync version to dashboard
                self.dashboard.ytdlp_version = self.updates_tab.current_version.clone();
                task.map(Message::UpdatesTab)
            }

            // ----- Cookies Tab -----
            Message::CookiesTab(msg) => {
                let task = tabs::cookies::update(&mut self.cookies_tab, msg);
                // Sync cookie status to dashboard
                self.dashboard.cookies_exist = self.cookies_tab.status.exists;
                task.map(Message::CookiesTab)
            }

            // ----- Health check subscription -----
            Message::HealthTick => {
                let port = self.config.server_port;
                Task::perform(
                    async move { services::server_manager::check_health(port).await },
                    Message::HealthResult,
                )
            }
            Message::HealthResult(healthy) => {
                self.server_tab.running = healthy;
                if healthy {
                    self.server_tab.pid = services::server_manager::read_pid();
                }
                self.dashboard.server_running = healthy;
                Task::none()
            }

            // ----- Log poll subscription -----
            Message::LogPollTick => {
                self.logs_tab.poll();
                Task::none()
            }

            // ----- Tray poll subscription -----
            Message::TrayPollTick => {
                if let Some(event) = poll_tray_events() {
                    return self.update(Message::Tray(event));
                }
                Task::none()
            }

            Message::Tray(event) => match event {
                TrayEvent::ShowWindow => {
                    // iced doesn't expose window focus APIs easily; this is a no-op placeholder
                    Task::none()
                }
                TrayEvent::ToggleServer => {
                    if self.server_tab.running {
                        tabs::server::update(
                            &mut self.server_tab,
                            tabs::server::ServerMessage::Stop,
                        )
                        .map(Message::ServerTab)
                    } else {
                        tabs::server::update(
                            &mut self.server_tab,
                            tabs::server::ServerMessage::Start,
                        )
                        .map(Message::ServerTab)
                    }
                }
                TrayEvent::Quit => {
                    std::process::exit(0);
                }
            },

            Message::Noop => Task::none(),
        }
    }

    fn view(&self) -> Element<Message> {
        match &self.phase {
            Phase::Wizard(state) => wizard::view(state).map(Message::Wizard),
            Phase::Main => {
                let sidebar = sidebar::sidebar_view(self.active_tab, Message::TabSelected);

                let content: Element<Message> = match self.active_tab {
                    Tab::Dashboard => {
                        tabs::dashboard::view(&self.dashboard).map(Message::Dashboard)
                    }
                    Tab::Config => {
                        tabs::config_tab::view(&self.config_tab).map(Message::ConfigTab)
                    }
                    Tab::Server => {
                        tabs::server::view(&self.server_tab).map(Message::ServerTab)
                    }
                    Tab::Cache => {
                        tabs::cache::view(&self.cache_tab).map(Message::CacheTab)
                    }
                    Tab::Logs => {
                        tabs::logs::view(&self.logs_tab).map(Message::LogsTab)
                    }
                    Tab::Updates => {
                        tabs::updates::view(&self.updates_tab).map(Message::UpdatesTab)
                    }
                    Tab::Cookies => {
                        tabs::cookies::view(&self.cookies_tab).map(Message::CookiesTab)
                    }
                };

                let body = container(content)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .padding(theme::PADDING);

                row![sidebar, body].into()
            }
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        let mut subs = vec![];

        // Health check every 5 seconds (always running)
        if matches!(self.phase, Phase::Main) {
            subs.push(
                iced::time::every(Duration::from_secs(5)).map(|_| Message::HealthTick),
            );
        }

        // Log poll every 1 second when Logs tab is active
        if matches!(self.phase, Phase::Main) && self.active_tab == Tab::Logs {
            subs.push(
                iced::time::every(Duration::from_secs(1)).map(|_| Message::LogPollTick),
            );
        }

        // Tray event poll every 250ms
        if self._tray.is_some() {
            subs.push(
                iced::time::every(Duration::from_millis(250)).map(|_| Message::TrayPollTick),
            );
        }

        Subscription::batch(subs)
    }

    /// Apply a new config to all tab states.
    fn apply_config(&mut self, cfg: config::Config) {
        let ytdlp_path = paths::resolve_path(&cfg.ytdlp_location)
            .unwrap_or_else(|_| PathBuf::from(&cfg.ytdlp_location));
        let ffmpeg_path = paths::resolve_path(&cfg.ffmpeg_location)
            .unwrap_or_else(|_| PathBuf::from(&cfg.ffmpeg_location));
        let cache_dir = paths::resolve_path(&cfg.cache_dir)
            .unwrap_or_else(|_| PathBuf::from(&cfg.cache_dir));

        // Dashboard
        self.dashboard.server_port = cfg.server_port;
        self.dashboard.cache_max_mb = cfg.cache_max_size_mb;

        // Config tab
        self.config_tab.sync_saved(cfg.clone());

        // Server tab
        self.server_tab.port = cfg.server_port;
        self.server_tab.idle_timeout = cfg.server_idle_timeout_secs;

        // Cache tab
        self.cache_tab.cache_dir = cache_dir;
        self.cache_tab.max_size_mb = cfg.cache_max_size_mb;

        // Updates tab
        self.updates_tab.ytdlp_path = ytdlp_path.clone();
        self.updates_tab.ffmpeg_path = ffmpeg_path;
        self.updates_tab.current_version =
            services::downloader::current_version(&ytdlp_path);

        // Cookies tab
        self.cookies_tab.browser = cfg.cookies_browser.clone();
        self.cookies_tab.ytdlp_path = ytdlp_path;

        self.config = cfg;
    }
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

fn main() -> iced::Result {
    iced::application("vrc-ytdlp", App::update, App::view)
        .subscription(App::subscription)
        .theme(|_| Theme::Dark)
        .window_size((900.0, 600.0))
        .run()
}
