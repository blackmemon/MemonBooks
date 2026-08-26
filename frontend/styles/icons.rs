//! Named Unicode glyph constants (Section 6). This app has no bundled
//! icon font or SVG pipeline yet -- every icon today is a Unicode
//! character rendered through `text(...)` (see `sidebar.rs`'s
//! `icon_slot` doc comment for the same point about monogram avatars).
//! This file just names the glyphs the Phase 1 audit found scattered
//! as raw `\u{...}` literals across `sidebar.rs` and `data_table.rs`,
//! so a future real icon-font swap has one place to change per glyph
//! instead of a grep-and-replace across the app.

/// Sidebar collapse toggle, shown when the sidebar is collapsed
/// (points the direction a click would expand it).
pub const CHEVRON_RIGHT: &str = "\u{203A}";

/// Sidebar collapse toggle, shown when the sidebar is expanded (points
/// the direction a click would collapse it).
pub const CHEVRON_LEFT: &str = "\u{2039}";

/// Dropdown/expandable indicator -- today, the sidebar profile card.
pub const CARET_DOWN: &str = "\u{25BE}";

/// Theme toggle, shown while dark mode is active (i.e. this is what a
/// click switches *to* -- light mode).
pub const SUN: &str = "\u{2600}";

/// Theme toggle, shown while light mode is active (i.e. this is what a
/// click switches *to* -- dark mode).
pub const MOON: &str = "\u{1F319}";

/// Data table column-header ascending-sort indicator.
pub const SORT_ASCENDING: &str = "\u{25B2}";

/// Data table column-header descending-sort indicator.
pub const SORT_DESCENDING: &str = "\u{25BC}";

/// Data table row's secondary-actions menu trigger ("more options").
pub const KEBAB: &str = "\u{22EE}";

/// Leading dot on a data table status pill (`Cell::Status`).
pub const STATUS_DOT: &str = "\u{25CF}";
