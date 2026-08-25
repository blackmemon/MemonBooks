//! Toolbar row styling (Section 6): the bordered "Filters" / "Sort by"
//! dropdown buttons and the search input, all sharing one height and a
//! hairline border so they read as one aligned row.

use iced::widget::button;
use iced::{Background, Theme as IcedTheme};

use crate::frontend::styles::borders;
use crate::frontend::styles::colors::Theme;

pub const CONTROL_HEIGHT: f32 = 36.0;

pub fn dropdown_button(theme: Theme) -> impl Fn(&IcedTheme, button::Status) -> button::Style {
    move |_iced_theme: &IcedTheme, status: button::Status| {
        let background = match status {
            button::Status::Hovered | button::Status::Pressed => iced::Color { a: 0.6, ..theme.neutral_bg() },
            _ => theme.surface(),
        };
        button::Style {
            background: Some(Background::Color(background)),
            text_color: theme.neutral_front(),
            border: borders::card(theme, borders::RADIUS_SM),
            shadow: iced::Shadow::default(),
        }
    }
}
