use iced::{
    Color, Element, Length,
    widget::{column, container, row, text, text_input, toggler},
};

use crate::theme;

pub fn section_header<'a, M: 'a>(title: &str) -> Element<'a, M> {
    text(title.to_owned()).size(18).into()
}

pub fn labeled_input<'a, M: Clone + 'a>(
    label: &str,
    value: &str,
    placeholder: &str,
    on_change: impl Fn(String) -> M + 'a,
) -> Element<'a, M> {
    column![
        text(label.to_owned()).size(14),
        text_input(placeholder, value).on_input(on_change),
    ]
    .spacing(4)
    .into()
}

pub fn labeled_toggle<'a, M: 'a>(
    label: &str,
    value: bool,
    on_toggle: impl Fn(bool) -> M + 'a,
) -> Element<'a, M> {
    row![
        text(label.to_owned()).size(14).width(Length::Fill),
        toggler(value).on_toggle(on_toggle),
    ]
    .spacing(8)
    .align_y(iced::Alignment::Center)
    .into()
}

pub fn status_dot<'a, M: 'a>(color: Color) -> Element<'a, M> {
    container(iced::widget::Space::new(0, 0))
        .width(12)
        .height(12)
        .style(move |_theme: &iced::Theme| container::Style {
            background: Some(iced::Background::Color(color)),
            border: iced::Border {
                radius: 6.0.into(),
                ..Default::default()
            },
            ..Default::default()
        })
        .into()
}

pub fn card<'a, M: 'a>(content: impl Into<Element<'a, M>>) -> Element<'a, M> {
    container(content)
        .padding(theme::PADDING)
        .style(|_theme: &iced::Theme| container::Style {
            background: Some(iced::Background::Color(theme::CARD_BG)),
            border: iced::Border {
                radius: 8.0.into(),
                ..Default::default()
            },
            ..Default::default()
        })
        .into()
}
