mod config;
mod paths;
mod services;
mod sidebar;
mod tabs;
mod theme;
mod widget;
mod wizard;

use iced::widget::{container, row};
use iced::{Element, Length, Task, Theme};

use sidebar::Tab;

fn main() -> iced::Result {
    iced::application("vrc-ytdlp", App::update, App::view)
        .theme(|_| Theme::Dark)
        .window_size((900.0, 600.0))
        .run()
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
}

impl Default for App {
    fn default() -> Self {
        // Determine config path; if it doesn't exist yet, start in wizard mode.
        let config_path = paths::config_path().unwrap_or_else(|_| std::path::PathBuf::from("config.json"));

        let (phase, cfg) = if config::config_exists(&config_path) {
            let cfg = config::load_config(&config_path).unwrap_or_default();
            (Phase::Main, cfg)
        } else {
            (Phase::Wizard(wizard::WizardState::new()), config::Config::default())
        };

        Self {
            phase,
            active_tab: Tab::Dashboard,
            config: cfg,
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
}

// ---------------------------------------------------------------------------
// Update / View
// ---------------------------------------------------------------------------

impl App {
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::TabSelected(tab) => {
                self.active_tab = tab;
                Task::none()
            }

            Message::Wizard(wizard_msg) => {
                if let Phase::Wizard(ref mut state) = self.phase {
                    let (maybe_config, task) = wizard::update(state, wizard_msg);
                    if let Some(cfg) = maybe_config {
                        // Save the config and transition to main phase.
                        let config_path = paths::config_path()
                            .unwrap_or_else(|_| std::path::PathBuf::from("config.json"));
                        let _ = config::save_config(&config_path, &cfg);
                        self.config = cfg;
                        self.phase = Phase::Main;
                        return Task::none();
                    }
                    task.map(Message::Wizard)
                } else {
                    Task::none()
                }
            }
        }
    }

    fn view(&self) -> Element<Message> {
        match &self.phase {
            Phase::Wizard(state) => {
                wizard::view(state).map(Message::Wizard)
            }
            Phase::Main => {
                let sidebar = sidebar::sidebar_view(self.active_tab, Message::TabSelected);

                let content = match self.active_tab {
                    Tab::Dashboard => tabs::dashboard::view(),
                    Tab::Config => tabs::config_tab::view(),
                    Tab::Server => tabs::server::view(),
                    Tab::Cache => tabs::cache::view(),
                    Tab::Logs => tabs::logs::view(),
                    Tab::Updates => tabs::updates::view(),
                    Tab::Cookies => tabs::cookies::view(),
                };

                let body = container(content)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .padding(theme::PADDING);

                row![sidebar, body].into()
            }
        }
    }
}
