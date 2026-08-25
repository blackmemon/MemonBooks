//! Status pill tones (Section 6): maps a meaning to the existing White
//! Theme background/front color pair. No new colors -- this only picks
//! which existing pair applies, plus the one new neutral pair.

use iced::Color;

use crate::frontend::styles::colors::Theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BadgeTone {
    Positive,
    Negative,
    Warning,
    Neutral,
}

/// Returns `(background, foreground)` for the given tone under the
/// active theme.
pub fn tone_colors(theme: Theme, tone: BadgeTone) -> (Color, Color) {
    match tone {
        BadgeTone::Positive => (theme.positive_bg(), theme.positive_front()),
        BadgeTone::Negative => (theme.negative_bg(), theme.negative_front()),
        BadgeTone::Warning => (theme.warning_bg(), theme.warning_front()),
        BadgeTone::Neutral => (theme.neutral_bg(), theme.neutral_front()),
    }
}
