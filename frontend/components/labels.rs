//! Text labels (Section 6): the muted caption/label and the bold dark
//! heading text that views repeat everywhere.

use iced::widget::{column, text, Text};
use iced::{Element, Length};

use crate::frontend::components::colors::Theme;
use crate::frontend::components::{fonts, spacing, typography};

/// A muted caption over a bold value, sharing its row equally with its
/// neighbours -- one figure in a summary strip.
pub fn stat<'a, M: 'a>(
    theme: Theme,
    label: impl text::IntoFragment<'a>,
    value: impl text::IntoFragment<'a>,
) -> Element<'a, M> {
    column![
        muted(theme, label, typography::caption()),
        text(value)
            .size(typography::emphasis())
            .font(fonts::BOLD)
            .color(theme.neutral_font()),
    ]
    .spacing(spacing::SPACE_2)
    .width(Length::FillPortion(1))
    .into()
}

/// Small gray text: captions, field labels, secondary lines.
pub fn muted<'a>(theme: Theme, value: impl text::IntoFragment<'a>, size: f32) -> Text<'a> {
    text(value).size(size).color(theme.neutral_front())
}

/// Bold dark text: page and panel titles, names.
pub fn strong<'a>(theme: Theme, value: impl text::IntoFragment<'a>, size: f32) -> Text<'a> {
    text(value)
        .size(size)
        .font(fonts::BOLD)
        .color(theme.neutral_font())
}
