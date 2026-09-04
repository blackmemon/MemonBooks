//! Shared spacing scale (Section 6). Every new shell/list component uses
//! these instead of ad-hoc padding/spacing numbers, so the app-wide
//! rhythm stays consistent as more views adopt the pattern.
//!
//! `XS` through `XL` are the original five tokens. The Phase 1
//! consistency audit found six more distinct values in real use across
//! the app -- `2`, `6`, `10`, `12`, `20`, and `40` -- that don't land on
//! any of the five. Each is genuinely different from its neighbors, not
//! typo drift (e.g. `20` alone accounts for 29 of the 258 raw calls the
//! audit found, as the standard content-column padding), so each keeps
//! its own constant rather than being rounded to the nearest existing
//! tier. Named by pixel value (`SPACE_N`) rather than invented T-shirt
//! sizes, since there's no honest "between XS and SM" word for six more
//! sizes -- the number is the name.
//!
//! A seventh, `SPACE_14`, surfaced only while migrating array-form
//! `.padding([vertical, horizontal])` calls (Batch 9) -- it never
//! appears as a bare single-value `.padding(N)`, only as the
//! horizontal half of a button's `[8.0, 14.0]` pair (11 call sites), so
//! the original single-value audit pass didn't catch it. It numerically
//! matches `typography::EMPHASIS` (`14.0`), but that's coincidence, not
//! shared meaning -- one is a padding distance, the other a font size --
//! so it gets its own constant here rather than reusing that one.

use iced::Padding;

pub const XS: f32 = 4.0;
pub const SM: f32 = 8.0;
pub const MD: f32 = 16.0;
pub const LG: f32 = 24.0;
pub const XL: f32 = 32.0;

pub const SPACE_2: f32 = 2.0;
pub const SPACE_6: f32 = 6.0;
pub const SPACE_10: f32 = 10.0;
pub const SPACE_12: f32 = 12.0;
pub const SPACE_14: f32 = 14.0;
pub const SPACE_20: f32 = 20.0;
pub const SPACE_40: f32 = 40.0;

/// The "Maintain" views' page-header padding: top/right/left `LG`, no
/// bottom (the gap down to the body comes from the column's own
/// `spacing`, not from this). Copy-pasted identically, byte for byte,
/// 11 times across `chart_of_accounts_view.rs`, `vendors_view.rs`,
/// `warehouse_view.rs`, `bank_of_accounts_view.rs`, `customers_view.rs`,
/// and `src/main.rs` before this constant existed.
pub const HEADER_PADDING: Padding = Padding { top: LG, right: LG, bottom: 0.0, left: LG };

/// The same five views' body-content padding: right/left/bottom `LG`,
/// no top (the header above already supplies that gap via
/// [`HEADER_PADDING`] plus the wrapping column's own `spacing`).
/// Copy-pasted identically, byte for byte, 5 times before this constant
/// existed.
pub const CONTENT_PADDING: Padding = Padding { top: 0.0, right: LG, bottom: LG, left: LG };
