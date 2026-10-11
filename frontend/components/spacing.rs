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
//! matches `typography::emphasis()` (`14.0`), but that's coincidence, not
//! shared meaning -- one is a padding distance, the other a font size --
//! so it gets its own constant here rather than reusing that one.

use iced::Padding;

pub const XS: f32 = 4.0;
pub const SM: f32 = 8.0;
pub const MD: f32 = 16.0;
pub const LG: f32 = 24.0;
pub const XL: f32 = 32.0;

pub const SPACE_2: f32 = 2.0;
pub const SPACE_4: f32 = 4.0;
pub const SPACE_6: f32 = 6.0;
pub const SPACE_8: f32 = 8.0;
pub const SPACE_10: f32 = 10.0;
pub const SPACE_12: f32 = 12.0;
pub const SPACE_14: f32 = 14.0;
pub const SPACE_18: f32 = 18.0;
pub const SPACE_24: f32 = 24.0;
pub const SPACE_30: f32 = 30.0;
pub const SPACE_5: f32 = 5.0;
pub const SPACE_1: f32 = 1.0;
pub const SPACE_20: f32 = 20.0;
pub const SPACE_40: f32 = 40.0;
/// Side of a square icon-only button.
pub const ICON_BUTTON_SIZE: f32 = 36.0;
/// Width of a numbered pagination button.
pub const PAGE_BUTTON_WIDTH: f32 = 32.0;
/// Height of the underline under a tab.
pub const TAB_UNDERLINE_HEIGHT: f32 = 2.0;
/// Left padding that leaves room for a glyph inside a search box.
pub const SEARCH_ICON_INSET: f32 = 36.0;
/// Data table card pieces: row height, round avatar diameter, leading
/// checkbox column, trailing actions column, header search width.
pub const TABLE_ROW_HEIGHT: f32 = 52.0;
pub const AVATAR_SIZE: f32 = 36.0;
pub const CHECKBOX_COLUMN_WIDTH: f32 = 24.0;
pub const ACTION_COLUMN_WIDTH: f32 = 48.0;
pub const SEARCH_WIDTH: f32 = 260.0;
/// Width of a vertical scrollbar and of the gutter reserved beside it.
pub const SCROLLBAR_WIDTH: f32 = 8.0;
pub const HEADER_PADDING: Padding = Padding {
    top: LG,
    right: LG,
    bottom: 0.0,
    left: LG,
};
pub const CONTENT_PADDING: Padding = Padding {
    top: 0.0,
    right: LG,
    bottom: LG,
    left: LG,
};

/// Responsive breakpoints, defined once for every page. A width below
/// `BREAKPOINT_NARROW` is [`LayoutClass::Narrow`], one below
/// `BREAKPOINT_WIDE` is [`LayoutClass::Medium`], anything else is
/// [`LayoutClass::Wide`].
pub const BREAKPOINT_NARROW: f32 = 800.0;
pub const BREAKPOINT_WIDE: f32 = 1200.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutClass {
    Narrow,
    Medium,
    Wide,
}

pub fn layout_class(width: f32) -> LayoutClass {
    if width < BREAKPOINT_NARROW {
        LayoutClass::Narrow
    } else if width < BREAKPOINT_WIDE {
        LayoutClass::Medium
    } else {
        LayoutClass::Wide
    }
}

/// Side panel next to a table: never narrower than `PANEL_MIN_WIDTH` (so
/// inputs never clip), one third of the page when wide, and
/// `PANEL_MEDIUM_SHARE` of it when medium.
pub const PANEL_MIN_WIDTH: f32 = 420.0;
pub const PANEL_WIDE_SHARE: f32 = 1.0 / 3.0;
pub const PANEL_MEDIUM_SHARE: f32 = 0.45;
/// A panel at least this wide lays its fields out in two columns.
pub const PANEL_TWO_COLUMN_MIN_WIDTH: f32 = 520.0;
/// Smallest width of a header or footer button, so its label never wraps.
pub const BUTTON_MIN_WIDTH: f32 = 72.0;
/// Narrowest the header search box is allowed to shrink to.
pub const SEARCH_MIN_WIDTH: f32 = 140.0;
