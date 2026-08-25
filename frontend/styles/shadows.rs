//! Shared shadow presets (Section 6). Used for the data table card and
//! similar contained panels -- a soft, closely-clipped shadow rather than
//! a full-bleed page shadow.

use iced::{Color, Shadow, Vector};

use crate::frontend::styles::colors::Theme;

/// A contained shadow for cards floating slightly above the page
/// background: soft, short throw, low opacity.
pub fn card(theme: Theme) -> Shadow {
    Shadow { color: Color { a: 0.08, ..theme.neutral_front() }, offset: Vector::new(0.0, 1.0), blur_radius: 8.0 }
}
