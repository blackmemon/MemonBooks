//! Named Unicode glyph constants (Section 6). This app has no bundled
//! icon font or SVG pipeline yet -- every icon today is a Unicode
//! character rendered through `text(...)` (see `sidebar.rs`'s
//! `icon_slot` doc comment for the same point about monogram avatars).
//! This file just names the glyphs the Phase 1 audit found scattered
//! as raw `\u{...}` literals across `sidebar.rs` and `data_table.rs`,
//! so a future real icon-font swap has one place to change per glyph
//! instead of a grep-and-replace across the app.
//!
//! The app font is a sans face that has no emoji or dingbat glyphs, so the
//! action icons below stick to characters that the common sans fonts
//! (Liberation Sans, Cantarell, Adwaita Sans) all carry; the earlier
//! magnifier, hamburger, pencil, cross and vertical-ellipsis glyphs drew
//! as empty boxes.

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
pub const KEBAB: &str = "\u{2026}";

/// Leading dot on a data table status pill (`Cell::Status`).
pub const STATUS_DOT: &str = "\u{25CF}";

/// Tree row toggle, shown when a parent node (an account with children)
/// is expanded -- points down since that's the direction its children
/// are listed. Kept distinct from `SORT_DESCENDING`, even though both
/// are the same down-triangle glyph today, since the two meanings
/// (tree state vs. column sort) have no reason to stay visually
/// coupled if either changes later.
pub const TREE_EXPANDED: &str = "\u{25BC}";

/// Tree row toggle, shown when a parent node is collapsed (points the
/// direction -- right, toward its hidden children -- a click would
/// expand it).
pub const TREE_COLLAPSED: &str = "\u{25BA}";

/// Search box prefix (Maintain Customers/Vendors).
pub const SEARCH: &str = "\u{25CB}";

/// "Reset all" icon button.
pub const RESET: &str = "\u{21BB}";

/// "Columns" picker icon button.
pub const COLUMNS: &str = "\u{2261}";

/// "Export" icon button.
pub const EXPORT: &str = "\u{2193}";

/// Close (X) button on a side panel.
pub const CLOSE: &str = "\u{00D7}";

/// Detail-panel field glyphs (Maintain Customers/Vendors).
pub const FIELD_NAME: &str = "\u{263A}";
pub const FIELD_COMPANY: &str = "\u{25A3}";
pub const FIELD_PHONE: &str = "\u{260E}";
pub const FIELD_EMAIL: &str = "\u{2709}";
pub const FIELD_WEBSITE: &str = "\u{2316}";
pub const FIELD_ADDRESS: &str = "\u{2302}";
pub const FIELD_PAYMENT: &str = "\u{00A4}";
pub const FIELD_NOTE: &str = "\u{270E}";
pub const FIELD_STATUS: &str = "\u{25CF}";
pub const FIELD_OTHER: &str = "\u{2022}";

/// "Back" arrow on an inline page view.
pub const ARROW_LEFT: &str = "\u{2190}";

/// Pencil on a row's "edit" icon button.
pub const EDIT: &str = "\u{203A}";

/// Dash drawn in a partially-checked (tri-state) checkbox.
pub const INDETERMINATE: &str = "\u{2212}";

/// Tick drawn in a checked box.
pub const CHECK: &str = "\u{2713}";
