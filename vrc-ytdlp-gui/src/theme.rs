use iced::color;
use iced::theme::Palette;
use iced::{Color, Theme};

/// Build our custom dark theme with teal accent.
pub fn app_theme() -> Theme {
    Theme::custom(
        "vrc-ytdlp".to_string(),
        Palette {
            background: color!(0x08080b),
            text: color!(0xe8e8e8),
            primary: color!(0x7dcfb6),
            success: color!(0x4ade80),
            danger: color!(0xef4444),
        },
    )
}

// -- Accent --
pub const ACCENT: Color = Color::from_rgb(0.49, 0.81, 0.71); // #7dcfb6
pub const ACCENT_DIM: Color = Color::from_rgb(0.36, 0.62, 0.54); // #5ba898

// -- Backgrounds --
pub const BG_BASE: Color = Color::from_rgb(0.031, 0.031, 0.043); // #08080b
pub const BG_SIDEBAR: Color = Color::from_rgb(0.047, 0.047, 0.063); // #0c0c10
pub const BG_CARD: Color = Color::from_rgb(0.063, 0.063, 0.086); // #101016
pub const BG_INPUT: Color = Color::from_rgb(0.055, 0.055, 0.078); // #0e0e14
pub const BG_ELEVATED: Color = Color::from_rgb(0.067, 0.067, 0.082); // #111115

// -- Borders --
pub const BORDER_SUBTLE: Color = Color::from_rgba(1.0, 1.0, 1.0, 0.04);
pub const BORDER_INPUT: Color = Color::from_rgba(1.0, 1.0, 1.0, 0.06);

// -- Text --
pub const TEXT_PRIMARY: Color = Color::from_rgb(0.91, 0.91, 0.91); // #e8e8e8
pub const TEXT_SECONDARY: Color = Color::from_rgb(0.33, 0.33, 0.33); // #555
pub const TEXT_MUTED: Color = Color::from_rgb(0.23, 0.23, 0.26); // #3a3a42
pub const TEXT_LABEL: Color = Color::from_rgb(0.53, 0.53, 0.53); // #888
pub const TEXT_SECTION: Color = Color::from_rgb(0.40, 0.40, 0.40); // #666

// -- Status --
pub const STATUS_GREEN: Color = Color::from_rgb(0.29, 0.87, 0.50); // #4ade80
pub const STATUS_RED: Color = Color::from_rgb(0.94, 0.27, 0.27); // #ef4444
pub const STATUS_YELLOW: Color = Color::from_rgb(0.98, 0.80, 0.08); // #facc15

// -- Layout --
pub const SPACING_SM: u16 = 8;
pub const SPACING: u16 = 10;
pub const SPACING_LG: u16 = 16;
pub const PADDING: u16 = 16;
pub const PADDING_LG: u16 = 28;
pub const SIDEBAR_WIDTH: u16 = 190;
pub const CARD_RADIUS: f32 = 10.0;
pub const INPUT_RADIUS: f32 = 6.0;

// -- Aliases --
pub const GREEN: Color = STATUS_GREEN;
pub const RED: Color = STATUS_RED;
pub const YELLOW: Color = STATUS_YELLOW;
pub const GREY: Color = TEXT_SECONDARY;
pub const SIDEBAR_BG: Color = BG_SIDEBAR;
