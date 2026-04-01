use iced::widget::{button, column, container, text};
use iced::{Element, Length};

use crate::theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Dashboard,
    Config,
    Server,
    Cache,
    Logs,
    Updates,
    Cookies,
}

impl Tab {
    pub const ALL: &[Tab] = &[
        Tab::Dashboard,
        Tab::Config,
        Tab::Server,
        Tab::Cache,
        Tab::Logs,
        Tab::Updates,
        Tab::Cookies,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Tab::Dashboard => "Dashboard",
            Tab::Config => "Config",
            Tab::Server => "Server",
            Tab::Cache => "Cache",
            Tab::Logs => "Logs",
            Tab::Updates => "Updates",
            Tab::Cookies => "Cookies",
        }
    }
}

pub fn sidebar_view<'a, M: Clone + 'a>(
    active: Tab,
    on_select: impl Fn(Tab) -> M + 'a,
) -> Element<'a, M> {
    let buttons = Tab::ALL.iter().fold(
        column![].spacing(4).padding(8),
        |col, &tab| {
            let btn = button(text(tab.label()).size(14))
                .on_press(on_select(tab))
                .width(Length::Fill);
            let btn = if tab == active {
                btn.style(button::primary)
            } else {
                btn.style(button::secondary)
            };
            col.push(btn)
        },
    );

    container(buttons)
        .width(theme::SIDEBAR_WIDTH)
        .height(Length::Fill)
        .style(|_theme: &iced::Theme| iced::widget::container::Style {
            background: Some(iced::Background::Color(theme::SIDEBAR_BG)),
            ..Default::default()
        })
        .into()
}
