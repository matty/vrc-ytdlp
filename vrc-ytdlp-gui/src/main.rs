mod config;
mod paths;
mod sidebar;
mod tabs;
mod theme;
mod widget;

use iced::widget::{container, row};
use iced::{Element, Length, Theme};

use sidebar::Tab;

fn main() -> iced::Result {
    iced::application("vrc-ytdlp", App::update, App::view)
        .theme(|_| Theme::Dark)
        .window_size((900.0, 600.0))
        .run()
}

#[derive(Debug)]
struct App {
    active_tab: Tab,
}

impl Default for App {
    fn default() -> Self {
        Self { active_tab: Tab::Dashboard }
    }
}

#[derive(Debug, Clone)]
enum Message {
    TabSelected(Tab),
}

impl App {
    fn update(&mut self, message: Message) {
        match message {
            Message::TabSelected(tab) => self.active_tab = tab,
        }
    }

    fn view(&self) -> Element<Message> {
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
