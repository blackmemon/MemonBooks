use iced::widget::{button, checkbox, column, container, progress_bar, row, scrollable, text, Space};
use iced::{Alignment, Background, Border, Element, Font, Length, Theme as IcedTheme};

use crate::frontend::styles::badges::{tone_colors, BadgeTone};
use crate::frontend::styles::colors::Theme;
use crate::frontend::styles::typography;
use crate::frontend::styles::fonts;
use crate::frontend::styles::{icons, panels, spacing, states, table as table_style};
use crate::frontend::styles::button as button_style;
use crate::frontend::styles::checkbox as checkbox_style;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortDirection {
    None,
    Ascending,
    Descending,
}

pub struct ColumnDef<'a> {
    pub label: &'a str,
    pub width: Length,
    pub sort: SortDirection,
}

impl<'a> ColumnDef<'a> {
    pub fn new(label: &'a str, width: Length) -> Self {
        Self { label, width, sort: SortDirection::None }
    }

    pub fn sorted(mut self, sort: SortDirection) -> Self {
        self.sort = sort;
        self
    }
}

pub enum Cell {
    Text(String),
    Date(String),
    Person { name: String, subtitle: String },
    Status { label: String, tone: BadgeTone },
    Progress { fraction: f32, label: String },
}

impl Cell {
    pub fn text(value: impl Into<String>) -> Self {
        Cell::Text(value.into())
    }
    pub fn date(value: impl Into<String>) -> Self {
        Cell::Date(value.into())
    }
    pub fn person(name: impl Into<String>, subtitle: impl Into<String>) -> Self {
        Cell::Person { name: name.into(), subtitle: subtitle.into() }
    }
    pub fn status(label: impl Into<String>, tone: BadgeTone) -> Self {
        Cell::Status { label: label.into(), tone }
    }
    pub fn progress(fraction: f32, label: impl Into<String>) -> Self {
        Cell::Progress { fraction, label: label.into() }
    }
}

pub struct Row {
    pub cells: Vec<Cell>,
    pub primary_action_label: Option<String>,
    pub highlighted: bool,
}

impl Row {
    pub fn new(cells: Vec<Cell>) -> Self {
        Self { cells, primary_action_label: None, highlighted: false }
    }

    /// Gives this row a visible primary-action button (e.g. "Switch to",
    /// "Depreciate", "Record Payment") -- for the action that *is* the
    /// row's main purpose, as opposed to secondary actions (edit, delete,
    /// view details) which stay behind the kebab menu. Rows within the
    /// same table may mix "has a primary action" and "doesn't" freely;
    /// rows without one just reserve the same width as blank space, so
    /// columns stay aligned.
    pub fn with_primary_action(mut self, label: impl Into<String>) -> Self {
        self.primary_action_label = Some(label.into());
        self
    }

    /// Keeps this row's background persistently highlighted (e.g. the
    /// row the caller is currently editing) rather than only tinting on
    /// hover.
    pub fn highlighted(mut self, highlighted: bool) -> Self {
        self.highlighted = highlighted;
        self
    }
}

const PRIMARY_ACTION_WIDTH: f32 = 132.0;

/// `on_row_action(row_index)` fires from either the row itself or its
/// trailing kebab (⋮) button -- callers decide what that opens; this is
/// for secondary actions (edit, delete, view details). `on_primary_action`
/// fires from a row's visible primary-action button, when
/// `Row::with_primary_action` gave it one -- for the action that is the
/// row's main purpose. Pass `None` for `selected` to omit the leading
/// checkbox column (and the header's "select all" checkbox) entirely;
/// `on_select_all(true)` fires when every row is checked and the header
/// checkbox is unchecked (select all currently-visible rows), and
/// `on_select_all(false)` fires when it's checked (clear the selection).
#[allow(clippy::too_many_arguments)]
pub fn view<'a, Message: Clone + 'a>(
    theme: Theme,
    columns: Vec<ColumnDef<'a>>,
    rows: Vec<Row>,
    selected: Option<Vec<bool>>,
    on_row_select: impl Fn(usize, bool) -> Message + Clone + 'a,
    on_select_all: impl Fn(bool) -> Message + 'a,
    on_row_action: impl Fn(usize) -> Message + Clone + 'a,
    on_primary_action: impl Fn(usize) -> Message + Clone + 'a,
) -> Element<'a, Message> {
    view_with_column_visibility(theme, columns, &[], rows, selected, on_row_select, on_select_all, on_row_action, on_primary_action)
}

/// Same as [`view`], except `visible_columns[i]` (columns past the end
/// of a shorter slice default to visible) controls whether column `i`'s
/// header cell and every row's cell for it are rendered at all -- the
/// live-apply side of a "Columns" show/hide toggle. Lives here (rather
/// than on any one view) so any list view can wire a column toggle up
/// against the same table without reimplementing the filtering.
#[allow(clippy::too_many_arguments)]
pub fn view_with_column_visibility<'a, Message: Clone + 'a>(
    theme: Theme,
    columns: Vec<ColumnDef<'a>>,
    visible_columns: &[bool],
    rows: Vec<Row>,
    selected: Option<Vec<bool>>,
    on_row_select: impl Fn(usize, bool) -> Message + Clone + 'a,
    on_select_all: impl Fn(bool) -> Message + 'a,
    on_row_action: impl Fn(usize) -> Message + Clone + 'a,
    on_primary_action: impl Fn(usize) -> Message + Clone + 'a,
) -> Element<'a, Message> {
    let header_container = build_header(theme, &columns, visible_columns, &selected, on_select_all, false);
    let body = build_body(theme, &columns, visible_columns, rows, selected, on_row_select, on_row_action, on_primary_action, false);

    let card = column![header_container, body].width(Length::Fill);

    // Same radius token as `side_panel`'s card (RADIUS_LG), so the table
    // and an open side panel show matching rounded corners -- applied
    // unconditionally here since this component has no notion of
    // whether a panel is open next to it.
    //
    // Uses the shadowed `panels::card` (not `flat_card`): this is a
    // top-level panel sitting directly on the page background, not
    // nested inside an already-shadowed ancestor, so the shadow reads
    // as the card lifting off the page. Confirmed with the user in
    // Batch 12 rather than applied silently, since it's the one
    // visible change in the whole consistency-audit pass.
    container(card).style(panels::card(theme, states::RADIUS_LG)).width(Length::Fill).into()
}

/// Same as [`view_with_column_visibility`], except the header row stays
/// fixed in place while the rows beneath it scroll independently -- for
/// tables with enough rows that keeping the column headers on screen
/// while scrolling matters.
#[allow(clippy::too_many_arguments)]
pub fn view_with_frozen_header<'a, Message: Clone + 'a>(
    theme: Theme,
    columns: Vec<ColumnDef<'a>>,
    visible_columns: &[bool],
    rows: Vec<Row>,
    selected: Option<Vec<bool>>,
    on_row_select: impl Fn(usize, bool) -> Message + Clone + 'a,
    on_select_all: impl Fn(bool) -> Message + 'a,
    on_row_action: impl Fn(usize) -> Message + Clone + 'a,
    on_primary_action: impl Fn(usize) -> Message + Clone + 'a,
) -> Element<'a, Message> {
    let header_container = build_header(theme, &columns, visible_columns, &selected, on_select_all, true);
    let body = build_body(theme, &columns, visible_columns, rows, selected, on_row_select, on_row_action, on_primary_action, true);
    let header_divider = container(Space::with_height(Length::Fixed(1.0))).width(Length::Fill).style(move |_iced_theme: &IcedTheme| container::Style {
        background: Some(Background::Color(table_style::grid_line_color(theme))),
        ..container::Style::default()
    });

    let card = column![header_container, header_divider, scrollable(body).height(Length::Fill)].width(Length::Fill).height(Length::Fill);

    container(card).style(panels::card(theme, states::RADIUS_LG)).width(Length::Fill).height(Length::Fill).into()
}

/// The pixel width a [`view_with_frozen_header`] table renders at when
/// every visible column has a [`Length::Fixed`] width -- lets a caller
/// wrap the table in a container of exactly this width before putting
/// it in a horizontally-scrolling container, so the table keeps this
/// size instead of its columns compressing to fit whatever space
/// happens to be available. Columns with a non-fixed width (e.g.
/// `FillPortion`) don't contribute a measurable amount and are
/// effectively counted as zero-width.
pub fn frozen_header_row_width(columns: &[ColumnDef], visible_columns: &[bool], has_selection: bool) -> f32 {
    let is_visible = |index: usize| visible_columns.get(index).copied().unwrap_or(true);
    let mut segments = 0usize;
    let mut total = 2.0 * spacing::MD;
    if has_selection {
        total += 24.0;
        segments += 1;
    }
    for (index, column_def) in columns.iter().enumerate() {
        if !is_visible(index) {
            continue;
        }
        if let Length::Fixed(width) = column_def.width {
            total += width;
        }
        segments += 1;
    }
    total += PRIMARY_ACTION_WIDTH + 32.0;
    segments += 2;
    total + segments.saturating_sub(1) as f32 * spacing::MD
}

// Must match the outer card's own radius (`states::RADIUS_LG`) -- see
// `table_style::header_row`'s doc comment for why a mismatch (or an
// unrounded header) visually flattens the card's top corners.
fn build_header<'a, Message: Clone + 'a>(theme: Theme, columns: &[ColumnDef<'a>], visible_columns: &[bool], selected: &Option<Vec<bool>>, on_select_all: impl Fn(bool) -> Message + 'a, bordered: bool) -> Element<'a, Message> {
    let is_visible = |index: usize| visible_columns.get(index).copied().unwrap_or(true);
    let mut header = row![].spacing(spacing::MD).padding(iced::Padding { top: spacing::SM, right: spacing::MD, bottom: spacing::SM, left: spacing::MD }).align_y(Alignment::Center);
    if let Some(selected) = selected {
        let all_selected = !selected.is_empty() && selected.iter().all(|&value| value);
        header = header.push(container(checkbox("", all_selected).style(checkbox_style::style(theme)).on_toggle(on_select_all)).width(Length::Fixed(24.0)));
    }
    let mut is_first_column = true;
    for (index, column_def) in columns.iter().enumerate() {
        if !is_visible(index) {
            continue;
        }
        if bordered && !is_first_column {
            header = header.push(column_divider(theme));
        }
        is_first_column = false;
        let arrow = match column_def.sort {
            SortDirection::Ascending => format!(" {}", icons::SORT_ASCENDING),
            SortDirection::Descending => format!(" {}", icons::SORT_DESCENDING),
            SortDirection::None => String::new(),
        };
        let label = text(format!("{}{}", column_def.label, arrow)).size(typography::body()).color(theme.neutral_front());
        header = header.push(label.width(column_def.width));
    }
    header = header.push(Space::with_width(Length::Fixed(PRIMARY_ACTION_WIDTH)));
    header = header.push(Space::with_width(Length::Fixed(32.0)));

    // See the matching comment in `build_body`: a pushed `column_divider`
    // reports `Length::Fill` height so the grid line stretches to the
    // row's cross-axis size, which would otherwise make this row (and
    // the `container` wrapping it) report `Fill` too and compete with
    // the scrollable body for vertical space in the enclosing column.
    header = header.height(Length::Shrink);

    container(header).style(table_style::header_row(theme, states::RADIUS_LG)).into()
}

/// A 1px vertical grid line dropped between two column cells in a
/// `bordered` table -- mirrors the horizontal divider `build_body`
/// draws between rows, so the grid reads as lines between cells rather
/// than a border boxing in each cell individually.
fn column_divider<'a, Message: 'a>(theme: Theme) -> Element<'a, Message> {
    container(Space::with_width(Length::Fixed(1.0)))
        .height(Length::Fill)
        .style(move |_iced_theme: &IcedTheme| container::Style { background: Some(Background::Color(table_style::grid_line_color(theme))), ..container::Style::default() })
        .into()
}

#[allow(clippy::too_many_arguments)]
fn build_body<'a, Message: Clone + 'a>(
    theme: Theme,
    columns: &[ColumnDef<'a>],
    visible_columns: &[bool],
    rows: Vec<Row>,
    selected: Option<Vec<bool>>,
    on_row_select: impl Fn(usize, bool) -> Message + Clone + 'a,
    on_row_action: impl Fn(usize) -> Message + Clone + 'a,
    on_primary_action: impl Fn(usize) -> Message + Clone + 'a,
    bordered: bool,
) -> Element<'a, Message> {
    let show_checkbox = selected.is_some();
    let is_visible = |index: usize| visible_columns.get(index).copied().unwrap_or(true);
    let mut body = column![];
    let row_count = rows.len();
    for (row_index, table_row) in rows.into_iter().enumerate() {
        let highlighted = table_row.highlighted;
        let mut cells_row = row![].spacing(spacing::MD).padding([spacing::SM, spacing::MD]).align_y(Alignment::Center);
        if show_checkbox {
            let is_checked = selected.as_ref().and_then(|s| s.get(row_index)).copied().unwrap_or(false);
            let select_this_row = on_row_select.clone();
            cells_row = cells_row.push(container(checkbox("", is_checked).style(checkbox_style::style(theme)).on_toggle(move |value| select_this_row(row_index, value))).width(Length::Fixed(24.0)));
        }
        let mut is_first_column = true;
        for (index, (cell, column_def)) in table_row.cells.into_iter().zip(columns.iter()).enumerate() {
            if !is_visible(index) {
                continue;
            }
            if bordered && !is_first_column {
                cells_row = cells_row.push(column_divider(theme));
            }
            is_first_column = false;
            cells_row = cells_row.push(container(cell_view::<Message>(theme, cell)).width(column_def.width));
        }
        let primary_action_slot: Element<'a, Message> = match table_row.primary_action_label {
            Some(label) => button(text(label).size(typography::body()))
                .style(button_style::outline_primary(theme))
                .padding([spacing::SPACE_6, spacing::SPACE_14])
                .on_press(on_primary_action(row_index))
                .into(),
            None => Space::with_width(Length::Shrink).into(),
        };
        cells_row = cells_row.push(container(primary_action_slot).width(Length::Fixed(PRIMARY_ACTION_WIDTH)).align_x(iced::alignment::Horizontal::Left));
        cells_row = cells_row.push(
            container(
                button(text(icons::KEBAB).size(typography::emphasis()).color(theme.neutral_front()))
                    .style(button_style::plain(theme))
                    .on_press(on_row_action(row_index)),
            )
            .width(Length::Fixed(32.0)),
        );

        // `column_divider`'s `Length::Fill` height (needed so the grid
        // line stretches to the row's actual cross-axis size during
        // layout) would otherwise propagate through `Row::push`'s
        // enclose logic and make this whole row -- and in turn the
        // `column![]` body feeding `scrollable` -- report a `Fill`
        // height too, which trips `scrollable`'s "must not fill its
        // vertical scrolling axis" assertion. Pin the row back to
        // `Shrink` explicitly now that every child (dividers included)
        // has been pushed.
        cells_row = cells_row.height(Length::Shrink);
        let interactive_row = button(cells_row).style(table_style::row(theme, highlighted)).on_press(on_row_action(row_index)).width(Length::Fill);

        body = body.push(interactive_row);
        if row_index + 1 < row_count {
            let line_color = if bordered { table_style::grid_line_color(theme) } else { table_style::divider_color(theme) };
            body = body.push(container(Space::with_height(Length::Fixed(1.0))).width(Length::Fill).style(move |_iced_theme: &IcedTheme| container::Style {
                background: Some(Background::Color(line_color)),
                ..container::Style::default()
            }));
        }
    }

    body.into()
}

fn cell_view<'a, Message: 'a>(theme: Theme, cell: Cell) -> Element<'a, Message> {
    match cell {
        Cell::Text(value) => text(value).size(typography::label()).color(theme.text_primary()).into(),
        Cell::Date(value) => text(value).size(typography::label()).color(theme.neutral_front()).into(),
        Cell::Person { name, subtitle } => {
            let initial = name.chars().next().unwrap_or('?').to_uppercase().to_string();
            let avatar = container(text(initial).size(typography::body()).color(theme.accent_front()).font(Font { weight: iced::font::Weight::Bold, ..Font::DEFAULT }))
                .width(Length::Fixed(28.0))
                .height(Length::Fixed(28.0))
                .align_x(iced::alignment::Horizontal::Center)
                .align_y(iced::alignment::Vertical::Center)
                .style(move |_iced_theme: &IcedTheme| container::Style {
                    background: Some(Background::Color(theme.primary())),
                    border: Border { radius: states::RADIUS_SM.into(), ..Border::default() },
                    ..container::Style::default()
                });
            row![avatar, column![text(name).size(typography::label()).color(theme.text_primary()), text(subtitle).size(typography::caption()).color(theme.neutral_front())].spacing(spacing::SPACE_2)]
                .spacing(spacing::SM)
                .align_y(Alignment::Center)
                .into()
        }
        Cell::Status { label, tone } => {
            let (bg, front) = tone_colors(theme, tone);
            container(row![text(icons::STATUS_DOT).size(typography::micro()).color(front), text(label).size(typography::body()).color(front)].spacing(spacing::SPACE_6).align_y(Alignment::Center))
                .padding([spacing::XS, spacing::SPACE_10])
                .style(move |_iced_theme: &IcedTheme| container::Style {
                    background: Some(Background::Color(bg)),
                    border: Border { radius: states::RADIUS_PILL.into(), ..Border::default() },
                    ..container::Style::default()
                })
                .into()
        }
        Cell::Progress { fraction, label } => column![
            progress_bar(0.0..=1.0, fraction).height(Length::Fixed(6.0)).style(move |_iced_theme: &IcedTheme| iced::widget::progress_bar::Style {
                background: Background::Color(theme.neutral_bg()),
                bar: Background::Color(theme.primary()),
                border: Border::default(),
            }),
            text(label).size(typography::caption()).color(theme.neutral_front()),
        ]
        .spacing(spacing::XS)
        .into(),
    }
}

/// The show/hide checklist content opened by [`columns_button`] --
pub fn columns_button<'a, Message: Clone + 'a>(theme: Theme, on_toggle: Message) -> Element<'a, Message> {
    button(text("Columns").size(typography::label()).font(fonts::BOLD)).padding([spacing::SM, spacing::SPACE_14]).style(button_style::neutral(theme)).on_press(on_toggle).into()
}

pub fn columns_panel<'a, Message: Clone + 'a>(
    theme: Theme,
    labels: &'a [&'a str],
    visible: &[bool],
    on_toggle: impl Fn(usize) -> Message + Clone + 'a,
    on_select_all: Message,
    on_deselect_all: Message,
    on_reset: Message,
) -> Element<'a, Message> {
    let select_all = button(text("Select All").size(typography::body())).padding([spacing::SM, spacing::SPACE_14]).style(button_style::primary(theme)).on_press(on_select_all);
    let deselect_all = button(text("Deselect All").size(typography::body())).padding([spacing::SM, spacing::SPACE_14]).style(button_style::secondary(theme)).on_press(on_deselect_all);

    let mut list = column![].spacing(spacing::SPACE_10).padding(spacing::SM);
    for (index, label) in labels.iter().enumerate() {
        let is_visible = visible.get(index).copied().unwrap_or(true);
        let toggle_this = on_toggle.clone();
        let option = checkbox(*label, is_visible)
            .style(checkbox_style::style(theme))
            .on_toggle(move |_| toggle_this(index))
            .size(16)
            .text_size(typography::label());
        list = list.push(
            container(option)
                .width(Length::Fill)
                .padding([spacing::SM, spacing::MD])
                .style(move |_iced_theme| container::Style {
                    border: Border {
                        width: states::STYLES_BORDER_WIDTH_ALPHA,
                        color: theme.neutral_bg(),
                        radius: states::RADIUS_SM.into(),
                    },
                    ..container::Style::default()
                }),
        );
    }

    let reset = button(text("Reset to Default").size(typography::label())).padding([spacing::SPACE_10, spacing::MD]).width(Length::Fill).style(button_style::secondary(theme)).on_press(on_reset);
    column![
        text("Select Columns to Display").size(typography::panel_title()).color(theme.text_primary()),
        row![select_all, deselect_all].spacing(spacing::SM),
        scrollable(list.width(Length::Fill))
            .height(Length::Fill)
            .style(move |iced_theme, status| {
                let mut style = iced::widget::scrollable::default(iced_theme, status);
                style.container.border = Border {
                    width: states::STYLES_BORDER_WIDTH_ALPHA,
                    color: theme.neutral_bg(),
                    radius: states::RADIUS_SM.into(),
                };
                style
            }),
        reset,
    ]
    .spacing(spacing::MD)
    .height(Length::Fill)
    .into()
}


#[cfg(test)]
mod tests {
    use super::*;

    /// Building the widget tree exercises every iced widget constructor
    /// this module calls -- including their `debug_assert!`s (e.g.
    /// `Scrollable::validate` panicking on a `Fill`-width child on the
    /// scrolling axis). `update()`-only tests never call `view` at all,
    /// so a bad `Length` combination here can reach a real running app
    /// with every other check green; this catches that class of bug
    /// without needing a window. `()` as the message type: nothing here
    /// inspects what's produced, only that construction doesn't panic.
    #[test]
    fn view_builds_without_panicking() {
        let columns = vec![ColumnDef::new("A", Length::FillPortion(1)).sorted(SortDirection::Ascending), ColumnDef::new("B", Length::FillPortion(2))];
        let rows = vec![
            Row::new(vec![Cell::text("1"), Cell::status("Active", BadgeTone::Positive)]).with_primary_action("Go"),
            Row::new(vec![Cell::date("2026-01-01"), Cell::person("Name", "Subtitle")]),
        ];

        let _: Element<'_, ()> = view(Theme::Light, columns, rows, Some(vec![false, true]), |_, _| (), |_| (), |_| (), |_| ());
    }

    #[test]
    fn view_with_column_visibility_builds_without_panicking_including_hidden_columns() {
        let columns = vec![ColumnDef::new("A", Length::FillPortion(1)), ColumnDef::new("B", Length::FillPortion(1)), ColumnDef::new("C", Length::FillPortion(1))];
        let rows = vec![Row::new(vec![Cell::text("1"), Cell::text("2"), Cell::text("3")])];

        let _: Element<'_, ()> = view_with_column_visibility(Theme::Dark, columns, &[true, false, true], rows, None, |_, _| (), |_| (), |_| (), |_| ());
    }

    #[test]
    fn view_with_frozen_header_builds_without_panicking() {
        let columns = vec![ColumnDef::new("A", Length::FillPortion(1)).sorted(SortDirection::Ascending), ColumnDef::new("B", Length::FillPortion(2))];
        let rows = vec![
            Row::new(vec![Cell::text("1"), Cell::status("Active", BadgeTone::Positive)]).with_primary_action("Go"),
            Row::new(vec![Cell::date("2026-01-01"), Cell::person("Name", "Subtitle")]),
        ];

        let _: Element<'_, ()> = view_with_frozen_header(Theme::Light, columns, &[], rows, Some(vec![false, true]), |_, _| (), |_| (), |_| (), |_| ());
    }

    #[test]
    fn columns_panel_builds_without_panicking() {
        let labels = ["Code", "Name", "Type"];
        let _: Element<'_, ()> = columns_panel(Theme::Light, &labels, &[true, false, true], |_| (), (), (), ());
    }
}
