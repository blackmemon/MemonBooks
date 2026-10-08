use super::font_scale::scaled;

/// Smallest size in the scale.
pub fn micro() -> f32 {
    scaled(9.0)
}

/// Secondary/subtitle text sitting beside or under something more
/// prominent -- avatar subtitles, the sidebar's user subtitle and
/// dropdown caret, a table row's progress-bar percentage label.
pub fn caption() -> f32 {
    scaled(11.0)
}

/// Base/default text size -- this is the value the original spec's
/// (previously unused) `BASE_FONT_SIZE` already named; kept as the
/// same number here under a name that groups it with the rest of the
/// scale. Used for muted/secondary body copy: field labels like
/// "Reference Code", `headers::section_description_size()`-adjacent
/// text that isn't going through that constant.
pub fn body() -> f32 {
    scaled(12.0)
}

/// Native checkbox control dimension (not a text size).
pub fn checkbox_size() -> f32 {
    16.0
}

/// The single most common size in the app (39 of the audit's 87 raw
/// `.size(N)` call sites) -- button labels, form `text_input`s, table
/// cell text, toolbar controls. Also what `headers::
/// page_description_size()` independently already equals.
pub fn label() -> f32 {
    scaled(13.0)
}

/// Slightly emphasized incidental text and small icon glyphs one step
/// above [`label`] -- the page header's "+" action-button prefix, the
/// kebab menu button, sidebar's collapse-chevron and theme-toggle
/// icons, a couple of small inline value displays (e.g. the Add
/// Account panel's generated reference code).
pub fn emphasis() -> f32 {
    scaled(14.0)
}

/// In-panel/card heading, one step below a full section heading -- see
/// the open question above about whether this should just be
/// `headers::section_heading_size()` instead.
pub fn panel_title() -> f32 {
    scaled(15.0)
}

/// Sub-heading size for a titled block within a page -- report section
/// titles ("Balance Sheet", "Profit & Loss"), a prominently-displayed
/// generated value (the one-time-shown new user password).
pub fn subtitle() -> f32 {
    scaled(20.0)
}

/// Standalone page title on views that render their own heading
/// directly instead of going through `page_header::view` -- see the
/// open question above about `headers::page_title_size()`.
pub fn title() -> f32 {
    scaled(24.0)
}

/// A large, prominently-displayed numeric value -- today, only the
/// dashboard's stat tiles. Numerically identical to [`title`] in the
/// audit, but a different *role* (a data value, not a heading) that
/// could reasonably diverge from it later -- kept as its own named
/// getter rather than reusing `title` for an unrelated purpose.
pub fn stat_value() -> f32 {
    scaled(24.0)
}

/// The sidebar's own bold app-name/brand text, next to its logo mark.
/// Numerically identical to [`panel_title`], but a different role (a
/// fixed brand label, not a panel heading) -- same reasoning as
/// [`stat_value`] vs [`title`]. Missed by the original Phase 1 audit
/// because it reaches `.size()` through `sidebar.rs`'s local
/// `bold_text` helper rather than a direct `.size(N)` call site;
/// caught while building Batch 13's regression guard.
pub fn app_name() -> f32 {
    scaled(15.0)
}

/// The sidebar's bold user-name text in the profile card, one step
/// above its subtitle ([`caption`], 11.0) and one step below
/// [`label`] (13.0). A genuinely new, previously undiscovered value --
/// same missed-by-a-helper-function story as [`app_name`].
pub fn profile_name() -> f32 {
    scaled(12.5)
}
