use iced::widget::{button, column, container, row, text, Space};
use iced::{Element, Length};

use crate::theme;
use crate::widget;

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

    pub fn icon(self) -> &'static str {
        match self {
            Tab::Dashboard => "⊞",
            Tab::Config => "⚙",
            Tab::Server => "◉",
            Tab::Cache => "◫",
            Tab::Logs => "☰",
            Tab::Updates => "↓",
            Tab::Cookies => "🍪",
        }
    }
}

pub fn sidebar_view<'a, M: Clone + 'a>(
    active: Tab,
    server_running: bool,
    server_port: u16,
    on_select: impl Fn(Tab) -> M + 'a,
) -> Element<'a, M> {
    // Branding block at top
    let brand_icon = container(Space::new(0, 0))
        .width(24)
        .height(24)
        .style(|_: &iced::Theme| iced::widget::container::Style {
            background: Some(iced::Background::Color(theme::ACCENT_DIM)),
            border: iced::Border {
                radius: 6.0.into(),
                ..Default::default()
            },
            ..Default::default()
        });

    let brand = container(
        row![
            brand_icon,
            column![
                text("vrc-ytdlp").size(12).color(theme::TEXT_PRIMARY),
                text("MEDIA PROXY").size(9).color(theme::TEXT_MUTED),
            ]
            .spacing(1),
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center),
    )
    .padding(iced::Padding { top: 16.0, right: 12.0, bottom: 14.0, left: 12.0 })
    .width(Length::Fill);

    // Nav items
    let nav_items = Tab::ALL.iter().fold(
        column![].spacing(1).padding(iced::Padding { top: 0.0, right: 8.0, bottom: 0.0, left: 8.0 }),
        |col, &tab| {
            let is_active = tab == active;

            let label_color = if is_active {
                theme::ACCENT
            } else {
                theme::TEXT_SECONDARY
            };

            let item_content = row![
                text(tab.icon()).size(12).color(label_color),
                text(tab.label()).size(12).color(label_color),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center);

            let item_inner = if is_active {
                // Active: teal left rail + teal background fill
                container(
                    row![
                        // Left accent rail
                        container(Space::new(0, 0))
                            .width(2)
                            .height(Length::Fill)
                            .style(|_: &iced::Theme| iced::widget::container::Style {
                                background: Some(iced::Background::Color(theme::ACCENT)),
                                border: iced::Border {
                                    radius: 2.0.into(),
                                    ..Default::default()
                                },
                                ..Default::default()
                            }),
                        container(item_content)
                            .padding(iced::Padding { top: 7.0, right: 10.0, bottom: 7.0, left: 8.0 })
                            .width(Length::Fill),
                    ]
                    .align_y(iced::Alignment::Center),
                )
                .width(Length::Fill)
                .style(|_: &iced::Theme| iced::widget::container::Style {
                    background: Some(iced::Background::Color(
                        iced::Color::from_rgba(0.49, 0.81, 0.71, 0.07),
                    )),
                    border: iced::Border {
                        radius: theme::INPUT_RADIUS.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
            } else {
                container(
                    row![
                        Space::with_width(2),
                        container(item_content)
                            .padding(iced::Padding { top: 7.0, right: 10.0, bottom: 7.0, left: 8.0 })
                            .width(Length::Fill),
                    ]
                    .align_y(iced::Alignment::Center),
                )
                .width(Length::Fill)
            };

            let btn = button(item_inner)
                .on_press(on_select(tab))
                .width(Length::Fill)
                .padding(0)
                .style(|_: &iced::Theme, _status| button::Style {
                    background: None,
                    text_color: iced::Color::WHITE,
                    border: iced::Border::default(),
                    shadow: iced::Shadow::default(),
                });

            col.push(btn)
        },
    );

    // Status card pinned to bottom
    let dot_color = if server_running {
        theme::STATUS_GREEN
    } else {
        theme::STATUS_RED
    };
    let status_label = if server_running { "Server running" } else { "Server stopped" };
    let port_label = format!("Port {server_port}");

    let status_card = container(
        column![
            row![
                widget::status_dot(dot_color),
                text(status_label).size(11).color(theme::TEXT_PRIMARY),
            ]
            .spacing(6)
            .align_y(iced::Alignment::Center),
            text(port_label).size(10).color(theme::TEXT_MUTED),
        ]
        .spacing(3),
    )
    .padding(iced::Padding { top: 10.0, right: 12.0, bottom: 10.0, left: 12.0 })
    .width(Length::Fill)
    .style(|_: &iced::Theme| iced::widget::container::Style {
        background: Some(iced::Background::Color(theme::BG_ELEVATED)),
        border: iced::Border {
            width: 1.0,
            radius: theme::INPUT_RADIUS.into(),
            color: theme::BORDER_SUBTLE,
        },
        ..Default::default()
    });

    let status_wrapper = container(status_card)
        .padding(iced::Padding { top: 0.0, right: 8.0, bottom: 12.0, left: 8.0 })
        .width(Length::Fill);

    // Full sidebar layout
    let sidebar_body = column![
        brand,
        nav_items,
        Space::with_height(Length::Fill),
        status_wrapper,
    ];

    container(sidebar_body)
        .width(theme::SIDEBAR_WIDTH)
        .height(Length::Fill)
        .style(|_: &iced::Theme| iced::widget::container::Style {
            background: Some(iced::Background::Color(theme::BG_SIDEBAR)),
            border: iced::Border {
                width: 0.0,
                radius: 0.0.into(),
                color: theme::BORDER_SUBTLE,
            },
            ..Default::default()
        })
        .into()
}
