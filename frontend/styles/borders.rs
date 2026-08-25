//! Shared corner radii and border helpers (Section 6). Border *color*
//! always comes from the active theme's `neutral_bg` token -- this file
//! adds no new hex literals, only radius values and small `iced::Border`
//! builders composed from that token.

use iced::Border;

use crate::frontend::styles::colors::Theme;

pub const RADIUS_SM: f32 = 6.0;
pub const RADIUS_MD: f32 = 10.0;
pub const RADIUS_LG: f32 = 14.0;
pub const RADIUS_PILL: f32 = 999.0;

/// A hairline border for cards (data table, panels): subtle, same tone as
/// the neutral background so it reads as a boundary, not a rule.
pub fn card(theme: Theme, radius: f32) -> Border {
    Border { color: theme.neutral_bg(), width: 1.0, radius: radius.into() }
}
