//! Canonical `button` styles (Section 6). Every variant below already
//! existed as an inline closure somewhere in `components/common/` or
//! `src/main.rs` before this file did -- the Phase 1 consistency audit
//! found six separately-written closures that were byte-for-byte
//! identical to another one elsewhere (collapsed into one function
//! each here) and one pair that differed only in an unreachable
//! `Disabled` branch (kept as the more complete version, since it's a
//! strict superset, not a design change -- see [`destructive`]). Two
//! more turned out to be genuinely different treatments once compared
//! side by side (different rest-state colors, not just different hover
//! alphas) and stayed as their own variants: [`profile_card`] and
//! [`secondary`] both hover via
//! `states::NEUTRAL_CONTROL_HOVER_ALPHA`, but `profile_card` is opaque
//! `neutral_bg` at rest while `secondary` is `theme.surface()` -- the
//! same alpha produces a *lighter* hover on one and a *darker* hover
//! on the other, so they could not become one function without
//! changing how one of them looks.
//!
//! Every function here returns `impl Fn(&IcedTheme, button::Status) ->
//! button::Style`, so a call site just writes
//! `.style(button::secondary(theme))` -- matching the convention
//! `toolbar::dropdown_button` already established, rather than the
//! `.style(move |_iced_theme, status| some_fn(theme, status))` wrapper
//! several of the closures being consolidated here used instead.

use iced::widget::button::{Status, Style};
use iced::{Background, Border, Color, Theme as IcedTheme};

use crate::frontend::styles::colors::Theme;
use crate::frontend::styles::{borders, states};

/// Solid, primary-colored fill -- the one "this is the main action"
/// button treatment in the app (a page header's "+ Add X", a selected
/// side/tab toggle). Darkens on hover/press via
/// [`states::SOLID_BUTTON_HOVER_ALPHA`]. Collapsed from two previously
/// separate, identical closures: `page_header::action_button` and
/// `src/main.rs`'s `transaction_side_button` (active branch).
pub fn primary(theme: Theme) -> impl Fn(&IcedTheme, Status) -> Style {
    move |_iced_theme: &IcedTheme, status: Status| {
        let background = match status {
            Status::Hovered | Status::Pressed => Color { a: states::SOLID_BUTTON_HOVER_ALPHA, ..theme.primary() },
            _ => theme.primary(),
        };
        Style { background: Some(Background::Color(background)), text_color: Color::WHITE, border: Border { radius: borders::RADIUS_SM.into(), ..Border::default() }, shadow: iced::Shadow::default() }
    }
}

/// Bordered, neutral-surface button -- the app's default "secondary
/// action" treatment (Export, Reset, Cancel-as-dropdown-style, the
/// Columns toggle). Rest state is `theme.surface()`; hovers to
/// [`states::NEUTRAL_CONTROL_HOVER_ALPHA`] over `neutral_bg`.
/// Collapsed from three previously separate, identical closures:
/// `bulk_action_bar::secondary_button`, `toolbar::dropdown_button`,
/// and `src/main.rs`'s `transaction_side_button` (inactive branch).
pub fn secondary(theme: Theme) -> impl Fn(&IcedTheme, Status) -> Style {
    move |_iced_theme: &IcedTheme, status: Status| {
        let background = match status {
            Status::Hovered | Status::Pressed => Color { a: states::NEUTRAL_CONTROL_HOVER_ALPHA, ..theme.neutral_bg() },
            _ => theme.surface(),
        };
        Style { background: Some(Background::Color(background)), text_color: theme.neutral_front(), border: borders::card(theme, borders::RADIUS_SM), shadow: iced::Shadow::default() }
    }
}

/// Solid, negative/danger-colored fill -- for a destructive action
/// (Delete). Darkens on hover/press via
/// [`states::SOLID_BUTTON_HOVER_ALPHA`]; disabled swaps to a flat,
/// opaque neutral color pair rather than blending an alpha (see
/// `states.rs`'s note on why there's no `DISABLED_ALPHA`). Collapsed
/// from two previously separate closures -- `bulk_action_bar::
/// destructive_button` and `toolbar::destructive_button` -- that
/// agreed on every branch except one: only the `toolbar` version had a
/// `Disabled` arm. `bulk_action_bar`'s Delete button is always built
/// with a plain `.on_press` (never `on_press_maybe`), so it can never
/// actually receive `Disabled` -- using the more complete version here
/// changes nothing observable for it, and gives it correct handling if
/// that ever changes.
pub fn destructive(theme: Theme) -> impl Fn(&IcedTheme, Status) -> Style {
    move |_iced_theme: &IcedTheme, status: Status| {
        let background = match status {
            Status::Disabled => theme.neutral_bg(),
            Status::Hovered | Status::Pressed => Color { a: states::SOLID_BUTTON_HOVER_ALPHA, ..theme.negative_front() },
            Status::Active => theme.negative_front(),
        };
        let text_color = if matches!(status, Status::Disabled) { theme.neutral_front() } else { Color::WHITE };
        Style { background: Some(Background::Color(background)), text_color, border: Border { radius: borders::RADIUS_SM.into(), ..Border::default() }, shadow: iced::Shadow::default() }
    }
}

/// Borderless "ghost" button for an icon-only control (the sidebar's
/// collapse toggle and theme toggle, a table row's kebab menu, a bulk
/// action bar's "Clear"). Transparent at rest, opaque `neutral_bg` on
/// hover -- not an alpha blend, the original closures all used the
/// flat color directly. Collapsed from three previously separate,
/// identical closures: `bulk_action_bar::plain_button`, `sidebar::
/// plain_button`, and `data_table::kebab_button`.
pub fn plain(theme: Theme) -> impl Fn(&IcedTheme, Status) -> Style {
    move |_iced_theme: &IcedTheme, status: Status| {
        let background = if matches!(status, Status::Hovered) { Some(Background::Color(theme.neutral_bg())) } else { None };
        Style { background, text_color: theme.neutral_front(), border: Border { radius: borders::RADIUS_SM.into(), ..Border::default() }, shadow: iced::Shadow::default() }
    }
}

/// Outline button in the theme's primary color -- for a row-level
/// "this is the point of this row" action (a table row's "Switch to"/
/// "Record Payment") that shouldn't compete visually with a page-level
/// primary button. Transparent at rest with a colored 1px border;
/// hover adds a light primary-tinted fill via
/// [`states::OUTLINE_BUTTON_HOVER_ALPHA`]. From `data_table::
/// primary_row_button`, the only place this treatment existed.
pub fn outline_primary(theme: Theme) -> impl Fn(&IcedTheme, Status) -> Style {
    move |_iced_theme: &IcedTheme, status: Status| {
        let background = match status {
            Status::Hovered | Status::Pressed => Color { a: states::OUTLINE_BUTTON_HOVER_ALPHA, ..theme.primary() },
            _ => Color::TRANSPARENT,
        };
        Style {
            background: Some(Background::Color(background)),
            text_color: theme.primary(),
            border: Border { color: theme.primary(), width: 1.0, radius: borders::RADIUS_SM.into() },
            shadow: iced::Shadow::default(),
        }
    }
}

/// The sidebar's primary-nav-item treatment: a standing tinted pill
/// while `active` (via [`states::SELECTED_ALPHA`], not hover-driven --
/// it persists), a barely-there tint on hover while inactive (via
/// [`states::GHOST_ROW_HOVER_ALPHA`]), transparent otherwise. `active`
/// is baked into the returned closure (not read from `Status`) because
/// "is this the current page" isn't something iced's button state
/// machine knows about. From `sidebar::nav_row`'s inline closure, the
/// only place this three-way treatment existed.
pub fn nav_item(theme: Theme, active: bool) -> impl Fn(&IcedTheme, Status) -> Style {
    move |_iced_theme: &IcedTheme, status: Status| {
        let background = if active {
            Color { a: states::SELECTED_ALPHA, ..theme.primary() }
        } else if matches!(status, Status::Hovered) {
            Color { a: states::GHOST_ROW_HOVER_ALPHA, ..theme.neutral_front() }
        } else {
            Color::TRANSPARENT
        };
        let text_color = if active { theme.primary() } else { theme.neutral_front() };
        Style { background: Some(Background::Color(background)), text_color, border: Border { radius: borders::RADIUS_MD.into(), ..Border::default() }, shadow: iced::Shadow::default() }
    }
}

/// The sidebar's own profile card, rendered as a button so it gets
/// hover feedback. Opaque `neutral_bg` at rest -- hovering blends it to
/// [`states::NEUTRAL_CONTROL_HOVER_ALPHA`], which (unlike
/// [`secondary`]'s `theme.surface()` rest state) makes it *lighter* on
/// hover rather than darker, since the same alpha is blending over an
/// already-opaque neutral background instead of the page surface. From
/// `sidebar::profile_card_style`, the only place this treatment
/// existed.
pub fn profile_card(theme: Theme) -> impl Fn(&IcedTheme, Status) -> Style {
    move |_iced_theme: &IcedTheme, status: Status| {
        let background = if matches!(status, Status::Hovered) { Color { a: states::NEUTRAL_CONTROL_HOVER_ALPHA, ..theme.neutral_bg() } } else { theme.neutral_bg() };
        Style { background: Some(Background::Color(background)), text_color: theme.text_primary(), border: Border { radius: borders::RADIUS_MD.into(), ..Border::default() }, shadow: iced::Shadow::default() }
    }
}
