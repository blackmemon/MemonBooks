//! Toolbar row styling (Section 6): the bordered "Filters" / "Sort by"
//! dropdown buttons and the search input, all sharing one height and a
//! hairline border so they read as one aligned row.

use iced::widget::button;
use iced::{Background, Color, Theme};

use crate::frontend::styles::{borders, colors};

pub const CONTROL_HEIGHT: f32 = 36.0;

pub fn dropdown_button() -> impl Fn(&Theme, button::Status) -> button::Style {
    |_theme: &Theme, status: button::Status| {
        let background = match status {
            button::Status::Hovered | button::Status::Pressed => Color { a: 0.6, ..colors::WHITE_NEUTRAL_BG },
            _ => Color::WHITE,
        };
        button::Style {
            background: Some(Background::Color(background)),
            text_color: colors::WHITE_NEUTRAL_FRONT,
            border: borders::card(borders::RADIUS_SM),
            shadow: iced::Shadow::default(),
        }
    }
}
