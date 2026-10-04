//! Toolbar tab styling (Section 6): active tab is bold text plus a
//! bottom-border underline in `white_primary`; inactive tabs are plain
//! gray text with no border.

use iced::widget::button;
use iced::{Background, Border, Color, Theme as IcedTheme};

use crate::frontend::styles::colors::Theme;

pub fn text_color(theme: Theme, active: bool) -> Color {
    if active {
        theme.primary()
    } else {
        theme.neutral_front()
    }
}

pub fn underline(
    theme: Theme,
    active: bool,
) -> impl Fn(&IcedTheme) -> iced::widget::container::Style {
    move |_iced_theme: &IcedTheme| iced::widget::container::Style {
        background: Some(Background::Color(if active {
            theme.primary()
        } else {
            theme.transparent()
        })),
        ..iced::widget::container::Style::default()
    }
}

/// The tab is a plain (borderless, backgroundless) button so its label
/// and underline can be styled directly -- this only strips the default
/// button chrome.
pub fn tab_button(theme: Theme) -> impl Fn(&IcedTheme, button::Status) -> button::Style {
    move |_iced_theme: &IcedTheme, _status: button::Status| button::Style {
        background: None,
        text_color: theme.text_primary(),
        border: Border::default(),
        shadow: iced::Shadow::default(),
    }
}
