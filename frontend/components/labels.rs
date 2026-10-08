//! Text labels (Section 6): the muted caption/label and the bold dark
//! heading text that views repeat everywhere.

use iced::widget::{text, Text};

use crate::frontend::components::colors::Theme;
use crate::frontend::components::fonts;

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
