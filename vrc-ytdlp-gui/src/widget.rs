use iced::{
    Color, Element, Length,
    widget::{column, container, row, text, text_input, toggler, button},
};

use crate::theme;

// ---------------------------------------------------------------------------
// Section header — uppercase muted label with bottom border feel
// ---------------------------------------------------------------------------

pub fn section_header<'a, M: 'a>(title: &str) -> Element<'a, M> {
    container(
        text(title.to_uppercase())
            .size(10)
            .color(theme::TEXT_SECTION)
    )
    .padding(iced::Padding { top: 0.0, right: 0.0, bottom: 8.0, left: 0.0 })
    .width(Length::Fill)
    .style(|_: &iced::Theme| container::Style {
        border: iced::Border {
            width: 0.0,
            radius: 0.0.into(),
            color: Color::TRANSPARENT,
        },
        ..Default::default()
    })
    .into()
}

// ---------------------------------------------------------------------------
// Section divider — thin line separator
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
// Labeled input — uppercase label above dark input field
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
// Labeled toggle — description text + toggle in a card-like row
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

    container(
        row![label_col, toggler(value).on_toggle(on_toggle)]
            .spacing(theme::SPACING)
            .align_y(iced::Alignment::Center),
    )
    .padding(iced::Padding { top: 10.0, right: 14.0, bottom: 10.0, left: 14.0 })
    .style(|_: &iced::Theme| container::Style {
        background: Some(iced::Background::Color(theme::BG_INPUT)),
        border: iced::Border {
            width: 1.0,
            radius: theme::INPUT_RADIUS.into(),
            color: theme::BORDER_INPUT,
        },
        ..Default::default()
    })
    .into()
}

// ---------------------------------------------------------------------------
// Status dot — small glowing circle
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
// Card — gradient dark card with subtle border
// ---------------------------------------------------------------------------

pub fn card<'a, M: 'a>(content: impl Into<Element<'a, M>>) -> Element<'a, M> {
    container(content)
        .padding(theme::PADDING)
        .style(|_: &iced::Theme| container::Style {
            background: Some(iced::Background::Color(theme::BG_CARD)),
            border: iced::Border {
                width: 1.0,
                radius: theme::CARD_RADIUS.into(),
                color: theme::BORDER_SUBTLE,
            },
            ..Default::default()
        })
        .into()
}

// ---------------------------------------------------------------------------
// Page header — title + subtitle at top of content area
// ---------------------------------------------------------------------------

pub fn page_header<'a, M: 'a>(title: &str, subtitle: &str) -> Element<'a, M> {
    column![
        text(title.to_owned())
            .size(17)
            .color(theme::TEXT_PRIMARY),
        text(subtitle.to_owned())
            .size(11)
            .color(theme::TEXT_MUTED),
    ]
    .spacing(2)
    .into()
}

// ---------------------------------------------------------------------------
// Primary button — teal accent
// ---------------------------------------------------------------------------

pub fn primary_button<'a, M: Clone + 'a>(
    label: &str,
    msg: Option<M>,
) -> Element<'a, M> {
    let btn = button(
        text(label.to_owned()).size(11).color(Color::WHITE)
    )
    .padding(iced::Padding { top: 6.0, right: 14.0, bottom: 6.0, left: 14.0 })
    .style(|_: &iced::Theme, status| {
        let bg = match status {
            button::Status::Hovered => theme::ACCENT,
            _ => theme::ACCENT_DIM,
        };
        button::Style {
            background: Some(iced::Background::Color(bg)),
            text_color: Color::WHITE,
            border: iced::Border {
                radius: 6.0.into(),
                ..Default::default()
            },
            ..Default::default()
        }
    });

    match msg {
        Some(m) => btn.on_press(m).into(),
        None => btn.into(),
    }
}

// ---------------------------------------------------------------------------
// Secondary button — outlined
// ---------------------------------------------------------------------------

pub fn secondary_button<'a, M: Clone + 'a>(
    label: &str,
    msg: Option<M>,
) -> Element<'a, M> {
    let btn = button(
        text(label.to_owned()).size(11).color(theme::TEXT_SECONDARY)
    )
    .padding(iced::Padding { top: 6.0, right: 14.0, bottom: 6.0, left: 14.0 })
    .style(|_: &iced::Theme, _status| {
        button::Style {
            background: Some(iced::Background::Color(Color::TRANSPARENT)),
            text_color: theme::TEXT_SECONDARY,
            border: iced::Border {
                width: 1.0,
                radius: 6.0.into(),
                color: theme::BORDER_INPUT,
            },
            ..Default::default()
        }
    });

    match msg {
        Some(m) => btn.on_press(m).into(),
        None => btn.into(),
    }
}

// ---------------------------------------------------------------------------
// Danger button — red tinted
// ---------------------------------------------------------------------------

pub fn danger_button<'a, M: Clone + 'a>(
    label: &str,
    msg: Option<M>,
) -> Element<'a, M> {
    let btn = button(
        text(label.to_owned()).size(11).color(theme::STATUS_RED)
    )
    .padding(iced::Padding { top: 6.0, right: 14.0, bottom: 6.0, left: 14.0 })
    .style(|_: &iced::Theme, _status| {
        button::Style {
            background: Some(iced::Background::Color(Color::TRANSPARENT)),
            text_color: theme::STATUS_RED,
            border: iced::Border {
                width: 1.0,
                radius: 6.0.into(),
                color: Color::from_rgba(0.94, 0.27, 0.27, 0.2),
            },
            ..Default::default()
        }
    });

    match msg {
        Some(m) => btn.on_press(m).into(),
        None => btn.into(),
    }
}

// ---------------------------------------------------------------------------
// Pill badge — small status label
// ---------------------------------------------------------------------------

pub fn pill_badge<'a, M: 'a>(label: &str, color: Color) -> Element<'a, M> {
    container(
        text(label.to_owned()).size(9).color(color)
    )
    .padding(iced::Padding { top: 2.0, right: 8.0, bottom: 2.0, left: 8.0 })
    .style(move |_: &iced::Theme| container::Style {
        background: Some(iced::Background::Color(Color::from_rgba(
            color.r, color.g, color.b, 0.08,
        ))),
        border: iced::Border {
            width: 1.0,
            radius: 10.0.into(),
            color: Color::from_rgba(color.r, color.g, color.b, 0.12),
        },
        ..Default::default()
    })
    .into()
}

// ---------------------------------------------------------------------------
// Input container — styled wrapper for inputs/dropdowns
// ---------------------------------------------------------------------------

pub fn input_container<'a, M: 'a>(content: impl Into<Element<'a, M>>) -> Element<'a, M> {
    container(content)
        .padding(iced::Padding { top: 8.0, right: 12.0, bottom: 8.0, left: 12.0 })
        .style(|_: &iced::Theme| container::Style {
            background: Some(iced::Background::Color(theme::BG_INPUT)),
            border: iced::Border {
                width: 1.0,
                radius: theme::INPUT_RADIUS.into(),
                color: theme::BORDER_INPUT,
            },
            ..Default::default()
        })
        .into()
}
