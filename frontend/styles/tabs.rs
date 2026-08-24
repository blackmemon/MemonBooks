//! Toolbar tab styling (Section 6): active tab is bold text plus a
//! bottom-border underline in `white_primary`; inactive tabs are plain
//! gray text with no border.

use iced::widget::button;
use iced::{Background, Border, Color, Theme};

use crate::frontend::styles::colors;

pub fn text_color(active: bool) -> Color {
    if active {
        colors::WHITE_PRIMARY
    } else {
        colors::WHITE_NEUTRAL_FRONT
    }
}

pub fn underline(active: bool) -> impl Fn(&Theme) -> iced::widget::container::Style {
    move |_theme: &Theme| iced::widget::container::Style {
        background: Some(Background::Color(if active { colors::WHITE_PRIMARY } else { Color::TRANSPARENT })),
        ..iced::widget::container::Style::default()
    }
}

/// The tab is a plain (borderless, backgroundless) button so its label
/// and underline can be styled directly -- this only strips the default
/// button chrome.
pub fn tab_button() -> impl Fn(&Theme, button::Status) -> button::Style {
    |_theme: &Theme, _status: button::Status| button::Style {
        background: None,
        text_color: Color::BLACK,
        border: Border::default(),
        shadow: iced::Shadow::default(),
    }
}
