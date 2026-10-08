//! Expandable tree row styling (Section 6): shares the data table's
//! header background and hover-highlight behavior (`table.rs`) so a
//! tree and a table sitting in the same view read as one family, but
//! adds a folder tint for parent (has-children) rows -- the "distinct
//! styling for parent vs. leaf rows" a tree needs that a flat table
//! never did.

use iced::widget::button;
use iced::{Background, Border, Color, Theme as IcedTheme};

use crate::frontend::components::colors::Theme;
use crate::frontend::components::states;

/// Indent added per depth level, in pixels -- large enough to make
/// nesting obvious at a glance, small enough that deeply-nested
/// accounts (5-6 levels) don't run out of row width.
pub const INDENT_STEP: f32 = 18.0;

/// Fixed width reserved for the toggle glyph/blank spacer, so every
/// row's code/name column lines up regardless of whether that row has
/// a toggle or not.
pub const TOGGLE_WIDTH: f32 = 20.0;

/// Fixed width reserved for the account code, before the name.
pub const CODE_WIDTH: f32 = 84.0;

/// Fixed width reserved for the trailing balance, right-aligned.
pub const BALANCE_WIDTH: f32 = 120.0;

/// A tree row rendered as a `button` (same trick as `table::row`, for
/// free hover feedback) -- `has_children` rows (folders) get a faint
/// tint even at rest, so the eye can separate group headers from leaf
/// accounts before ever reading the toggle glyph.
pub fn row(
    theme: Theme,
    has_children: bool,
) -> impl Fn(&IcedTheme, button::Status) -> button::Style {
    move |_iced_theme: &IcedTheme, status: button::Status| {
        let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
        let background = match (has_children, hovered) {
            (_, true) => Some(Background::Color(Color {
                a: states::TABLE_ROW_HOVER_ALPHA,
                ..theme.neutral_bg()
            })),
            (true, false) => Some(Background::Color(Color {
                a: states::TINTED_PANEL_ALPHA,
                ..theme.primary()
            })),
            (false, false) => None,
        };
        button::Style {
            background,
            text_color: theme.text_primary(),
            border: Border::default(),
            shadow: iced::Shadow::default(),
        }
    }
}

pub fn divider_color(theme: Theme) -> Color {
    theme.neutral_bg()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parent_rows_are_tinted_even_at_rest_but_leaf_rows_are_not() {
        let parent_style = row(Theme::Light, true)(&IcedTheme::Light, button::Status::Active);
        let leaf_style = row(Theme::Light, false)(&IcedTheme::Light, button::Status::Active);

        assert!(parent_style.background.is_some());
        assert!(leaf_style.background.is_none());
    }

    #[test]
    fn hover_wins_over_the_resting_folder_tint() {
        let hovered = row(Theme::Light, true)(&IcedTheme::Light, button::Status::Hovered);
        let Some(Background::Color(color)) = hovered.background else {
            panic!("expected a background color")
        };
        assert_eq!(color.a, states::TABLE_ROW_HOVER_ALPHA);
    }
}
