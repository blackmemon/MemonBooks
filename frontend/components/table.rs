//! Data table row styling (Section 6): a header background, a row that
//! highlights subtly on hover, and the hairline divider drawn between
//! rows instead of full alternating stripes.

use iced::alignment::Horizontal;
use iced::widget::{button, column, container, text, Row, Space};
use iced::{Alignment, Background, Border, Color, Element, Length, Theme as IcedTheme};

use crate::frontend::components::button as button_style;
use crate::frontend::components::colors::Theme;
use crate::frontend::components::{fonts, icons, shadows, spacing, typography};

/// A compact read-only table: bold caption headers over a rule, the body
/// rows, then an optional bold totals row. `widths` are the columns'
/// fill portions; the columns listed in `right` are right-aligned.
pub fn grid<'a, M: 'a>(
    theme: Theme,
    labels: &[&str],
    widths: &[u16],
    right: &[usize],
    rows: Vec<Vec<String>>,
    totals: Option<Vec<String>>,
) -> Element<'a, M> {
    let mut body = column![grid_header(theme, labels, widths, right)].spacing(spacing::SPACE_6);
    for cells in rows {
        body = body.push(grid_row(theme, cells, widths, right, false));
    }
    if let Some(cells) = totals {
        body = body.push(grid_row(theme, cells, widths, right, true));
    }
    body.into()
}

/// One cell of a [`grid`] column, right-aligned when listed in `right`.
fn grid_cell<'a, M: 'a>(
    content: impl Into<Element<'a, M>>,
    index: usize,
    widths: &[u16],
    right: &[usize],
) -> Element<'a, M> {
    let cell = container(content).width(Length::FillPortion(widths[index]));
    if right.contains(&index) {
        cell.align_x(Horizontal::Right).into()
    } else {
        cell.into()
    }
}

fn grid_header<'a, M: 'a>(
    theme: Theme,
    labels: &[&str],
    widths: &[u16],
    right: &[usize],
) -> Element<'a, M> {
    let mut line = Row::new().spacing(spacing::SM);
    for (index, label) in labels.iter().enumerate() {
        line = line.push(grid_cell(
            text(label.to_string())
                .size(typography::caption())
                .font(fonts::BOLD)
                .color(theme.neutral_front()),
            index,
            widths,
            right,
        ));
    }
    column![
        line,
        container(Space::with_height(Length::Fixed(spacing::SPACE_1)))
            .width(Length::Fill)
            .style(rule(theme)),
    ]
    .spacing(spacing::SPACE_4)
    .into()
}

fn grid_row<'a, M: 'a>(
    theme: Theme,
    cells: Vec<String>,
    widths: &[u16],
    right: &[usize],
    bold: bool,
) -> Element<'a, M> {
    let mut line = Row::new().spacing(spacing::SM).align_y(Alignment::Center);
    for (index, cell) in cells.into_iter().enumerate() {
        let mut label = text(cell)
            .size(typography::body())
            .color(theme.neutral_font());
        if bold {
            label = label.font(fonts::BOLD);
        }
        line = line.push(grid_cell(label, index, widths, right));
    }
    line.into()
}

/// A clickable column label for a wide table's header row, followed by
/// the sort arrow when the column is sorted (`Some(ascending)`).
pub fn sort_header<'a, M: Clone + 'a>(
    theme: Theme,
    label: &'a str,
    sorted: Option<bool>,
    width: Length,
    on_press: M,
) -> Element<'a, M> {
    let mut cell = Row::new()
        .push(
            text(label)
                .size(typography::label() + 1.0)
                .font(fonts::BOLD)
                .color(theme.neutral_front()),
        )
        .spacing(spacing::SPACE_4)
        .align_y(Alignment::Center);
    if let Some(ascending) = sorted {
        cell = cell.push(
            icons::icon(if ascending {
                icons::SORT_ASCENDING
            } else {
                icons::SORT_DESCENDING
            })
            .size(typography::micro())
            .color(theme.neutral_front()),
        );
    }
    button(cell)
        .padding(0)
        .width(width)
        .style(button_style::bare(theme))
        .on_press(on_press)
        .into()
}

/// The [`divider`] hairline itself, spanning the width.
pub fn divider_line<'a, M: 'a>(theme: Theme) -> Element<'a, M> {
    container(Space::with_height(Length::Fixed(spacing::SPACE_1)))
        .width(Length::Fill)
        .style(divider(theme))
        .into()
}

/// The 1px rule under a [`grid`]'s headers: the regular border color.
pub fn rule(theme: Theme) -> impl Fn(&IcedTheme) -> container::Style {
    move |_iced_theme: &IcedTheme| container::Style {
        background: Some(Background::Color(theme.neutral_border())),
        ..container::Style::default()
    }
}

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
