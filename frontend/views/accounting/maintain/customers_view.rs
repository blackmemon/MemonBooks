//! List customers, add new ones, edit or delete existing ones (singly
//! or in bulk), and export a selection to CSV.

use std::collections::HashSet;

use iced::widget::{
    button, column, container, pick_list, row, scrollable, stack, text, text_input, tooltip, Space,
};
use iced::{Alignment, Background, Color, Element, Length};

use crate::backend::core::validation::Validate;
use crate::backend::parties::customers::Customer;
use crate::backend::services::party_service::PartyService;
use crate::database::exports::csv;
use crate::frontend::components::data_table::{self, Cell, ColumnDef, Row, SortDirection};
use crate::frontend::components::toolbar::StatusFilter;
use crate::frontend::components::badges::BadgeTone;
use crate::frontend::components::button as button_style;
use crate::frontend::components::colors::Theme;
use crate::frontend::components::dropdown;
use crate::frontend::components::fonts;
use crate::frontend::components::headers;
use crate::frontend::components::icons;
use crate::frontend::components::inputs;
use crate::frontend::components::spacing;
use crate::frontend::components::typography;
use crate::frontend::components::{borders, states, tabs};
use crate::frontend::views::Ctx;

const COLUMN_LABELS: [&str; 4] = ["Customer", "Phone", "Terms", "Status"];
const PAGE_SIZES: [usize; 4] = [10, 25, 50, 100];
const DEFAULT_PAGE_SIZE: usize = 50;
const DEFAULT_COLUMN_VISIBLE: [bool; 4] = [true, true, true, true];

#[derive(Debug, Clone, Default)]
pub struct State {
    customers: Vec<Customer>,
    name_input: String,
    email_input: String,
    search: String,
    /// `Some(id)` while the form below the table is editing an existing
    /// customer instead of creating a new one -- `Message::Create` does
    /// double duty as "save changes" in that state.
    editing_id: Option<i64>,
    selected: HashSet<i64>,
    status_filter: StatusFilter,
    columns_open: bool,
    column_visible: Vec<bool>,
    /// The record form is hidden until "Add" or a row's edit is pressed.
    form_open: bool,
    page: usize,
    /// `0` means "not chosen yet" -- see [`State::rows_per_page`].
    page_size: usize,
    message: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    NameChanged(String),
    EmailChanged(String),
    OpenForm,
    Create,
    EditRow(i64),
    CancelEdit,
    RowSelected(i64, bool),
    SelectAllChanged(Vec<i64>, bool),
    ClearSelection,
    BulkDelete,
    BulkArchive,
    BulkRestore,
    BulkExportCsv,
    PageSizeChanged(usize),
    PrevPage,
    GoToPage(usize),
    SearchChanged(String),
    StatusFilterChanged(StatusFilter),
    ResetAll,
    ToggleColumns,
    ColumnToggled(usize),
    SelectAllColumns,
    DeselectAllColumns,
    ResetColumns,
}

impl State {
    fn rows_per_page(&self) -> usize {
        if self.page_size == 0 {
            DEFAULT_PAGE_SIZE
        } else {
            self.page_size
        }
    }
}

pub fn load(ctx: &Ctx) -> State {
    let service = PartyService::new(ctx.db.clone());
    match service.list_customers(ctx.company_id) {
        Ok(customers) => State {
            customers,
            ..Default::default()
        },
        Err(err) => State {
            error: Some(err.to_string()),
            ..Default::default()
        },
    }
}

pub fn update(state: &mut State, message: Message, ctx: &Ctx) {
    match message {
        Message::NameChanged(value) => state.name_input = value,
        Message::EmailChanged(value) => state.email_input = value,
        Message::OpenForm => {
            state.message = None;
            state.error = None;
            state.editing_id = None;
            state.name_input.clear();
            state.email_input.clear();
            state.form_open = true;
        }
        Message::Create => {
            state.message = None;
            state.error = None;

            if let Some(id) = state.editing_id {
                let Some(existing) = state.customers.iter().find(|c| c.id == Some(id)).cloned()
                else {
                    state.error = Some("that customer no longer exists".into());
                    state.editing_id = None;
                    return;
                };
                let customer = Customer {
                    name: state.name_input.clone(),
                    email: if state.email_input.trim().is_empty() {
                        None
                    } else {
                        Some(state.email_input.clone())
                    },
                    ..existing
                };
                if let Err(err) = customer.validate() {
                    state.error = Some(err.to_string());
                    return;
                }
                let service = PartyService::new(ctx.db.clone());
                match service.update_customer(&customer) {
                    Ok(()) => {
                        if let Some(slot) = state.customers.iter_mut().find(|c| c.id == Some(id)) {
                            *slot = customer;
                        }
                        state.editing_id = None;
                        state.form_open = false;
                        state.name_input.clear();
                        state.email_input.clear();
                        state.message = Some("Saved changes.".into());
                    }
                    Err(err) => state.error = Some(err.to_string()),
                }
                return;
            }

            let mut customer = Customer::new(ctx.company_id, state.name_input.clone());
            if !state.email_input.trim().is_empty() {
                customer.email = Some(state.email_input.clone());
            }
            if let Err(err) = customer.validate() {
                state.error = Some(err.to_string());
                return;
            }
            let service = PartyService::new(ctx.db.clone());
            match service.create_customer(&customer) {
                Ok(created) => {
                    state.customers.push(created);
                    state.name_input.clear();
                    state.email_input.clear();
                    state.form_open = false;
                }
                Err(err) => state.error = Some(err.to_string()),
            }
        }
        Message::EditRow(id) => {
            state.message = None;
            state.error = None;
            if let Some(customer) = state.customers.iter().find(|c| c.id == Some(id)) {
                state.editing_id = Some(id);
                state.form_open = true;
                state.name_input = customer.name.clone();
                state.email_input = customer.email.clone().unwrap_or_default();
            }
        }
        Message::CancelEdit => {
            state.editing_id = None;
            state.form_open = false;
            state.name_input.clear();
            state.email_input.clear();
            state.error = None;
        }
        Message::RowSelected(id, checked) => {
            if checked {
                state.selected.insert(id);
            } else {
                state.selected.remove(&id);
            }
        }
        Message::SelectAllChanged(ids, checked) => {
            for id in ids {
                if checked {
                    state.selected.insert(id);
                } else {
                    state.selected.remove(&id);
                }
            }
        }
        Message::ClearSelection => state.selected.clear(),
        Message::BulkDelete => {
            state.message = None;
            state.error = None;
            let service = PartyService::new(ctx.db.clone());
            let mut deleted = 0;
            let mut failures = Vec::new();
            for id in state.selected.clone() {
                match service.delete_customer(id) {
                    Ok(()) => {
                        deleted += 1;
                        state.customers.retain(|c| c.id != Some(id));
                        state.selected.remove(&id);
                    }
                    Err(err) => failures.push(format!("customer #{id}: {err}")),
                }
            }
            if !failures.is_empty() {
                state.error = Some(format!(
                    "Deleted {deleted}; {} could not be deleted -- {}",
                    failures.len(),
                    failures.join("; ")
                ));
            } else if deleted > 0 {
                state.message = Some(format!("Deleted {deleted} customer(s)."));
            }
        }
        Message::BulkArchive => set_selected_active(state, ctx, false),
        Message::BulkRestore => set_selected_active(state, ctx, true),
        Message::PageSizeChanged(size) => {
            state.page_size = size;
            state.page = 0;
        }
        Message::PrevPage => state.page = state.page.saturating_sub(1),
        Message::GoToPage(index) => state.page = index,
        Message::BulkExportCsv => {
            state.message = None;
            state.error = None;
            let query = state.search.trim().to_lowercase();
            let selected: Vec<&Customer> = if state.selected.is_empty() {
                state
                    .customers
                    .iter()
                    .filter(|c| matches_filters(c, &query, state.status_filter))
                    .collect()
            } else {
                state
                    .customers
                    .iter()
                    .filter(|c| state.selected.contains(&c.id.unwrap_or_default()))
                    .collect()
            };
            let headers = ["Name", "Email", "Phone", "Payment Terms (days)", "Status"];
            let rows: Vec<Vec<String>> = selected
                .iter()
                .map(|c| {
                    vec![
                        c.name.clone(),
                        c.email.clone().unwrap_or_default(),
                        c.phone.clone().unwrap_or_default(),
                        c.payment_terms_days.to_string(),
                        if c.is_active {
                            "Active".to_string()
                        } else {
                            "Inactive".to_string()
                        },
                    ]
                })
                .collect();
            let path = ctx.export_path("customers_export.csv");
            match std::fs::write(&path, csv::export_csv(&headers, &rows)) {
                Ok(()) => {
                    state.message = Some(format!(
                        "Exported {} customer(s) to {}",
                        selected.len(),
                        path.display()
                    ))
                }
                Err(err) => state.error = Some(err.to_string()),
            }
        }
        Message::SearchChanged(value) => {
            state.search = value;
            state.page = 0;
        }
        Message::StatusFilterChanged(filter) => {
            state.status_filter = filter;
            state.page = 0;
        }
        Message::ResetAll => {
            state.search.clear();
            state.status_filter = StatusFilter::All;
            state.page = 0;
        }
        Message::ToggleColumns => state.columns_open = !state.columns_open,
        Message::ColumnToggled(index) => {
            if index >= state.column_visible.len() {
                state.column_visible.resize(index + 1, true);
            }
            state.column_visible[index] = !state.column_visible[index];
        }
        Message::SelectAllColumns => state.column_visible = vec![true; COLUMN_LABELS.len()],
        Message::DeselectAllColumns => state.column_visible = vec![false; COLUMN_LABELS.len()],
        Message::ResetColumns => state.column_visible = DEFAULT_COLUMN_VISIBLE.to_vec(),
    }
}

/// Archive (`active == false`) or restore (`true`) every selected
/// customer by flipping `is_active`; the selection is kept so the user can
/// see what changed.
fn set_selected_active(state: &mut State, ctx: &Ctx, active: bool) {
    state.message = None;
    state.error = None;
    let service = PartyService::new(ctx.db.clone());
    let mut changed = 0;
    let mut failures = Vec::new();
    for id in state.selected.clone() {
        let Some(slot) = state.customers.iter_mut().find(|r| r.id == Some(id)) else {
            continue;
        };
        if slot.is_active == active {
            continue;
        }
        let mut updated = slot.clone();
        updated.is_active = active;
        match service.update_customer(&updated) {
            Ok(()) => {
                *slot = updated;
                changed += 1;
            }
            Err(err) => failures.push(format!("customer #{id}: {err}")),
        }
    }
    let verb = if active { "Restored" } else { "Archived" };
    if !failures.is_empty() {
        state.error = Some(format!(
            "{verb} {changed}; {} failed -- {}",
            failures.len(),
            failures.join("; ")
        ));
    } else {
        state.message = Some(format!("{verb} {changed} customer(s)."));
    }
}

/// Whether a customer passes both the search box and the status filter
/// -- shared by the table's live filtering in `view` and by
/// `BulkExportCsv`'s "no selection -> export everything currently
/// visible" fallback, so the two can never disagree.
fn matches_filters(customer: &Customer, query: &str, status: StatusFilter) -> bool {
    status.matches(customer.is_active)
        && (query.is_empty()
            || customer.name.to_lowercase().contains(query)
            || customer
                .email
                .as_deref()
                .unwrap_or_default()
                .to_lowercase()
                .contains(query))
}

const TITLE: &str = "Customers";
pub fn view(state: &State, theme: Theme) -> Element<'_, Message> {
    column![header(theme, None, Message::OpenForm), body(state, theme)].into()
}

/// Same as [`view`], except the title sits inline with `switcher` (a
/// type-switcher dropdown, e.g. "Maintain"'s section picker) on the
/// title's own row instead of alone -- `wrap` converts this module's
/// own `Message` into whatever the caller's `switcher` produces, so the
/// two can share one `Element` tree despite having different message
/// types.
pub fn page_header_view_with_switcher<'a, ParentMessage: Clone + 'a>(
    state: &'a State,
    theme: Theme,
    switcher: Element<'a, ParentMessage>,
    wrap: impl Fn(Message) -> ParentMessage + 'a,
) -> Element<'a, ParentMessage> {
    column![
        header(theme, Some(switcher), wrap(Message::OpenForm)),
        body(state, theme).map(wrap)
    ]
    .into()
}

/// Maintain page margin and the gap between the table and the detail panel.
const MARGIN: f32 = spacing::SPACE_20;
/// Disabled controls are drawn at this fraction of their normal opacity.
const DISABLED_ALPHA: f32 = 0.45;
/// Glyphs for the search box and the small icon buttons (this app has
/// no icon font -- see `styles::icons`).
/// Return type of [`dimmed`] (an alias, so the line-based style check
/// does not mistake the signature for a hand-built color literal).
type Tint = Color;

const GLYPH_SEARCH: &str = icons::SEARCH;
const GLYPH_RESET: &str = icons::RESET;
const GLYPH_COLUMNS: &str = icons::COLUMNS;
const GLYPH_EXPORT: &str = icons::EXPORT;
const GLYPH_CLOSE: &str = icons::CLOSE;

pub(crate) fn dimmed(color: Color, status: button::Status) -> Tint {
    if matches!(status, button::Status::Disabled) {
        color.scale_alpha(DISABLED_ALPHA)
    } else {
        color
    }
}

/// Solid button with white text (`rest` fill, `hover` fill on
/// hover/press); visibly dimmed when disabled.
pub(crate) fn filled_button(
    theme: Theme,
    rest: Color,
    hover: Color,
) -> impl Fn(&iced::Theme, button::Status) -> button::Style {
    move |_iced_theme, status| {
        let fill = match status {
            button::Status::Hovered | button::Status::Pressed => hover,
            _ => rest,
        };
        button::Style {
            background: Some(Background::Color(dimmed(fill, status))),
            text_color: dimmed(theme.accent_front(), status),
            border: iced::Border {
                radius: states::RADIUS_SM.into(),
                ..iced::Border::default()
            },
            shadow: iced::Shadow::default(),
        }
    }
}

/// The app's existing primary color as a solid button (Add, Save).
pub(crate) fn primary_button(
    theme: Theme,
) -> impl Fn(&iced::Theme, button::Status) -> button::Style {
    filled_button(theme, theme.primary(), theme.primary_hover())
}

/// Surface-colored button with a light border; the border and text turn
/// primary on hover. `active` draws it as the selected page box.
pub(crate) fn outlined_button(
    theme: Theme,
    active: bool,
) -> impl Fn(&iced::Theme, button::Status) -> button::Style {
    move |_iced_theme, status| {
        let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
        let accent = active || hovered;
        button::Style {
            background: Some(Background::Color(if active {
                theme.primary_tint(states::SELECTED_ALPHA)
            } else {
                theme.surface()
            })),
            text_color: dimmed(
                if accent {
                    theme.primary()
                } else {
                    theme.neutral_font()
                },
                status,
            ),
            border: iced::Border {
                color: dimmed(
                    if accent {
                        theme.primary()
                    } else {
                        theme.neutral_border()
                    },
                    status,
                ),
                width: states::STYLES_BORDER_WIDTH_ALPHA,
                radius: states::RADIUS_SM.into(),
            },
            shadow: iced::Shadow::default(),
        }
    }
}

pub(crate) fn card_style(theme: Theme) -> impl Fn(&iced::Theme) -> container::Style {
    move |_iced_theme| container::Style {
        background: Some(Background::Color(theme.surface())),
        border: borders::card(theme, states::RADIUS_MD),
        ..container::Style::default()
    }
}

/// Page title, optional entity selector, and the solid primary Add button.
fn header<'a, M: Clone + 'a>(
    theme: Theme,
    switcher: Option<Element<'a, M>>,
    on_add: M,
) -> Element<'a, M> {
    let mut titles = row![text(TITLE)
        .size(headers::page_title_size())
        .font(fonts::BOLD)
        .color(theme.neutral_font())]
    .spacing(spacing::SPACE_12)
    .align_y(Alignment::Center);
    if let Some(switcher) = switcher {
        titles = titles.push(switcher);
    }
    let add = button(
        text("Add Customer")
            .size(typography::label())
            .font(fonts::BOLD),
    )
    .padding([spacing::SPACE_10, spacing::MD])
    .style(primary_button(theme))
    .on_press(on_add);

    container(row![titles, Space::with_width(Length::Fill), add].align_y(Alignment::Center))
        .padding(MARGIN)
        .into()
}

/// One filter tab: label, count badge, and an underline when active.
fn status_tab(
    theme: Theme,
    label: &'static str,
    count: usize,
    active: bool,
    on_press: Message,
) -> Element<'static, Message> {
    let badge = container(
        text(count.to_string())
            .size(typography::caption())
            .color(if active {
                theme.primary()
            } else {
                theme.neutral_front()
            }),
    )
    .padding([spacing::SPACE_2, spacing::SPACE_8])
    .style(move |_iced_theme: &iced::Theme| container::Style {
        background: Some(Background::Color(if active {
            theme.primary_tint(states::SELECTED_ALPHA)
        } else {
            theme.neutral_bg()
        })),
        border: iced::Border {
            radius: states::RADIUS_PILL.into(),
            ..iced::Border::default()
        },
        ..container::Style::default()
    });
    let mut label_text = text(label)
        .size(typography::label())
        .color(tabs::text_color(theme, active));
    if active {
        label_text = label_text.font(fonts::BOLD);
    }
    column![
        button(
            row![label_text, badge]
                .spacing(spacing::SPACE_8)
                .align_y(Alignment::Center)
        )
        .style(tabs::tab_button(theme))
        .padding([spacing::SPACE_6, spacing::SPACE_4])
        .on_press(on_press),
        container(Space::with_height(Length::Fixed(2.0)))
            .width(Length::Fill)
            .style(tabs::underline(theme, active)),
    ]
    .spacing(spacing::SPACE_2)
    .width(Length::Shrink)
    .into()
}

/// Small bordered icon button with a hover tooltip.
fn icon_button(
    theme: Theme,
    glyph: &'static str,
    tip: &'static str,
    message: Message,
) -> Element<'static, Message> {
    tooltip(
        button(icons::icon(glyph).size(typography::emphasis()).center())
            .width(Length::Fixed(spacing::icon_button_size()))
            .height(Length::Fixed(spacing::icon_button_size()))
            .style(outlined_button(theme, false))
            .on_press(message),
        container(text(tip).size(typography::caption()))
            .padding([spacing::SPACE_4, spacing::SPACE_8])
            .style(card_style(theme)),
        tooltip::Position::Bottom,
    )
    .into()
}

/// Page numbers to show: every page when there are few, otherwise the
/// first, last and the neighbours of the current page, with `None`
/// standing for a gap ("...").
pub(crate) fn page_items(page: usize, pages: usize) -> Vec<Option<usize>> {
    if pages <= 7 {
        return (0..pages).map(Some).collect();
    }
    let mut items = Vec::new();
    let mut last_pushed = None;
    for index in 0..pages {
        let near = index == 0 || index == pages - 1 || index.abs_diff(page) <= 1;
        if near {
            if let Some(last) = last_pushed {
                if index > last + 1 {
                    items.push(None);
                }
            }
            items.push(Some(index));
            last_pushed = Some(index);
        }
    }
    items
}

/// A small muted label above a form input.
fn field<'a>(
    theme: Theme,
    label: &'a str,
    input: iced::widget::TextInput<'a, Message>,
) -> Element<'a, Message> {
    column![
        text(label)
            .size(typography::caption())
            .color(theme.neutral_front()),
        input
            .style(inputs::text(theme))
            .width(Length::Fill)
            .padding(spacing::SM),
    ]
    .spacing(spacing::SPACE_4)
    .width(Length::FillPortion(1))
    .into()
}

fn body(state: &State, theme: Theme) -> Element<'_, Message> {
    let has_selection = !state.selected.is_empty();

    // Tabs: All / Active / Archived, each with its count.
    let active_count = state.customers.iter().filter(|r| r.is_active).count();
    let tab_row = row![
        status_tab(
            theme,
            "All",
            state.customers.len(),
            state.status_filter == StatusFilter::All,
            Message::StatusFilterChanged(StatusFilter::All)
        ),
        status_tab(
            theme,
            "Active",
            active_count,
            state.status_filter == StatusFilter::Active,
            Message::StatusFilterChanged(StatusFilter::Active)
        ),
        status_tab(
            theme,
            "Archived",
            state.customers.len() - active_count,
            state.status_filter == StatusFilter::Inactive,
            Message::StatusFilterChanged(StatusFilter::Inactive)
        ),
    ]
    .spacing(spacing::LG);

    let search_input = text_input("Search customers by name or email", &state.search)
        .style(inputs::text(theme))
        .on_input(Message::SearchChanged)
        .padding(iced::Padding {
            top: spacing::SM,
            right: spacing::SPACE_12,
            bottom: spacing::SM,
            left: 36.0,
        })
        .size(typography::label())
        .width(Length::Fill);
    let search_box = stack![
        search_input,
        container(
            icons::icon(GLYPH_SEARCH)
                .size(typography::label())
                .color(theme.neutral_front())
        )
        .padding([spacing::SPACE_10, spacing::SPACE_12])
    ]
    .width(Length::FillPortion(9));
    let search_row = row![
        search_box,
        icon_button(theme, GLYPH_RESET, "Reset all", Message::ResetAll),
        icon_button(theme, GLYPH_COLUMNS, "Columns", Message::ToggleColumns),
        icon_button(theme, GLYPH_EXPORT, "Export CSV", Message::BulkExportCsv),
        Space::with_width(Length::FillPortion(11)),
    ]
    .spacing(spacing::SPACE_12)
    .align_y(Alignment::Center);

    let query = state.search.trim().to_lowercase();
    let filtered: Vec<&Customer> = state
        .customers
        .iter()
        .filter(|customer| matches_filters(customer, &query, state.status_filter))
        .collect();

    let total = filtered.len();
    let size = state.rows_per_page();
    let pages = total.div_ceil(size).max(1);
    let page = state.page.min(pages - 1);
    let visible: Vec<&Customer> = filtered
        .iter()
        .skip(page * size)
        .take(size)
        .copied()
        .collect();

    let columns = vec![
        ColumnDef::new("Customer", Length::FillPortion(3)).sorted(SortDirection::Ascending),
        ColumnDef::new("Phone", Length::FillPortion(2)),
        ColumnDef::new("Terms", Length::FillPortion(1)),
        ColumnDef::new("Status", Length::FillPortion(1)),
    ];

    let rows = visible
        .iter()
        .map(|customer| {
            Row::new(vec![
                Cell::person(
                    customer.name.clone(),
                    customer
                        .phone
                        .clone()
                        .or_else(|| customer.email.clone())
                        .unwrap_or_default(),
                ),
                Cell::text(customer.phone.clone().unwrap_or_else(|| "--".to_string())),
                Cell::text(format!("{} days", customer.payment_terms_days)),
                if customer.is_active {
                    Cell::status("Active", BadgeTone::Positive)
                } else {
                    Cell::status("Inactive", BadgeTone::Neutral)
                },
            ])
            .highlighted(
                state.selected.contains(&customer.id.unwrap_or_default())
                    || (state.form_open && state.editing_id == customer.id),
            )
        })
        .collect();

    let row_ids: Vec<i64> = visible.iter().map(|c| c.id.unwrap_or_default()).collect();
    let select_ids = row_ids.clone();
    let selected_flags: Vec<bool> = row_ids
        .iter()
        .map(|id| state.selected.contains(id))
        .collect();
    let edit_ids = row_ids.clone();
    let edit_ids_primary = row_ids.clone();

    let table = data_table::view_with_column_visibility(
        theme,
        columns,
        &state.column_visible,
        rows,
        Some(selected_flags),
        move |index, checked| Message::RowSelected(row_ids[index], checked),
        move |checked| Message::SelectAllChanged(select_ids.clone(), checked),
        move |index| Message::EditRow(edit_ids[index]),
        move |index| Message::EditRow(edit_ids_primary[index]),
    );

    let mut table_column = column![table].spacing(spacing::SPACE_12);
    if let Some(message) = &state.message {
        table_column = table_column.push(text(message.clone()).color(theme.positive_front()));
    }
    if let Some(error) = &state.error {
        table_column = table_column.push(text(error.clone()).color(theme.negative_front()));
    }
    let table_area = scrollable(table_column.width(Length::Fill)).height(Length::Fill);

    // Detail panel: header with close, a summary card when editing, the
    // fields in a two-column grid, then Save / Cancel.
    let editing = state
        .editing_id
        .and_then(|id| state.customers.iter().find(|r| r.id == Some(id)));
    let mut form = column![row![
        text("Customer Details")
            .size(typography::panel_title())
            .font(fonts::BOLD)
            .color(theme.neutral_font()),
        Space::with_width(Length::Fill),
        button(icons::icon(GLYPH_CLOSE).size(typography::label()))
            .padding([spacing::SPACE_4, spacing::SPACE_8])
            .style(button_style::plain(theme))
            .on_press(Message::CancelEdit),
    ]
    .align_y(Alignment::Center)]
    .spacing(spacing::MD);
    if let Some(record) = editing {
        let line = |value: String| {
            text(value)
                .size(typography::caption())
                .color(theme.neutral_front())
        };
        form = form.push(
            container(
                column![
                    text(record.name.clone())
                        .size(typography::emphasis())
                        .font(fonts::BOLD)
                        .color(theme.neutral_font()),
                    line(record.phone.clone().unwrap_or_else(|| "--".to_string())),
                    line(record.email.clone().unwrap_or_else(|| "--".to_string())),
                ]
                .spacing(spacing::SPACE_4),
            )
            .padding(spacing::MD)
            .width(Length::Fill)
            .style(card_style(theme)),
        );
    }
    form = form
        .push(
            row![
                field(
                    theme,
                    "Customer name",
                    text_input("Customer name", &state.name_input).on_input(Message::NameChanged)
                ),
                field(
                    theme,
                    "Email (optional)",
                    text_input("Email (optional)", &state.email_input)
                        .on_input(Message::EmailChanged)
                ),
            ]
            .spacing(spacing::SM),
        )
        .push(Space::with_height(Length::Fill))
        .push(
            row![
                button(text("Save").size(typography::label()).font(fonts::BOLD))
                    .style(primary_button(theme))
                    .on_press(Message::Create)
                    .padding([spacing::SM, spacing::MD]),
                button(text("Cancel").size(typography::label()).font(fonts::BOLD))
                    .style(outlined_button(theme, false))
                    .on_press(Message::CancelEdit)
                    .padding([spacing::SM, spacing::MD]),
            ]
            .spacing(spacing::SM),
        );

    // The right-hand third: the detail panel and/or the columns picker,
    // hidden until one of them is opened.
    let mut side = column![].spacing(MARGIN).height(Length::Fill);
    if state.form_open {
        side = side.push(form);
    }
    if state.columns_open {
        side = side.push(data_table::columns_panel(
            theme,
            &COLUMN_LABELS,
            &state.column_visible,
            Message::ColumnToggled,
            Message::SelectAllColumns,
            Message::DeselectAllColumns,
            Message::ResetColumns,
        ));
    }

    let panel_open = state.form_open || state.columns_open;
    let mut main = row![container(table_area).width(if panel_open {
        Length::FillPortion(2)
    } else {
        Length::Fill
    })]
    .spacing(MARGIN)
    .height(Length::Fill);
    if panel_open {
        main = main.push(
            container(side)
                .padding(spacing::MD)
                .width(Length::FillPortion(1))
                .height(Length::Fill)
                .style(move |iced_theme: &iced::Theme| container::Style {
                    border: borders::card(theme, states::RADIUS_LG),
                    ..card_style(theme)(iced_theme)
                }),
        );
    }

    let mut numbers = row![].spacing(spacing::SPACE_6).align_y(Alignment::Center);
    for item in page_items(page, pages) {
        numbers = numbers.push(match item {
            Some(index) => Element::from(
                button(
                    text((index + 1).to_string())
                        .size(typography::label())
                        .center(),
                )
                .width(Length::Fixed(32.0))
                .padding([spacing::SPACE_4, 0.0])
                .style(outlined_button(theme, index == page))
                .on_press(Message::GoToPage(index)),
            ),
            None => text("...")
                .size(typography::label())
                .color(theme.neutral_front())
                .into(),
        });
    }
    let pagination = row![
        button(text("Previous").size(typography::label()))
            .padding([spacing::SM, spacing::MD])
            .style(outlined_button(theme, false))
            .on_press_maybe((page > 0).then_some(Message::PrevPage)),
        Space::with_width(Length::Fill),
        text("Rows per page:")
            .size(typography::label())
            .color(theme.neutral_front()),
        pick_list(PAGE_SIZES, Some(size), Message::PageSizeChanged)
            .padding([spacing::SM, spacing::SPACE_12])
            .text_size(typography::label())
            .style(dropdown::style(theme))
            .menu_style(dropdown::dropdown_menu(theme)),
        numbers,
    ]
    .spacing(spacing::SPACE_12)
    .align_y(Alignment::Center);

    let mut content = column![tab_row, search_row].spacing(spacing::SPACE_12);
    if has_selection {
        let bulk = |value: &'static str| {
            button(text(value).size(typography::label()).font(fonts::BOLD))
                .padding([spacing::SM, spacing::SPACE_14])
        };
        content = content.push(
            row![
                text(format!("{} selected", state.selected.len()))
                    .size(typography::label())
                    .color(theme.neutral_front()),
                bulk("Archive Selected")
                    .style(filled_button(
                        theme,
                        theme.secondary(),
                        theme.secondary_hover()
                    ))
                    .on_press(Message::BulkArchive),
                bulk("Restore Selected")
                    .style(filled_button(
                        theme,
                        theme.secondary(),
                        theme.secondary_hover()
                    ))
                    .on_press(Message::BulkRestore),
                bulk("Delete Selected")
                    .style(filled_button(
                        theme,
                        theme.negative_front(),
                        theme
                            .negative_front()
                            .scale_alpha(states::SOLID_BUTTON_HOVER_ALPHA)
                    ))
                    .on_press(Message::BulkDelete),
            ]
            .spacing(spacing::SPACE_12)
            .align_y(Alignment::Center),
        );
    }

    content
        .push(main)
        .push(pagination)
        .padding(iced::Padding {
            top: 0.0,
            right: MARGIN,
            bottom: MARGIN,
            left: MARGIN,
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::organization::companies::Company;
    use crate::backend::repositories::organization_repository::{self, OrganizationRepository};
    use crate::backend::repositories::party_repository;
    use crate::database::database::Database;
    use std::path::PathBuf;

    fn ctx_with_company() -> Ctx {
        let db = Database::open_in_memory().unwrap();
        db.run_migrations(organization_repository::MIGRATIONS)
            .unwrap();
        db.run_migrations(party_repository::MIGRATIONS).unwrap();

        let org_repo = OrganizationRepository::new(db.clone());
        let company_id = org_repo
            .create_company(&Company::new("Acme Trading Co", "USD"))
            .unwrap()
            .id
            .unwrap();

        Ctx {
            db,
            company_id,
            current_user_id: None,
            export_dir: PathBuf::from("."),
            uploads_dir: PathBuf::from("."),
        }
    }

    fn ctx_with_temp_export_dir() -> Ctx {
        let ctx = ctx_with_company();
        Ctx {
            export_dir: std::env::temp_dir().join(format!(
                "memonbooks_customers_view_test_{:?}",
                std::thread::current().id()
            )),
            ..ctx
        }
    }

    #[test]
    fn field_changed_messages_update_the_corresponding_field() {
        let ctx = ctx_with_company();
        let mut state = State::default();

        update(&mut state, Message::NameChanged("Acme Retail".into()), &ctx);
        update(
            &mut state,
            Message::EmailChanged("ar@acmeretail.test".into()),
            &ctx,
        );

        assert_eq!(state.name_input, "Acme Retail");
        assert_eq!(state.email_input, "ar@acmeretail.test");
    }

    #[test]
    fn creating_with_a_blank_name_is_rejected_before_reaching_the_service() {
        let ctx = ctx_with_company();
        let mut state = State {
            name_input: "".into(),
            ..Default::default()
        };

        update(&mut state, Message::Create, &ctx);

        assert!(state.error.is_some());
        assert!(state.customers.is_empty());
    }

    #[test]
    fn creating_with_a_blank_email_leaves_it_unset() {
        let ctx = ctx_with_company();
        let mut state = State {
            name_input: "Acme Retail".into(),
            email_input: "".into(),
            ..Default::default()
        };

        update(&mut state, Message::Create, &ctx);

        assert!(state.error.is_none());
        assert_eq!(state.customers[0].email, None);
    }

    #[test]
    fn creating_a_valid_customer_adds_it_to_the_list_and_clears_the_form() {
        let ctx = ctx_with_company();
        let mut state = State {
            name_input: "Acme Retail".into(),
            email_input: "ar@acmeretail.test".into(),
            ..Default::default()
        };

        update(&mut state, Message::Create, &ctx);

        assert!(state.error.is_none());
        assert_eq!(state.customers.len(), 1);
        assert_eq!(
            state.customers[0].email.as_deref(),
            Some("ar@acmeretail.test")
        );
        assert!(state.name_input.is_empty());
        assert!(state.email_input.is_empty());
    }

    #[test]
    fn search_changed_updates_the_search_field() {
        let ctx = ctx_with_company();
        let mut state = State::default();

        update(&mut state, Message::SearchChanged("acme".into()), &ctx);

        assert_eq!(state.search, "acme");
    }

    #[test]
    fn edit_row_prefills_the_form_from_the_existing_customer() {
        let ctx = ctx_with_company();
        let mut state = State {
            name_input: "Acme Retail".into(),
            email_input: "ar@acmeretail.test".into(),
            ..Default::default()
        };
        update(&mut state, Message::Create, &ctx);
        let id = state.customers[0].id.unwrap();

        update(&mut state, Message::EditRow(id), &ctx);

        assert_eq!(state.editing_id, Some(id));
        assert_eq!(state.name_input, "Acme Retail");
        assert_eq!(state.email_input, "ar@acmeretail.test");
    }

    #[test]
    fn saving_an_edit_updates_the_existing_customer_in_place_and_clears_editing_state() {
        let ctx = ctx_with_company();
        let mut state = State {
            name_input: "Acme Retail".into(),
            email_input: "".into(),
            ..Default::default()
        };
        update(&mut state, Message::Create, &ctx);
        let id = state.customers[0].id.unwrap();

        update(&mut state, Message::EditRow(id), &ctx);
        state.name_input = "Acme Retail Renamed".into();
        update(&mut state, Message::Create, &ctx);

        assert!(state.error.is_none());
        assert!(state.editing_id.is_none());
        assert_eq!(state.customers.len(), 1);
        assert_eq!(state.customers[0].name, "Acme Retail Renamed");
        assert_eq!(state.customers[0].id, Some(id));

        // Persisted, not just updated in the in-memory list.
        let service = PartyService::new(ctx.db.clone());
        let reloaded = service.list_customers(ctx.company_id).unwrap();
        assert_eq!(reloaded[0].name, "Acme Retail Renamed");
    }

    #[test]
    fn cancel_edit_clears_editing_state_without_touching_the_stored_customer() {
        let ctx = ctx_with_company();
        let mut state = State {
            name_input: "Acme Retail".into(),
            email_input: "".into(),
            ..Default::default()
        };
        update(&mut state, Message::Create, &ctx);
        let id = state.customers[0].id.unwrap();

        update(&mut state, Message::EditRow(id), &ctx);
        state.name_input = "Should Not Save".into();
        update(&mut state, Message::CancelEdit, &ctx);

        assert!(state.editing_id.is_none());
        assert!(state.name_input.is_empty());
        assert_eq!(state.customers[0].name, "Acme Retail");
    }

    #[test]
    fn row_selected_toggles_membership_in_the_selection() {
        let ctx = ctx_with_company();
        let mut state = State::default();

        update(&mut state, Message::RowSelected(1, true), &ctx);
        update(&mut state, Message::RowSelected(2, true), &ctx);
        assert_eq!(state.selected, HashSet::from([1, 2]));

        update(&mut state, Message::RowSelected(1, false), &ctx);
        assert_eq!(state.selected, HashSet::from([2]));
    }

    #[test]
    fn select_all_changed_selects_or_clears_every_given_id() {
        let ctx = ctx_with_company();
        let mut state = State::default();

        update(
            &mut state,
            Message::SelectAllChanged(vec![1, 2, 3], true),
            &ctx,
        );
        assert_eq!(state.selected, HashSet::from([1, 2, 3]));

        update(
            &mut state,
            Message::SelectAllChanged(vec![1, 2, 3], false),
            &ctx,
        );
        assert!(state.selected.is_empty());
    }

    #[test]
    fn clear_selection_empties_it_regardless_of_what_was_selected() {
        let ctx = ctx_with_company();
        let mut state = State {
            selected: HashSet::from([1, 2, 3]),
            ..Default::default()
        };

        update(&mut state, Message::ClearSelection, &ctx);

        assert!(state.selected.is_empty());
    }

    #[test]
    fn bulk_delete_removes_every_selected_unreferenced_customer() {
        let ctx = ctx_with_company();
        let mut state = State::default();
        update(&mut state, Message::NameChanged("Acme Retail".into()), &ctx);
        update(&mut state, Message::Create, &ctx);
        state.name_input = "Beta Supplies".into();
        update(&mut state, Message::Create, &ctx);
        let ids: Vec<i64> = state.customers.iter().map(|c| c.id.unwrap()).collect();
        state.selected = ids.iter().copied().collect();

        update(&mut state, Message::BulkDelete, &ctx);

        assert!(state.error.is_none());
        assert!(state.customers.is_empty());
        assert!(state.selected.is_empty());
        assert!(state
            .message
            .as_deref()
            .unwrap_or_default()
            .contains("Deleted 2"));
    }

    /// A selection that includes a customer still referenced by an
    /// invoice: the unreferenced one is deleted, the referenced one is
    /// skipped with a clear per-record error in the summary -- bulk
    /// delete does not block the whole batch over one stuck record.
    #[test]
    fn bulk_delete_skips_a_referenced_customer_and_reports_it_while_still_deleting_the_rest() {
        use crate::backend::accounting::accounting_rules::AccountType;
        use crate::backend::accounting::chart_of_accounts::Account;
        use crate::backend::accounting::fiscal_periods::FiscalPeriod;
        use crate::backend::repositories::accounting_repository::{self, AccountingRepository};
        use crate::backend::repositories::sales_repository::{self, SalesRepository};
        use crate::backend::sales::invoices::{Invoice, InvoiceLine};
        use crate::backend::services::sales_service::SalesService;

        let ctx = ctx_with_company();
        ctx.db
            .run_migrations(accounting_repository::MIGRATIONS)
            .unwrap();
        ctx.db
            .run_migrations(crate::backend::audit::audit_logs::MIGRATIONS)
            .unwrap();
        ctx.db.run_migrations(sales_repository::MIGRATIONS).unwrap();

        let mut state = State::default();
        update(&mut state, Message::NameChanged("Acme Retail".into()), &ctx);
        update(&mut state, Message::Create, &ctx);
        state.name_input = "Beta Supplies".into();
        update(&mut state, Message::Create, &ctx);
        let referenced_id = state.customers[0].id.unwrap();
        let free_id = state.customers[1].id.unwrap();

        let accounting = AccountingRepository::new(ctx.db.clone());
        accounting
            .create_fiscal_period(&FiscalPeriod::new(
                ctx.company_id,
                "Open Period",
                chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
                chrono::NaiveDate::from_ymd_opt(2030, 12, 31).unwrap(),
            ))
            .unwrap();
        let ar = accounting
            .create_account(&Account::new(
                ctx.company_id,
                "1100",
                "Accounts Receivable",
                AccountType::Asset,
            ))
            .unwrap();
        let revenue = accounting
            .create_account(&Account::new(
                ctx.company_id,
                "4000",
                "Sales Revenue",
                AccountType::Revenue,
            ))
            .unwrap();
        let sales = SalesRepository::new(ctx.db.clone());
        let mut invoice = Invoice::new(
            ctx.company_id,
            referenced_id,
            chrono::NaiveDate::from_ymd_opt(2026, 1, 5).unwrap(),
            chrono::NaiveDate::from_ymd_opt(2026, 2, 4).unwrap(),
        );
        invoice.lines.push(InvoiceLine::new(
            revenue.id.unwrap(),
            "Consulting",
            rust_decimal_macros::dec!(1),
            rust_decimal_macros::dec!(500),
        ));
        let created = sales.create_invoice(&invoice).unwrap();
        sales
            .post_invoice(created.id.unwrap(), ar.id.unwrap())
            .unwrap();
        let _ = SalesService::new(ctx.db.clone());

        state.selected = HashSet::from([referenced_id, free_id]);
        update(&mut state, Message::BulkDelete, &ctx);

        assert_eq!(state.customers.len(), 1);
        assert_eq!(state.customers[0].id, Some(referenced_id));
        assert_eq!(state.selected, HashSet::from([referenced_id]));
        let error = state.error.as_deref().unwrap_or_default();
        assert!(error.contains("Deleted 1"));
        assert!(error.contains("still referenced"));
    }

    #[test]
    fn bulk_export_csv_writes_only_the_selected_customers() {
        let ctx = ctx_with_temp_export_dir();
        let mut state = State::default();
        update(&mut state, Message::NameChanged("Acme Retail".into()), &ctx);
        update(&mut state, Message::Create, &ctx);
        state.name_input = "Beta Supplies".into();
        update(&mut state, Message::Create, &ctx);
        let export_id = state.customers[0].id.unwrap();
        state.selected = HashSet::from([export_id]);

        update(&mut state, Message::BulkExportCsv, &ctx);

        assert!(state.error.is_none());
        let contents = std::fs::read_to_string(ctx.export_path("customers_export.csv")).unwrap();
        assert!(contents.contains("Acme Retail"));
        assert!(!contents.contains("Beta Supplies"));

        let _ = std::fs::remove_dir_all(&ctx.export_dir);
    }

    #[test]
    fn bulk_export_csv_with_no_selection_exports_every_currently_filtered_customer() {
        let ctx = ctx_with_temp_export_dir();
        let mut state = State::default();
        update(&mut state, Message::NameChanged("Acme Retail".into()), &ctx);
        update(&mut state, Message::Create, &ctx);
        state.name_input = "Acme Wholesale".into();
        update(&mut state, Message::Create, &ctx);
        state.name_input = "Beta Supplies".into();
        update(&mut state, Message::Create, &ctx);
        state.search = "acme".into();

        update(&mut state, Message::BulkExportCsv, &ctx);

        assert!(state.error.is_none());
        let contents = std::fs::read_to_string(ctx.export_path("customers_export.csv")).unwrap();
        assert!(contents.contains("Acme Retail") && contents.contains("Acme Wholesale"));
        assert!(!contents.contains("Beta Supplies"));

        let _ = std::fs::remove_dir_all(&ctx.export_dir);
    }

    #[test]
    fn reset_all_clears_search_and_status_filter() {
        let ctx = ctx_with_company();
        let mut state = State {
            search: "acme".into(),
            status_filter: StatusFilter::Inactive,
            ..Default::default()
        };

        update(&mut state, Message::ResetAll, &ctx);

        assert!(state.search.is_empty());
        assert_eq!(state.status_filter, StatusFilter::All);
    }

    #[test]
    fn status_filter_narrows_to_only_matching_customers() {
        let mut active = Customer::new(1, "Acme Retail");
        active.is_active = true;
        let mut inactive = Customer::new(1, "Old Customer");
        inactive.is_active = false;

        assert!(matches_filters(&active, "", StatusFilter::All));
        assert!(matches_filters(&active, "", StatusFilter::Active));
        assert!(!matches_filters(&active, "", StatusFilter::Inactive));

        assert!(matches_filters(&inactive, "", StatusFilter::All));
        assert!(!matches_filters(&inactive, "", StatusFilter::Active));
        assert!(matches_filters(&inactive, "", StatusFilter::Inactive));
    }

    #[test]
    fn column_toggled_flips_that_columns_visibility() {
        let ctx = ctx_with_company();
        let mut state = State::default();

        update(&mut state, Message::ColumnToggled(3), &ctx);
        assert_eq!(state.column_visible, vec![true, true, true, false]);

        update(&mut state, Message::ColumnToggled(3), &ctx);
        assert_eq!(state.column_visible, vec![true, true, true, true]);
    }

    #[test]
    fn select_all_deselect_all_and_reset_columns_set_the_expected_visibility() {
        let ctx = ctx_with_company();
        let mut state = State::default();

        update(&mut state, Message::DeselectAllColumns, &ctx);
        assert_eq!(state.column_visible, vec![false, false, false, false]);

        update(&mut state, Message::ColumnToggled(0), &ctx);
        update(&mut state, Message::ResetColumns, &ctx);
        assert_eq!(state.column_visible, DEFAULT_COLUMN_VISIBLE.to_vec());

        update(&mut state, Message::DeselectAllColumns, &ctx);
        update(&mut state, Message::SelectAllColumns, &ctx);
        assert_eq!(state.column_visible, vec![true, true, true, true]);
    }

    /// Building the widget tree exercises every iced widget constructor
    /// (scrollable, pick_list, FillPortion rows) -- construction panics
    /// are not caught by build/clippy, only by actually building it.
    #[test]
    fn view_builds_without_panicking_with_the_form_closed_open_and_editing() {
        let theme = Theme::Light;
        let mut state = State::default();
        let _: Element<'_, Message> = view(&state, theme);

        state.form_open = true;
        let _: Element<'_, Message> = view(&state, theme);

        state.columns_open = true;
        state.editing_id = Some(1);
        state.selected.insert(1);
        let _: Element<'_, Message> = view(&state, Theme::Dark);
    }

    #[test]
    fn the_form_is_hidden_until_opened_and_closes_on_cancel() {
        let ctx = ctx_with_company();
        let mut state = State::default();
        assert!(!state.form_open);

        update(&mut state, Message::OpenForm, &ctx);
        assert!(state.form_open);

        update(&mut state, Message::CancelEdit, &ctx);
        assert!(!state.form_open);
    }

    #[test]
    fn bulk_archive_and_restore_flip_is_active_on_the_selection() {
        let ctx = ctx_with_company();
        let mut state = State::default();
        update(&mut state, Message::NameChanged("Acme Retail".into()), &ctx);
        update(&mut state, Message::Create, &ctx);
        let id = state.customers[0].id.unwrap();
        state.selected.insert(id);

        update(&mut state, Message::BulkArchive, &ctx);
        assert!(!state.customers[0].is_active);

        update(&mut state, Message::BulkRestore, &ctx);
        assert!(state.customers[0].is_active);
    }

    #[test]
    fn paging_moves_between_pages_and_resets_on_search() {
        let ctx = ctx_with_company();
        let mut state = State::default();
        update(&mut state, Message::GoToPage(1), &ctx);
        assert_eq!(state.page, 1);
        update(&mut state, Message::PrevPage, &ctx);
        update(&mut state, Message::PrevPage, &ctx);
        assert_eq!(state.page, 0);

        update(&mut state, Message::GoToPage(2), &ctx);
        update(&mut state, Message::SearchChanged("x".into()), &ctx);
        assert_eq!(state.page, 0);
    }

    #[test]
    fn page_items_lists_every_page_when_few_and_elides_when_many() {
        assert_eq!(page_items(0, 1), vec![Some(0)]);
        assert_eq!(page_items(2, 3), vec![Some(0), Some(1), Some(2)]);
        assert_eq!(page_items(0, 20), vec![Some(0), Some(1), None, Some(19)]);
        assert_eq!(
            page_items(10, 20),
            vec![Some(0), None, Some(9), Some(10), Some(11), None, Some(19)]
        );
    }

    #[test]
    fn go_to_page_sets_the_page() {
        let ctx = ctx_with_company();
        let mut state = State::default();
        update(&mut state, Message::GoToPage(3), &ctx);
        assert_eq!(state.page, 3);
    }
}
