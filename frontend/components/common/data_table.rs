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
}

impl Row {
    pub fn new(cells: Vec<Cell>) -> Self {
        Self { cells, primary_action_label: None }
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
    let show_checkbox = selected.is_some();
    let is_visible = |index: usize| visible_columns.get(index).copied().unwrap_or(true);
    let mut header = row![].spacing(spacing::MD).padding(iced::Padding { top: spacing::SM, right: spacing::MD, bottom: spacing::SM, left: spacing::MD }).align_y(Alignment::Center);
    if let Some(selected) = &selected {
        let all_selected = !selected.is_empty() && selected.iter().all(|&value| value);
        header = header.push(container(checkbox("", all_selected).style(checkbox_style::style(theme)).on_toggle(on_select_all)).width(Length::Fixed(24.0)));
    }
    for (index, column_def) in columns.iter().enumerate() {
        if !is_visible(index) {
            continue;
        }
        let arrow = match column_def.sort {
            SortDirection::Ascending => format!(" {}", icons::SORT_ASCENDING),
            SortDirection::Descending => format!(" {}", icons::SORT_DESCENDING),
            SortDirection::None => String::new(),
        };
        header = header.push(text(format!("{}{}", column_def.label, arrow)).size(typography::BODY).color(theme.neutral_front()).width(column_def.width));
    }
    header = header.push(Space::with_width(Length::Fixed(PRIMARY_ACTION_WIDTH)));
    header = header.push(Space::with_width(Length::Fixed(32.0)));

    // Must match the outer card's own radius below (`borders::RADIUS_LG`)
    // -- see `table_style::header_row`'s doc comment for why a mismatch
    // (or an unrounded header) visually flattens the card's top corners.
    let header_container = container(header).style(table_style::header_row(theme, states::RADIUS_LG));

    let mut body = column![];
    let row_count = rows.len();
    for (row_index, table_row) in rows.into_iter().enumerate() {
        let mut cells_row = row![].spacing(spacing::MD).padding([spacing::SM, spacing::MD]).align_y(Alignment::Center);
        if show_checkbox {
            let is_checked = selected.as_ref().and_then(|s| s.get(row_index)).copied().unwrap_or(false);
            let select_this_row = on_row_select.clone();
            cells_row = cells_row.push(container(checkbox("", is_checked).style(checkbox_style::style(theme)).on_toggle(move |value| select_this_row(row_index, value))).width(Length::Fixed(24.0)));
        }
        for (index, (cell, column_def)) in table_row.cells.into_iter().zip(columns.iter()).enumerate() {
            if !is_visible(index) {
                continue;
            }
            cells_row = cells_row.push(container(cell_view::<Message>(theme, cell)).width(column_def.width));
        }
        let primary_action_slot: Element<'a, Message> = match table_row.primary_action_label {
            Some(label) => button(text(label).size(typography::BODY))
                .style(button_style::outline_primary(theme))
                .padding([spacing::SPACE_6, spacing::SPACE_14])
                .on_press(on_primary_action(row_index))
                .into(),
            None => Space::with_width(Length::Shrink).into(),
        };
        cells_row = cells_row.push(container(primary_action_slot).width(Length::Fixed(PRIMARY_ACTION_WIDTH)).align_x(iced::alignment::Horizontal::Left));
        cells_row = cells_row.push(
            container(
                button(text(icons::KEBAB).size(typography::EMPHASIS).color(theme.neutral_front()))
                    .style(button_style::plain(theme))
                    .on_press(on_row_action(row_index)),
            )
            .width(Length::Fixed(32.0)),
        );

        let interactive_row = button(cells_row).style(table_style::row(theme)).on_press(on_row_action(row_index)).width(Length::Fill);

        body = body.push(interactive_row);
        if row_index + 1 < row_count {
            body = body.push(container(Space::with_height(Length::Fixed(1.0))).width(Length::Fill).style(move |_iced_theme: &IcedTheme| container::Style {
                background: Some(Background::Color(table_style::divider_color(theme))),
                ..container::Style::default()
            }));
        }
    }

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

fn cell_view<'a, Message: 'a>(theme: Theme, cell: Cell) -> Element<'a, Message> {
    match cell {
        Cell::Text(value) => text(value).size(typography::LABEL).color(theme.text_primary()).into(),
        Cell::Date(value) => text(value).size(typography::LABEL).color(theme.neutral_front()).into(),
        Cell::Person { name, subtitle } => {
            let initial = name.chars().next().unwrap_or('?').to_uppercase().to_string();
            let avatar = container(text(initial).size(typography::BODY).color(theme.accent_front()).font(Font { weight: iced::font::Weight::Bold, ..Font::DEFAULT }))
                .width(Length::Fixed(28.0))
                .height(Length::Fixed(28.0))
                .align_x(iced::alignment::Horizontal::Center)
                .align_y(iced::alignment::Vertical::Center)
                .style(move |_iced_theme: &IcedTheme| container::Style {
                    background: Some(Background::Color(theme.primary())),
                    border: Border { radius: states::RADIUS_SM.into(), ..Border::default() },
                    ..container::Style::default()
                });
            row![avatar, column![text(name).size(typography::LABEL).color(theme.text_primary()), text(subtitle).size(typography::CAPTION).color(theme.neutral_front())].spacing(spacing::SPACE_2)]
                .spacing(spacing::SM)
                .align_y(Alignment::Center)
                .into()
        }
        Cell::Status { label, tone } => {
            let (bg, front) = tone_colors(theme, tone);
            container(row![text(icons::STATUS_DOT).size(typography::MICRO).color(front), text(label).size(typography::BODY).color(front)].spacing(spacing::SPACE_6).align_y(Alignment::Center))
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
            text(label).size(typography::CAPTION).color(theme.neutral_front()),
        ]
        .spacing(spacing::XS)
        .into(),
    }
}

/// The show/hide checklist content opened by [`columns_button`] --
pub fn columns_button<'a, Message: Clone + 'a>(theme: Theme, on_toggle: Message) -> Element<'a, Message> {
    button(text("Columns").size(typography::LABEL).font(fonts::BOLD)).padding([spacing::SM, spacing::SPACE_14]).style(button_style::neutral(theme)).on_press(on_toggle).into()
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
    let select_all = button(text("Select All").size(typography::BODY)).padding([spacing::SM, spacing::SPACE_14]).style(button_style::primary(theme)).on_press(on_select_all);
    let deselect_all = button(text("Deselect All").size(typography::BODY)).padding([spacing::SM, spacing::SPACE_14]).style(button_style::secondary(theme)).on_press(on_deselect_all);

    let mut list = column![].spacing(spacing::SPACE_10).padding(spacing::SM);
    for (index, label) in labels.iter().enumerate() {
        let is_visible = visible.get(index).copied().unwrap_or(true);
        let toggle_this = on_toggle.clone();
        let option = checkbox(*label, is_visible)
            .style(checkbox_style::style(theme))
            .on_toggle(move |_| toggle_this(index))
            .size(16)
            .text_size(typography::LABEL);
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

    let reset = button(text("Reset to Default").size(typography::LABEL)).padding([spacing::SPACE_10, spacing::MD]).width(Length::Fill).style(button_style::secondary(theme)).on_press(on_reset);
    column![
        text("Select Columns to Display").size(typography::PANEL_TITLE).color(theme.text_primary()),
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
    fn columns_panel_builds_without_panicking() {
        let labels = ["Code", "Name", "Type"];
        let _: Element<'_, ()> = columns_panel(Theme::Light, &labels, &[true, false, true], |_| (), (), (), ());
    }
}
