use iced::{
    Color, Element, Length,
    widget::{button, column, container, row, text, text_input, toggler},
};

use crate::theme;

// ---------------------------------------------------------------------------
// Section header — uppercase muted label
// ---------------------------------------------------------------------------

pub fn section_header<'a, M: 'a>(title: &str) -> Element<'a, M> {
    text(title.to_uppercase())
        .size(10)
        .color(theme::TEXT_SECTION)
        .into()
}

// ---------------------------------------------------------------------------
// Section divider
// ---------------------------------------------------------------------------

pub fn section_divider<'a, M: 'a>() -> Element<'a, M> {
    container(iced::widget::Space::new(Length::Fill, 0))
        .height(1)
        .width(Length::Fill)
        .style(|_: &iced::Theme| container::Style {
            background: Some(iced::Background::Color(theme::BORDER_SUBTLE)),
            ..Default::default()
        })
        .into()
}

// ---------------------------------------------------------------------------
// Labeled input — label above text input
// ---------------------------------------------------------------------------

pub fn labeled_input<'a, M: Clone + 'a>(
    label: &str,
    value: &str,
    placeholder: &str,
    on_change: impl Fn(String) -> M + 'a,
) -> Element<'a, M> {
    column![
        text(label.to_uppercase())
            .size(10)
            .color(theme::TEXT_LABEL),
        text_input(placeholder, value)
            .on_input(on_change)
            .size(12)
            .padding(8),
    ]
    .spacing(5)
    .into()
}

// ---------------------------------------------------------------------------
// Labeled toggle — with description
// ---------------------------------------------------------------------------

pub fn labeled_toggle<'a, M: 'a>(
    label: &str,
    description: &str,
    value: bool,
    on_toggle: impl Fn(bool) -> M + 'a,
) -> Element<'a, M> {
    let label_col = column![
        text(label.to_owned()).size(12).color(theme::TEXT_PRIMARY),
        text(description.to_owned()).size(10).color(theme::TEXT_MUTED),
    ]
    .spacing(2)
    .width(Length::Fill);

    row![label_col, toggler(value).on_toggle(on_toggle)]
        .spacing(theme::SPACING)
        .align_y(iced::Alignment::Center)
        .into()
}

// ---------------------------------------------------------------------------
// Status dot
// ---------------------------------------------------------------------------

pub fn status_dot<'a, M: 'a>(color: Color) -> Element<'a, M> {
    container(iced::widget::Space::new(0, 0))
        .width(7)
        .height(7)
        .style(move |_: &iced::Theme| container::Style {
            background: Some(iced::Background::Color(color)),
            border: iced::Border {
                radius: 4.0.into(),
                ..Default::default()
            },
            ..Default::default()
        })
        .into()
}

// ---------------------------------------------------------------------------
// Card — elevated container
// ---------------------------------------------------------------------------

pub fn card<'a, M: 'a>(content: impl Into<Element<'a, M>>) -> Element<'a, M> {
    container(content)
        .padding(theme::PADDING)
        .style(|theme: &iced::Theme| {
            let palette = theme.extended_palette();
            container::Style {
                background: Some(iced::Background::Color(palette.background.weak.color)),
                border: iced::Border {
                    width: 1.0,
                    radius: theme::CARD_RADIUS.into(),
                    color: theme::BORDER_SUBTLE,
                },
                ..Default::default()
            }
        })
        .into()
}

// ---------------------------------------------------------------------------
// Page header — title + subtitle
// ---------------------------------------------------------------------------

pub fn page_header<'a, M: 'a>(title: &str, subtitle: &str) -> Element<'a, M> {
    column![
        text(title.to_owned()).size(17).color(theme::TEXT_PRIMARY),
        text(subtitle.to_owned()).size(11).color(theme::TEXT_MUTED),
    ]
    .spacing(2)
    .into()
}

// ---------------------------------------------------------------------------
// Primary button — uses theme's primary color
// ---------------------------------------------------------------------------

pub fn primary_button<'a, M: Clone + 'a>(
    label: &str,
    msg: Option<M>,
) -> Element<'a, M> {
    let btn = button(text(label.to_owned()).size(11))
        .padding(iced::Padding { top: 6.0, right: 14.0, bottom: 6.0, left: 14.0 })
        .style(button::primary);

    match msg {
        Some(m) => btn.on_press(m).into(),
        None => btn.into(),
    }
}

// ---------------------------------------------------------------------------
// Secondary button — subtle style
// ---------------------------------------------------------------------------

pub fn secondary_button<'a, M: Clone + 'a>(
    label: &str,
    msg: Option<M>,
) -> Element<'a, M> {
    let btn = button(text(label.to_owned()).size(11))
        .padding(iced::Padding { top: 6.0, right: 14.0, bottom: 6.0, left: 14.0 })
        .style(button::secondary);

    match msg {
        Some(m) => btn.on_press(m).into(),
        None => btn.into(),
    }
}

// ---------------------------------------------------------------------------
// Danger button — uses theme's danger color
// ---------------------------------------------------------------------------

pub fn danger_button<'a, M: Clone + 'a>(
    label: &str,
    msg: Option<M>,
) -> Element<'a, M> {
    let btn = button(text(label.to_owned()).size(11))
        .padding(iced::Padding { top: 6.0, right: 14.0, bottom: 6.0, left: 14.0 })
        .style(button::danger);

    match msg {
        Some(m) => btn.on_press(m).into(),
        None => btn.into(),
    }
}

// ---------------------------------------------------------------------------
// Pill badge — small colored label
// ---------------------------------------------------------------------------

pub fn pill_badge<'a, M: 'a>(label: &str, color: Color) -> Element<'a, M> {
    container(text(label.to_owned()).size(9).color(color))
        .padding(iced::Padding { top: 2.0, right: 8.0, bottom: 2.0, left: 8.0 })
        .style(move |_: &iced::Theme| container::Style {
            background: Some(iced::Background::Color(Color::from_rgba(
                color.r, color.g, color.b, 0.1,
            ))),
            border: iced::Border {
                width: 1.0,
                radius: 10.0.into(),
                color: Color::from_rgba(color.r, color.g, color.b, 0.15),
            },
            ..Default::default()
        })
        .into()
}

// ---------------------------------------------------------------------------
// Input container — styled wrapper for pick_lists etc.
// ---------------------------------------------------------------------------

pub fn input_container<'a, M: 'a>(content: impl Into<Element<'a, M>>) -> Element<'a, M> {
    container(content)
        .padding(iced::Padding { top: 8.0, right: 12.0, bottom: 8.0, left: 12.0 })
        .style(|theme: &iced::Theme| {
            let palette = theme.extended_palette();
            container::Style {
                background: Some(iced::Background::Color(palette.background.strong.color)),
                border: iced::Border {
                    width: 1.0,
                    radius: theme::INPUT_RADIUS.into(),
                    color: theme::BORDER_INPUT,
                },
                ..Default::default()
            }
        })
        .into()
}
