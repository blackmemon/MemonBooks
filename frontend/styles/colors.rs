// White Theme color tokens (Section 6 of PROJECT_SPEC.md).
// Usage rule: no hex literals anywhere outside this file -- every component
// and view imports these constants instead.

use iced::Color;

/// Primary accent -- buttons, active nav item, links, focus rings.
pub const WHITE_PRIMARY: Color = Color::from_rgb8(0x51, 0xA2, 0xFF);

/// Success / positive background (badges, toasts, confirmations).
pub const WHITE_POSITIVE_BG: Color = Color::from_rgb8(0xC6, 0xF1, 0xCE);
/// Success / positive text & icon color.
pub const WHITE_POSITIVE_FRONT: Color = Color::from_rgb8(0x00, 0x64, 0x00);

/// Error / negative background (validation errors, destructive confirmations).
pub const WHITE_NEGATIVE_BG: Color = Color::from_rgb8(0xFF, 0xC8, 0xCE);
/// Error / negative text & icon color.
pub const WHITE_NEGATIVE_FRONT: Color = Color::from_rgb8(0x9C, 0x00, 0x0A);

/// Warning background (pending states, unsaved-changes notices).
pub const WHITE_WARNING_BG: Color = Color::from_rgb8(0xFF, 0xEB, 0xA0);
/// Warning text & icon color.
pub const WHITE_WARNING_FRONT: Color = Color::from_rgb8(0x9C, 0x5A, 0x00);
