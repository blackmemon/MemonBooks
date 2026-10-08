//! Status pill tones (Section 6): maps a meaning to the existing White
//! Theme background/front color pair. No new colors -- this only picks
//! which existing pair applies, plus the one new neutral pair.

use iced::widget::container;
use iced::{Background, Border, Color, Theme as IcedTheme};

use crate::frontend::components::colors::Theme;
use crate::frontend::components::states;

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

/// Pill for a count next to a tab label: primary tint when `active`,
/// neutral otherwise.
pub fn count(theme: Theme, active: bool) -> impl Fn(&IcedTheme) -> container::Style {
    move |_iced_theme: &IcedTheme| container::Style {
        background: Some(Background::Color(if active {
            theme.primary_tint(states::SELECTED_ALPHA)
        } else {
            theme.neutral_bg()
        })),
        border: Border {
            radius: states::RADIUS_PILL.into(),
            ..Border::default()
        },
        ..container::Style::default()
    }
}

/// Plain tinted status pill (no dot): the tone's background, fully
/// rounded. Pair the text with `tone_colors(..).1`.
pub fn pill(theme: Theme, tone: BadgeTone) -> impl Fn(&IcedTheme) -> container::Style {
    move |_iced_theme: &IcedTheme| container::Style {
        background: Some(Background::Color(tone_colors(theme, tone).0)),
        border: Border {
            radius: states::RADIUS_PILL.into(),
            ..Border::default()
        },
        ..container::Style::default()
    }
}

/// Round initials avatar: a primary tint disc. Size it with
/// `spacing::AVATAR_SIZE` and color the initials `theme.primary()`.
pub fn avatar(theme: Theme) -> impl Fn(&IcedTheme) -> container::Style {
    move |_iced_theme: &IcedTheme| container::Style {
        background: Some(Background::Color(theme.primary_tint(states::SELECTED_ALPHA))),
        border: Border {
            radius: states::RADIUS_PILL.into(),
            ..Border::default()
        },
        ..container::Style::default()
    }
}
