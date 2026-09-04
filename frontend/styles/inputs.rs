//! Canonical `text_input` style (Section 6). Unlike `button.rs`, there
//! was nothing to consolidate here -- the Phase 1 audit found zero
//! styled `text_input`s anywhere in the app; every one of them (48+
//! call sites) falls back to iced's own built-in `theme.
//! extended_palette()`, a completely separate system from this app's
//! `Theme`, meaning every text field in the app currently ignores the
//! dark/light toggle entirely. This is a fresh design using this
//! app's existing tokens/visual language (matching `borders::card`'s
//! hairline-border treatment already used for cards and the
//! `secondary`-style button), not a reproduction of anything that
//! existed before -- user-confirmed direction, since there was nothing
//! to preserve.
//!
//! Also covers `textbox.rs`: iced has one text-entry widget
//! (`text_input`), not a separate "textbox" concept, so there's
//! nothing distinct for that file to hold -- see its own doc comment.

use iced::widget::text_input::{Status, Style};
use iced::{Background, Border, Color};

use crate::frontend::styles::colors::Theme;
use crate::frontend::styles::{borders, states};

/// A translucent primary tint for the selected-text highlight -- not
/// one of `states.rs`'s reconciled-from-the-audit constants (this
/// widget had no prior style for the audit to find), so it's scoped
/// here rather than implying it's an interactive hover/press/selected
/// state like those are.
const SELECTION_TINT_ALPHA: f32 = 0.25;

pub fn text(theme: Theme) -> impl Fn(&iced::Theme, Status) -> Style {
    move |_iced_theme: &iced::Theme, status: Status| {
        let background = if matches!(status, Status::Disabled) { theme.neutral_bg() } else { theme.surface() };
        let border_color = match status {
            Status::Focused => theme.primary(),
            _ => theme.neutral_bg(),
        };
        let text_color = if matches!(status, Status::Disabled) { theme.neutral_front() } else { theme.text_primary() };

        Style {
            background: Background::Color(background),
            border: Border { color: border_color, width: 1.0, radius: states::RADIUS_SM.into() },
            icon: theme.neutral_front(),
            placeholder: theme.neutral_front(),
            value: text_color,
            selection: Color { a: SELECTION_TINT_ALPHA, ..theme.primary() },
        }
    }
}
