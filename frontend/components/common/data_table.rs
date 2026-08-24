//! Reusable data table (Section 6): a card-contained table used by every
//! list-style view. Header row carries column labels with a sort-
//! direction indicator and a trailing empty action column; body rows are
//! separated by hairline dividers (not alternating stripes) and highlight
//! subtly on hover; cells come in five variants covering what the app's
//! list views need (plain text, dates, a person avatar + two-line name,
//! a colored status pill, and a progress bar with a percentage label).

use iced::widget::{button, checkbox, column, container, progress_bar, row, text, Space};
use iced::{Alignment, Background, Border, Color, Element, Font, Length, Theme};

use crate::frontend::styles::badges::{tone_colors, BadgeTone};
use crate::frontend::styles::{borders, colors, panels, spacing, table as table_style};

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
/// row's main purpose. Pass `None` for `show_checkbox` selection state to
/// omit the leading checkbox column entirely.
pub fn view<'a, Message: Clone + 'a>(
    columns: Vec<ColumnDef<'a>>,
    rows: Vec<Row>,
    selected: Option<&'a [bool]>,
    on_row_select: impl Fn(usize, bool) -> Message + Clone + 'a,
    on_row_action: impl Fn(usize) -> Message + Clone + 'a,
    on_primary_action: impl Fn(usize) -> Message + Clone + 'a,
) -> Element<'a, Message> {
    let show_checkbox = selected.is_some();

    let mut header = row![].spacing(spacing::MD).padding(iced::Padding { top: spacing::SM, right: spacing::MD, bottom: spacing::SM, left: spacing::MD }).align_y(Alignment::Center);
    if show_checkbox {
        header = header.push(Space::with_width(Length::Fixed(24.0)));
    }
    for column_def in &columns {
        let arrow = match column_def.sort {
            SortDirection::Ascending => " \u{25B2}",
            SortDirection::Descending => " \u{25BC}",
            SortDirection::None => "",
        };
        header = header.push(text(format!("{}{}", column_def.label, arrow)).size(12).color(colors::WHITE_NEUTRAL_FRONT).width(column_def.width));
    }
    header = header.push(Space::with_width(Length::Fixed(PRIMARY_ACTION_WIDTH)));
    header = header.push(Space::with_width(Length::Fixed(32.0)));

    let header_container = container(header).style(table_style::header_row());

    let mut body = column![];
    let row_count = rows.len();
    for (row_index, table_row) in rows.into_iter().enumerate() {
        let mut cells_row = row![].spacing(spacing::MD).padding([spacing::SM, spacing::MD]).align_y(Alignment::Center);
        if show_checkbox {
            let is_checked = selected.and_then(|s| s.get(row_index)).copied().unwrap_or(false);
            let select_this_row = on_row_select.clone();
            cells_row = cells_row.push(container(checkbox("", is_checked).on_toggle(move |value| select_this_row(row_index, value))).width(Length::Fixed(24.0)));
        }
        for (cell, column_def) in table_row.cells.into_iter().zip(columns.iter()) {
            cells_row = cells_row.push(container(cell_view::<Message>(cell)).width(column_def.width));
        }
        let primary_action_slot: Element<'a, Message> = match table_row.primary_action_label {
            Some(label) => button(text(label).size(12)).style(primary_row_button).padding([6.0, 14.0]).on_press(on_primary_action(row_index)).into(),
            None => Space::with_width(Length::Shrink).into(),
        };
        cells_row = cells_row.push(container(primary_action_slot).width(Length::Fixed(PRIMARY_ACTION_WIDTH)).align_x(iced::alignment::Horizontal::Left));
        cells_row = cells_row.push(
            container(
                button(text("\u{22EE}").size(14).color(colors::WHITE_NEUTRAL_FRONT))
                    .style(kebab_button)
                    .on_press(on_row_action(row_index)),
            )
            .width(Length::Fixed(32.0)),
        );

        let interactive_row = button(cells_row).style(table_style::row()).on_press(on_row_action(row_index)).width(Length::Fill);

        body = body.push(interactive_row);
        if row_index + 1 < row_count {
            body = body.push(container(Space::with_height(Length::Fixed(1.0))).width(Length::Fill).style(move |_theme: &Theme| container::Style {
                background: Some(Background::Color(table_style::divider_color())),
                ..container::Style::default()
            }));
        }
    }

    let card = column![header_container, body].width(Length::Fill);

    container(card).style(panels::flat_card(borders::RADIUS_MD)).width(Length::Fill).into()
}

fn cell_view<'a, Message: 'a>(cell: Cell) -> Element<'a, Message> {
    match cell {
        Cell::Text(value) => text(value).size(13).into(),
        Cell::Date(value) => text(value).size(13).color(colors::WHITE_NEUTRAL_FRONT).into(),
        Cell::Person { name, subtitle } => {
            let initial = name.chars().next().unwrap_or('?').to_uppercase().to_string();
            let avatar = container(text(initial).size(12).color(Color::WHITE).font(Font { weight: iced::font::Weight::Bold, ..Font::DEFAULT }))
                .width(Length::Fixed(28.0))
                .height(Length::Fixed(28.0))
                .align_x(iced::alignment::Horizontal::Center)
                .align_y(iced::alignment::Vertical::Center)
                .style(|_theme: &Theme| container::Style {
                    background: Some(Background::Color(colors::WHITE_PRIMARY)),
                    border: Border { radius: borders::RADIUS_SM.into(), ..Border::default() },
                    ..container::Style::default()
                });
            row![avatar, column![text(name).size(13), text(subtitle).size(11).color(colors::WHITE_NEUTRAL_FRONT)].spacing(2.0)]
                .spacing(spacing::SM)
                .align_y(Alignment::Center)
                .into()
        }
        Cell::Status { label, tone } => {
            let (bg, front) = tone_colors(tone);
            container(row![text("\u{25CF}").size(9).color(front), text(label).size(12).color(front)].spacing(6.0).align_y(Alignment::Center))
                .padding([4.0, 10.0])
                .style(move |_theme: &Theme| container::Style {
                    background: Some(Background::Color(bg)),
                    border: Border { radius: borders::RADIUS_PILL.into(), ..Border::default() },
                    ..container::Style::default()
                })
                .into()
        }
        Cell::Progress { fraction, label } => column![
            progress_bar(0.0..=1.0, fraction).height(Length::Fixed(6.0)).style(|_theme: &Theme| iced::widget::progress_bar::Style {
                background: Background::Color(colors::WHITE_NEUTRAL_BG),
                bar: Background::Color(colors::WHITE_PRIMARY),
                border: Border::default(),
            }),
            text(label).size(11).color(colors::WHITE_NEUTRAL_FRONT),
        ]
        .spacing(4.0)
        .into(),
    }
}

fn kebab_button(_theme: &Theme, status: iced::widget::button::Status) -> iced::widget::button::Style {
    let background = if matches!(status, iced::widget::button::Status::Hovered) { Some(Background::Color(colors::WHITE_NEUTRAL_BG)) } else { None };
    iced::widget::button::Style { background, text_color: colors::WHITE_NEUTRAL_FRONT, border: Border { radius: borders::RADIUS_SM.into(), ..Border::default() }, shadow: iced::Shadow::default() }
}

/// A row's visible primary-action button: an outline in `white_primary`
/// rather than a solid fill, so a table full of them doesn't compete
/// visually with the page-level primary action in the header.
fn primary_row_button(_theme: &Theme, status: iced::widget::button::Status) -> iced::widget::button::Style {
    let background = match status {
        iced::widget::button::Status::Hovered | iced::widget::button::Status::Pressed => Color { a: 0.1, ..colors::WHITE_PRIMARY },
        _ => Color::TRANSPARENT,
    };
    iced::widget::button::Style {
        background: Some(Background::Color(background)),
        text_color: colors::WHITE_PRIMARY,
        border: Border { color: colors::WHITE_PRIMARY, width: 1.0, radius: borders::RADIUS_SM.into() },
        shadow: iced::Shadow::default(),
    }
}
