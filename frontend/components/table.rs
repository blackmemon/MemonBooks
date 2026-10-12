//! Data table row styling (Section 6): a header background, a row that
//! highlights subtly on hover, and the hairline divider drawn between
//! rows instead of full alternating stripes.

use iced::widget::{button, container};
use iced::{Background, Border, Color, Theme as IcedTheme};

use crate::frontend::components::colors::Theme;
use crate::frontend::components::shadows;

const fn rgb8(r: u8, g: u8, b: u8) -> Color {
    Color {
        r: r as f32 / 255.0,
        g: g as f32 / 255.0,
        b: b as f32 / 255.0,
        a: 1.0,
    }
}

pub fn card(theme: Theme, radius: f32) -> impl Fn(&IcedTheme) -> container::Style {
    move |_iced_theme: &IcedTheme| container::Style {
        background: Some(Background::Color(theme.page_bg())),
        border: Border {
            color: border_color(theme),
            width: 1.0,
            radius: radius.into(),
        },
        shadow: shadows::card(theme),
        ..container::Style::default()
    }
}

pub fn border_color(theme: Theme) -> Color {
    match theme {
        Theme::Light => rgb8(0xD1, 0xD5, 0xDB),
        Theme::Dark => theme.neutral_border(),
    }
}

pub fn header_background(theme: Theme) -> Color {
    match theme {
        Theme::Light => rgb8(0xF8, 0xFA, 0xFC),
        Theme::Dark => theme.surface_hover(),
    }
}

pub fn hover_background(theme: Theme) -> Color {
    match theme {
        Theme::Light => rgb8(0xF0, 0xF9, 0xFF),
        Theme::Dark => theme.primary_tint(0.18),
    }
}

pub fn scrollbar_track(theme: Theme) -> Color {
    match theme {
        Theme::Light => rgb8(0xF8, 0xFA, 0xFC),
        Theme::Dark => theme.surface_hover(),
    }
}

pub fn scrollbar_thumb(theme: Theme) -> Color {
    match theme {
        Theme::Light => rgb8(0x56, 0xAE, 0xD7),
        Theme::Dark => theme.primary_hover(),
    }
}

pub fn scrollbar_thumb_hover(theme: Theme) -> Color {
    match theme {
        Theme::Light => rgb8(0x2C, 0x8F, 0xBD),
        Theme::Dark => theme.primary(),
    }
}

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
        background: Some(Background::Color(header_background(theme))),
        border: Border {
            radius: iced::border::top(radius),
            ..Border::default()
        },
        ..container::Style::default()
    }
}

/// A row rendered as a `button` so it gets hover feedback for free from
/// iced's `button::Status` -- pressing it opens that row's action menu,
/// same as its trailing kebab button. `highlighted` keeps the row's
/// tinted background on (e.g. the row currently being edited) instead
/// of only tinting while the mouse hovers it, mirroring `nav_item`'s
/// `active` flag.
pub fn row(
    theme: Theme,
    highlighted: bool,
) -> impl Fn(&IcedTheme, button::Status) -> button::Style {
    row_with_selection(theme, highlighted, false)
}

pub fn row_with_selection(
    theme: Theme,
    highlighted: bool,
    selected: bool,
) -> impl Fn(&IcedTheme, button::Status) -> button::Style {
    move |_iced_theme: &IcedTheme, status: button::Status| {
        let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
        let background = (highlighted || selected || hovered)
            .then(|| Background::Color(hover_background(theme)));
        button::Style {
            background,
            text_color: theme.text_primary(),
            border: Border::default(),
            shadow: iced::Shadow::default(),
        }
    }
}

pub fn divider_color(theme: Theme) -> Color {
    match theme {
        Theme::Light => rgb8(0xF0, 0xF2, 0xF5),
        Theme::Dark => theme.neutral_bg(),
    }
}

/// The 1px hairline drawn between two plain table rows.
pub fn divider(theme: Theme) -> impl Fn(&IcedTheme) -> container::Style {
    move |_iced_theme: &IcedTheme| container::Style {
        background: Some(Background::Color(divider_color(theme))),
        ..container::Style::default()
    }
}

/// The 1px grid-line color drawn between columns and rows in a bordered
/// table. Used as a thin `Space`'s background on both axes so the grid
/// reads as continuous lines between cells rather than boxing each cell.
pub fn grid_line_color(theme: Theme) -> Color {
    divider_color(theme)
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

    #[test]
    fn light_table_palette_matches_the_reference_colors() {
        assert_eq!(border_color(Theme::Light), rgb8(0xD1, 0xD5, 0xDB));
        assert_eq!(header_background(Theme::Light), rgb8(0xF8, 0xFA, 0xFC));
        assert_eq!(divider_color(Theme::Light), rgb8(0xF0, 0xF2, 0xF5));
        assert_eq!(hover_background(Theme::Light), rgb8(0xF0, 0xF9, 0xFF));
        assert_eq!(scrollbar_track(Theme::Light), rgb8(0xF8, 0xFA, 0xFC));
        assert_eq!(scrollbar_thumb(Theme::Light), rgb8(0x56, 0xAE, 0xD7));
        assert_eq!(scrollbar_thumb_hover(Theme::Light), rgb8(0x2C, 0x8F, 0xBD));
    }
}
