//! Named opacity tokens for interactive states (Section 6) -- reconciled
//! from the alpha values already scattered across `components/common`
//! and `src/main.rs` (see the Phase 1 consistency audit), not invented.
//! Every `Color { a: N, ..theme.X() }` found in that audit turned out to
//! express one of a small number of *distinct* meanings, not the same
//! value copy-pasted with different names -- the alpha is doing
//! different visual work (a solid button darkening vs. an outline
//! button gaining a light fill vs. a standing selected-state tint) even
//! in places where the *word* for the state ("hover") is the same.
//!
//! `Hovered` and `Pressed` are visually identical everywhere this
//! codebase currently styles them -- every `Status::Hovered | Status::
//! Pressed => ...` arm found in the audit used one shared value for
//! both. So there is no separate PRESSED_* set of constants here; the
//! `_HOVER_ALPHA` constants below cover both states, matched as
//! `Hovered | Pressed` at the call site, same as every existing caller
//! already does. If a future design wants Pressed to look different
//! from Hovered, that is a new value to add here, not something this
//! audit found evidence for.
//!
//! `Disabled` never appeared as an alpha blend in the audit -- the one
//! disabled button found (`toolbar::destructive_button`) swaps to a
//! flat, fully-opaque `theme.neutral_bg()` background and
//! `theme.neutral_front()` text instead of blending an alpha over the
//! active-state color. That is a color-*swap* convention, not an
//! opacity one, so it is not represented here as a constant -- follow
//! the same swap-to-neutral pattern at future disabled-state call
//! sites rather than inventing a DISABLED_ALPHA that doesn't correspond
//! to anything the audit actually found.

/// Hover/pressed background for a filled/solid button whose resting
/// state is already a strong, saturated fill (a primary or negative
/// base color) -- darkens it slightly rather than lightening. Found
/// identically at four call sites: `bulk_action_bar::destructive_button`,
/// `page_header::action_button`, `toolbar::destructive_button` (the one
/// inside `list_toolbar`), and `src/main.rs`'s `transaction_side_button`
/// (active/selected branch) -- all independently used exactly `0.85`.
pub const SOLID_BUTTON_HOVER_ALPHA: f32 = 0.85;

/// Hover/pressed/opened background for a neutral, bordered control --
/// buttons styled like `toolbar::dropdown_button`, the sidebar's
/// profile card, and every `pick_list` this app currently styles.
/// Found identically at six call sites, all independently blending
/// exactly `0.6` over `theme.neutral_bg()`: `bulk_action_bar::
/// secondary_button`, `sidebar::profile_card_style`, `toolbar_style::
/// dropdown_button`, `toolbar::status_picker_style`, `src/main.rs`'s
/// `transaction_side_button` (inactive branch), and `src/main.rs`'s
/// `maintain_picker_style`. The single most duplicated value in the
/// whole audit.
pub const NEUTRAL_CONTROL_HOVER_ALPHA: f32 = 0.6;

/// Hover/pressed background *tint* for an outline-style button -- one
/// whose resting state is a colored border with no fill (e.g. a data
/// table row's primary-action button, "Switch to"/"Record Payment").
/// Much lighter than [`SOLID_BUTTON_HOVER_ALPHA`] since it's adding a
/// fill where there was none, not darkening an existing one. From
/// `data_table::primary_row_button`, the only place this pattern
/// exists today.
pub const OUTLINE_BUTTON_HOVER_ALPHA: f32 = 0.1;

/// Hover background for a plain/ghost row with no border or fill at
/// rest -- the sidebar's *inactive* nav row is the only example today.
/// Barely-there compared to the control-hover constants above, since
/// this is text sitting directly on the sidebar background rather than
/// a bordered/filled control gaining emphasis. From `sidebar::nav_row`.
pub const GHOST_ROW_HOVER_ALPHA: f32 = 0.06;

/// Standing (not hover-triggered) background for a *selected/active*
/// item -- e.g. the sidebar's current-page nav pill. Distinct from
/// every hover constant above: this persists for as long as the item
/// is selected, it doesn't fade in and out with the pointer. From
/// `sidebar::nav_row`'s `item.active` branch.
pub const SELECTED_ALPHA: f32 = 0.14;

/// Subtle, permanent background tint for a panel that wants to read as
/// related to the primary action without competing with it -- e.g. the
/// bulk-action bar shown above a table once rows are selected. Not a
/// hover state -- applied unconditionally while the panel is shown.
/// From `bulk_action_bar`'s own container background, the only place
/// this pattern exists today.
pub const TINTED_PANEL_ALPHA: f32 = 0.08;
