//! Canonical `pick_list` style (Section 6). Also covers `combobox.rs`:
//! iced has one selection-dropdown widget (`pick_list`), not a
//! separate combobox concept, so there's nothing distinct for that
//! file to hold -- see its own doc comment.
//!
//! Unlike `inputs.rs`, this one *does* have real precedent: three
//! `pick_list`s in the app are already styled (the Maintain section
//! switcher and Transactions document picker in `src/main.rs`, and the
//! Status filter in `toolbar::list_toolbar`) -- every other `pick_list`
//! (32 call sites found by the audit) falls back to iced's own
//! `theme.extended_palette()`, same gap as `text_input`.
//!
//! The two existing implementations (`src/main.rs`'s
//! `maintain_picker_style` and `toolbar.rs`'s `status_picker_style`)
//! were identical in every field except `text_color`. Compared side by
//! side during the Phase 2 consolidation pass and confirmed as a
//! **deliberate semantic split, not accidental drift between two
//! batches**: [`style`] (`theme.text_primary()`, full-strength) is for
//! a picker that sits inline with a page title -- the Maintain section
//! switcher and Transactions document-type picker, both rendered via
//! `page_header::view_with_switcher` right next to the title text.
//! [`muted`] (`theme.neutral_front()`) is for an ordinary secondary
//! toolbar control -- the list-toolbar Status filter today, and any
//! future filter/utility dropdown that isn't title-adjacent. A future
//! cleanup pass finding these two functions differing only in
//! `text_color` should *not* read that as leftover drift to collapse
//! -- it's the intended distinction, kept as two named functions
//! specifically so it survives.

use iced::widget::pick_list::{Status, Style};
use iced::{Background, Color};

use crate::frontend::styles::colors::Theme;
use crate::frontend::styles::{borders, states};

/// For a picker rendered inline with a page title (full-strength
/// `theme.text_primary()`) -- the Maintain section switcher and
/// Transactions document-type picker, both placed right next to the
/// title text via `page_header::view_with_switcher`.
pub fn style(theme: Theme) -> impl Fn(&iced::Theme, Status) -> Style {
    styled(theme, theme.text_primary())
}

/// For an ordinary secondary toolbar control (muted
/// `theme.neutral_front()`) -- the list-toolbar Status filter, and any
/// future filter/utility dropdown that isn't title-adjacent.
pub fn muted(theme: Theme) -> impl Fn(&iced::Theme, Status) -> Style {
    styled(theme, theme.neutral_front())
}

fn styled(theme: Theme, text_color: Color) -> impl Fn(&iced::Theme, Status) -> Style {
    move |_iced_theme: &iced::Theme, status: Status| {
        let background = match status {
            Status::Hovered | Status::Opened => Color { a: states::NEUTRAL_CONTROL_HOVER_ALPHA, ..theme.neutral_bg() },
            Status::Active => theme.surface(),
        };
        Style { text_color, placeholder_color: theme.neutral_front(), handle_color: theme.neutral_front(), background: Background::Color(background), border: borders::card(theme, states::RADIUS_SM) }
    }
}
