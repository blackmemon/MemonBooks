//! Status pill tones (Section 6): maps a meaning to the existing White
//! Theme background/front color pair. No new colors -- this only picks
//! which existing pair applies, plus the one new neutral pair.

use iced::Color;

use crate::frontend::styles::colors;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BadgeTone {
    Positive,
    Negative,
    Warning,
    Neutral,
}

/// Returns `(background, foreground)` for the given tone.
pub fn tone_colors(tone: BadgeTone) -> (Color, Color) {
    match tone {
        BadgeTone::Positive => (colors::WHITE_POSITIVE_BG, colors::WHITE_POSITIVE_FRONT),
        BadgeTone::Negative => (colors::WHITE_NEGATIVE_BG, colors::WHITE_NEGATIVE_FRONT),
        BadgeTone::Warning => (colors::WHITE_WARNING_BG, colors::WHITE_WARNING_FRONT),
        BadgeTone::Neutral => (colors::WHITE_NEUTRAL_BG, colors::WHITE_NEUTRAL_FRONT),
    }
}
