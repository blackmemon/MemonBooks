//! Data table row styling (Section 6): a header background, a row that
//! highlights subtly on hover, and the hairline divider drawn between
//! rows instead of full alternating stripes.

use iced::widget::{button, container};
use iced::{Background, Border, Color, Theme};

use crate::frontend::styles::colors;

pub fn header_row() -> impl Fn(&Theme) -> container::Style {
    |_theme: &Theme| container::Style { background: Some(Background::Color(colors::WHITE_NEUTRAL_BG)), ..container::Style::default() }
}

/// A row rendered as a `button` so it gets hover feedback for free from
/// iced's `button::Status` -- pressing it opens that row's action menu,
/// same as its trailing kebab button.
pub fn row() -> impl Fn(&Theme, button::Status) -> button::Style {
    |_theme: &Theme, status: button::Status| {
        let background = match status {
            button::Status::Hovered | button::Status::Pressed => Some(Background::Color(Color { a: 0.5, ..colors::WHITE_NEUTRAL_BG })),
            _ => None,
        };
        button::Style { background, text_color: Color::BLACK, border: Border::default(), shadow: iced::Shadow::default() }
    }
}

pub fn divider_color() -> Color {
    colors::WHITE_NEUTRAL_BG
}
