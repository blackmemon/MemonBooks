//! White Theme typography scale (Section 6 of PROJECT_SPEC.md). Font:
//! OS-provided UI font stack (no bundled/custom font) -- see `fonts.rs`.
//!
//! Named by role, derived from the raw `.size(N)` literals the Phase 1
//! consistency audit actually found scattered across `frontend/views/`
//! and `frontend/components/` -- every value below is a real size in
//! use today, not a fresh design choice. `headers.rs` already owns
//! named sizes for page/section headings specifically
//! (`PAGE_TITLE_SIZE`, `PAGE_DESCRIPTION_SIZE`, `SECTION_HEADING_SIZE`,
//! `SECTION_DESCRIPTION_SIZE`) and is left as-is here -- it was already
//! compliant, not part of the audit's findings.
//!
//! Two things the audit's raw `.size(N)` count included that do **not**
//! belong in a font-size scale, so they're deliberately left out below:
//! - `checkbox(...).size(16)` (`data_table.rs`'s Columns panel) sets
//!   the checkbox widget's own box dimensions, not a font size. That's
//!   a Batch 2/6 (`checkbox.rs`) concern.
//! - Icon glyphs rendered through `text(...)` (kebab `⋮`, chevrons,
//!   sun/moon, the status-pill dot) reuse whichever size constant below
//!   matches their current value, but their *canonical* home is
//!   `icons.rs` (Batch 2/10), not this file.
//!
//! Two open questions flagged for a human call, not resolved here:
//! - [`PANEL_TITLE`] (15.0, "Add an account" / "Select Columns to
//!   Display") sits one step below `headers::SECTION_HEADING_SIZE`
//!   (16.0, e.g. "Account list") and reads as a near-duplicate of it --
//!   nothing in the code ties them together, so they're kept distinct
//!   here rather than silently merged. Worth a design decision on
//!   whether panel titles should just *be* section headings.
//! - [`TITLE`] (24.0, standalone page titles on views that don't use
//!   the shared `page_header::view` -- login, switch user, password
//!   change, dashboard, POS, reports, settings) is one step below
//!   `headers::PAGE_TITLE_SIZE` (26.0, the size `page_header::view`
//!   actually renders titles at). Whether those seven views should
//!   adopt the shared component (making 24 disappear entirely) is a
//!   real redesign question, out of scope for a consolidation pass --
//!   preserved at its current value here instead.

/// Tiny glyph/indicator text -- today, only the status-pill's leading
/// dot (`data_table::cell_view`'s `Cell::Status` rendering).
pub const MICRO: f32 = 9.0;

/// Secondary/subtitle text sitting beside or under something more
/// prominent -- avatar subtitles, the sidebar's user subtitle and
/// dropdown caret, a table row's progress-bar percentage label.
pub const CAPTION: f32 = 11.0;

/// Base/default text size -- this is the value the original spec's
/// (previously unused) `BASE_FONT_SIZE` already named; kept as the
/// same number here under a name that groups it with the rest of the
/// scale. Used for muted/secondary body copy: field labels like
/// "Reference Code", `headers::SECTION_DESCRIPTION_SIZE`-adjacent text
/// that isn't going through that constant.
pub const BODY: f32 = 12.0;

/// The single most common size in the app (39 of the audit's 87 raw
/// `.size(N)` call sites) -- button labels, form `text_input`s, table
/// cell text, toolbar controls. Also what `headers::
/// PAGE_DESCRIPTION_SIZE` independently already equals.
pub const LABEL: f32 = 13.0;

/// Slightly emphasized incidental text and small icon glyphs one step
/// above [`LABEL`] -- the page header's "+" action-button prefix, the
/// kebab menu button, sidebar's collapse-chevron and theme-toggle
/// icons, a couple of small inline value displays (e.g. the Add
/// Account panel's generated reference code).
pub const EMPHASIS: f32 = 14.0;

/// In-panel/card heading, one step below a full section heading -- see
/// the open question above about whether this should just be
/// `headers::SECTION_HEADING_SIZE` instead.
pub const PANEL_TITLE: f32 = 15.0;

/// Sub-heading size for a titled block within a page -- report section
/// titles ("Balance Sheet", "Profit & Loss"), a prominently-displayed
/// generated value (the one-time-shown new user password).
pub const SUBTITLE: f32 = 20.0;

/// Standalone page title on views that render their own heading
/// directly instead of going through `page_header::view` -- see the
/// open question above about `headers::PAGE_TITLE_SIZE`.
pub const TITLE: f32 = 24.0;

/// A large, prominently-displayed numeric value -- today, only the
/// dashboard's stat tiles. Numerically identical to [`TITLE`] in the
/// audit, but a different *role* (a data value, not a heading) that
/// could reasonably diverge from it later -- kept as its own named
/// constant rather than reusing `TITLE` for an unrelated purpose.
pub const STAT_VALUE: f32 = 24.0;
