//! Shared corner radii and border helpers (Section 6). Border *color*
//! always comes from `colors::WHITE_NEUTRAL_FRONT`/`WHITE_NEUTRAL_BG` --
//! this file adds no new hex literals, only radius values and small
//! `iced::Border` builders composed from those existing tokens.

use iced::Border;

use crate::frontend::styles::colors;

pub const RADIUS_SM: f32 = 6.0;
pub const RADIUS_MD: f32 = 10.0;
pub const RADIUS_LG: f32 = 14.0;
pub const RADIUS_PILL: f32 = 999.0;

/// A hairline border for cards (data table, panels): subtle, same tone as
/// the neutral background so it reads as a boundary, not a rule.
pub fn card(radius: f32) -> Border {
    Border { color: colors::WHITE_NEUTRAL_BG, width: 1.0, radius: radius.into() }
}
