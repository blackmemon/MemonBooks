//! Data table row styling (Section 6): a header background, a row that
//! highlights subtly on hover, and the hairline divider drawn between
//! rows instead of full alternating stripes.

use iced::widget::{button, container};
use iced::{Background, Border, Color, Theme as IcedTheme};

use crate::frontend::styles::colors::Theme;

pub fn header_row(theme: Theme) -> impl Fn(&IcedTheme) -> container::Style {
    move |_iced_theme: &IcedTheme| container::Style { background: Some(Background::Color(theme.neutral_bg())), ..container::Style::default() }
}

/// A row rendered as a `button` so it gets hover feedback for free from
/// iced's `button::Status` -- pressing it opens that row's action menu,
/// same as its trailing kebab button.
pub fn row(theme: Theme) -> impl Fn(&IcedTheme, button::Status) -> button::Style {
    move |_iced_theme: &IcedTheme, status: button::Status| {
        let background = match status {
            button::Status::Hovered | button::Status::Pressed => Some(Background::Color(Color { a: 0.5, ..theme.neutral_bg() })),
            _ => None,
        };
        button::Style { background, text_color: theme.text_primary(), border: Border::default(), shadow: iced::Shadow::default() }
    }
}

pub fn divider_color(theme: Theme) -> Color {
    theme.neutral_bg()
}
