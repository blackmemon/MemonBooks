//! Data table row styling (Section 6): a header background, a row that
//! highlights subtly on hover, and the hairline divider drawn between
//! rows instead of full alternating stripes.

use iced::widget::{button, container};
use iced::{Background, Border, Color, Theme as IcedTheme};

use crate::frontend::styles::colors::Theme;
use crate::frontend::styles::states;

/// `radius` must match the enclosing card's own corner radius (e.g.
/// `borders::RADIUS_LG`, as `data_table` passes) -- the header sits flush
/// against the card's top edge with no padding gap, and `container`
/// doesn't clip children to a parent's rounded shape (it only clips to a
/// plain rectangle, when `.clip(true)` is set at all), so an unrounded
/// header background paints squarely over the card's rounded top-left/
/// top-right corners, visually flattening them even though the card's
/// own border+radius are still being drawn underneath. Only the top
/// corners round -- the header never reaches the card's bottom edge, so
/// bottom-left/bottom-right stay square (irrelevant, since the header
/// doesn't extend there).
pub fn header_row(theme: Theme, radius: f32) -> impl Fn(&IcedTheme) -> container::Style {
    move |_iced_theme: &IcedTheme| container::Style {
        background: Some(Background::Color(theme.neutral_bg())),
        border: Border { radius: iced::border::top(radius), ..Border::default() },
        ..container::Style::default()
    }
}

/// A row rendered as a `button` so it gets hover feedback for free from
/// iced's `button::Status` -- pressing it opens that row's action menu,
/// same as its trailing kebab button.
pub fn row(theme: Theme) -> impl Fn(&IcedTheme, button::Status) -> button::Style {
    move |_iced_theme: &IcedTheme, status: button::Status| {
        let background = match status {
            button::Status::Hovered | button::Status::Pressed => Some(Background::Color(Color { a: states::TABLE_ROW_HOVER_ALPHA, ..theme.neutral_bg() })),
            _ => None,
        };
        button::Style { background, text_color: theme.text_primary(), border: Border::default(), shadow: iced::Shadow::default() }
    }
}

pub fn divider_color(theme: Theme) -> Color {
    theme.neutral_bg()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Regression guard for the header-overwrites-the-card's-rounded-
    /// corners bug: an unrounded (or mismatched-radius) header background
    /// paints squarely over the enclosing card's top corners since
    /// `container` never clips children to a parent's rounded shape.
    /// Asserting on the produced `Style` directly (rather than only a
    /// widget-construction test) is what actually catches a wrong or
    /// missing radius here -- construction alone doesn't inspect style
    /// values.
    #[test]
    fn header_row_rounds_only_its_top_corners_to_match_the_given_radius() {
        let style = header_row(Theme::Light, 14.0)(&IcedTheme::Light);

        assert_eq!(style.border.radius.top_left, 14.0);
        assert_eq!(style.border.radius.top_right, 14.0);
        assert_eq!(style.border.radius.bottom_left, 0.0);
        assert_eq!(style.border.radius.bottom_right, 0.0);
    }
}
