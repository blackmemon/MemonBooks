// White Theme color tokens (Section 6 of PROJECT_SPEC.md).
// Usage rule: no hex literals anywhere outside this file -- every component
// and view imports these constants instead.
//
// `Color::from_rgb8` isn't a `const fn` in the pinned iced version, so these
// are built as struct literals (`r`/`g`/`b`/`a` are public fields) to stay
// usable in `const` position.

use iced::Color;

const fn rgb8(r: u8, g: u8, b: u8) -> Color {
    Color {
        r: r as f32 / 255.0,
        g: g as f32 / 255.0,
        b: b as f32 / 255.0,
        a: 1.0,
    }
}

/// Primary accent -- buttons, active nav item, links, focus rings.
pub const WHITE_PRIMARY: Color = rgb8(0x51, 0xA2, 0xFF);

/// Success / positive background (badges, toasts, confirmations).
pub const WHITE_POSITIVE_BG: Color = rgb8(0xC6, 0xF1, 0xCE);
/// Success / positive text & icon color.
pub const WHITE_POSITIVE_FRONT: Color = rgb8(0x00, 0x64, 0x00);

/// Error / negative background (validation errors, destructive confirmations).
pub const WHITE_NEGATIVE_BG: Color = rgb8(0xFF, 0xC8, 0xCE);
/// Error / negative text & icon color.
pub const WHITE_NEGATIVE_FRONT: Color = rgb8(0x9C, 0x00, 0x0A);

/// Warning background (pending states, unsaved-changes notices).
pub const WHITE_WARNING_BG: Color = rgb8(0xFF, 0xEB, 0xA0);
/// Warning text & icon color.
pub const WHITE_WARNING_FRONT: Color = rgb8(0x9C, 0x5A, 0x00);

/// Neutral background -- status pills with no positive/negative/warning
/// meaning (e.g. "Draft"), muted panels.
pub const WHITE_NEUTRAL_BG: Color = rgb8(0xE5, 0xE7, 0xEB);
/// Neutral text & icon color. Doubles as the app's one muted/secondary
/// text tone (descriptions, secondary avatar lines, inactive tabs) and as
/// the subtle border/divider color for cards and table rows, so the shell
/// never needs a hex literal outside this file for those either.
pub const WHITE_NEUTRAL_FRONT: Color = rgb8(0x4B, 0x55, 0x63);
