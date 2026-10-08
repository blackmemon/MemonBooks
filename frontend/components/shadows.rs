//! Shared shadow presets (Section 6). Used for the data table card and
//! similar contained panels -- a soft, closely-clipped shadow rather than
//! a full-bleed page shadow.

use iced::{Color, Shadow, Vector};

use crate::frontend::components::colors::Theme;

/// Shadow opacity for [`card`] -- not one of `states.rs`'s reconciled
/// interactive-state constants (a shadow's own opacity isn't a hover/
/// pressed/selected state, and this happens to numerically match
/// `states::TINTED_PANEL_ALPHA` by coincidence, not shared meaning), so
/// it's scoped here rather than implying a connection that isn't real.
const CARD_SHADOW_ALPHA: f32 = 0.08;

/// A contained shadow for cards floating slightly above the page
/// background: soft, short throw, low opacity.
pub fn card(theme: Theme) -> Shadow {
    Shadow {
        color: Color {
            a: CARD_SHADOW_ALPHA,
            ..theme.neutral_front()
        },
        offset: Vector::new(0.0, 1.0),
        blur_radius: 8.0,
    }
}
