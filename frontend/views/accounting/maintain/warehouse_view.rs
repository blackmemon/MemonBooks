//! Warehouses: expandable cards with each warehouse's utilization and
//! categories, an add/edit side panel that lays out the bin grid, and a
//! full-page detail with the bin layout, a per-bin side panel (contents
//! by category, and a small "Place stock" form) and where every category
//! is held. Fullness is derived from what the bins hold, never typed.

use std::collections::{HashMap, HashSet};

use iced::widget::text::Wrapping;
use iced::widget::{
    column, container, responsive, row, scrollable, text, text_input, Column, Row, Space,
};
use iced::{Alignment, Element, Length};
use rust_decimal::Decimal;

use crate::backend::warehouse_layout::layout::{
    BinContents, Fullness, WarehouseOverview, FULLNESS_LEVELS,
};
use crate::backend::warehouse_layout::service::{Removal, WarehouseDraft, WarehouseService};
use crate::database::exports::csv;
use crate::frontend::components::badges::{self, BadgeTone};
use crate::frontend::components::button as button_style;
use crate::frontend::components::colors::Theme;
use crate::frontend::components::row_snap::{self, RowSnap};
use crate::frontend::components::table as table_style;
use crate::frontend::components::toolbar::{self, StatusFilter};
use crate::frontend::components::{
    dropdown, forms, headers, icons, labels, page_header, pagination, panels, scrollbar, search,
    spacing, states, typography,
};
use crate::frontend::views::Ctx;

/// Every field and column of a warehouse, its bins and their contents.
/// This is the only place a field's name is written: everything else
/// refers to a field by its index, through the `F_*` constants below.
const WAREHOUSE_FIELDS: [&str; 22] = [
    "Warehouse Name",
    "Company",
    "Lines",
    "Positions per Line",
    "Default Bin Capacity",
    "Code",
    "Status",
    "Utilization",
    "Category",
    "Bins Used",
    "Total Quantity",
    "Total Bins",
    "Used Bins",
    "Empty Bins",
    "Full Bins",
    "Available Capacity",
    "Categories",
    "Item",
    "Quantity",
    "Capacity",
    "Used",
    "Free",
];
const F_NAME: usize = 0;
const F_COMPANY: usize = 1;
const F_LINES: usize = 2;
const F_POSITIONS: usize = 3;
const F_DEFAULT_CAPACITY: usize = 4;
const F_CODE: usize = 5;
const F_STATUS: usize = 6;
const F_UTILIZATION: usize = 7;
const F_CATEGORY: usize = 8;
const F_BINS_USED: usize = 9;
const F_TOTAL_QUANTITY: usize = 10;
const F_TOTAL_BINS: usize = 11;
const F_USED_BINS: usize = 12;
const F_EMPTY_BINS: usize = 13;
const F_FULL_BINS: usize = 14;
const F_AVAILABLE_CAPACITY: usize = 15;
const F_CATEGORIES: usize = 16;
const F_ITEM: usize = 17;
const F_QUANTITY: usize = 18;
const F_CAPACITY: usize = 19;
const F_USED: usize = 20;
const F_FREE: usize = 21;

/// The columns of a card's category table and of a bin's contents.
const CATEGORY_COLUMNS: [usize; 3] = [F_CATEGORY, F_BINS_USED, F_TOTAL_QUANTITY];
const CONTENT_COLUMNS: [usize; 3] = [F_CATEGORY, F_ITEM, F_QUANTITY];
/// The fields written to the CSV export, in order.
const EXPORT_FIELDS: [usize; 12] = [
    F_NAME,
    F_CODE,
    F_COMPANY,
    F_LINES,
    F_POSITIONS,
    F_DEFAULT_CAPACITY,
    F_TOTAL_BINS,
    F_USED_BINS,
    F_FULL_BINS,
    F_TOTAL_QUANTITY,
    F_UTILIZATION,
    F_STATUS,
];

const TITLE: &str = "Warehouses";
const ADD_LABEL: &str = "Add Warehouse";
const ACTIVE_LABEL: &str = "Active";
const ARCHIVED_LABEL: &str = "Archived";
const ALL_LABEL: &str = "All";
const EMPTY_DASH: &str = "--";
const CARD_PAGE_SIZES: [usize; 3] = [12, 24, 48];
const EXPORT_FILE: &str = "warehouses_export.csv";
/// Lines shown side by side under one heading in the layout grid.
const LINES_PER_GROUP: u32 = 2;

/// A dropdown entry: an id (0 for "All" / "no category") and its label.
#[derive(Debug, Clone, PartialEq)]
pub struct Choice {
    id: i64,
    label: String,
}

impl std::fmt::Display for Choice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.label)
    }
}

/// The add/edit panel's inputs, kept as typed.
#[derive(Debug, Clone, Default)]
struct WarehouseForm {
    name: String,
    company: Option<Choice>,
    lines: String,
    positions: String,
    capacity: String,
}

/// The full-page view of one warehouse.
#[derive(Debug, Clone, Default)]
struct Detail {
    warehouse_id: i64,
    /// `Some(key)` highlights the bins holding that category (0 = none).
    category: Option<i64>,
    /// The bin whose side panel is open.
    bin: Option<i64>,
    /// Items of the warehouse's company, for the Place stock form.
    items: Vec<Choice>,
    place_item: Option<Choice>,
    place_quantity: String,
    place_error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct State {
    warehouses: Vec<WarehouseOverview>,
    companies: Vec<Choice>,
    default_company: i64,
    search: String,
    /// `None` shows every company.
    company_filter: Option<i64>,
    status_filter: StatusFilter,
    page: usize,
    page_size: usize,
    /// Cards showing their category table.
    expanded: HashSet<i64>,
    /// Which row of cards the grid starts at.
    scroll: RowSnap,
    form_open: bool,
    /// `Some(id)` while the panel edits an existing warehouse --
    /// `Message::Create` then saves the changes.
    editing_id: Option<i64>,
    form: WarehouseForm,
    form_error: Option<String>,
    detail: Option<Detail>,
    /// The warehouse a Delete press is waiting to have confirmed.
    pending_removal: Option<i64>,
    message: Option<String>,
    error: Option<String>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            warehouses: Vec::new(),
            companies: Vec::new(),
            default_company: 0,
            search: String::new(),
            company_filter: None,
            status_filter: StatusFilter::All,
            page: 0,
            page_size: CARD_PAGE_SIZES[0],
            expanded: HashSet::new(),
            scroll: RowSnap::default(),
            form_open: false,
            editing_id: None,
            form: WarehouseForm::default(),
            form_error: None,
            detail: None,
            pending_removal: None,
            message: None,
            error: None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    OpenForm,
    NameChanged(String),
    CompanySelected(Choice),
    LinesChanged(String),
    PositionsChanged(String),
    CapacityChanged(String),
    /// Saves the panel: adds a warehouse, or saves the one being edited.
    Create,
    EditRow(i64),
    CancelEdit,
    Remove(i64),
    ConfirmRemove,
    CancelRemove,
    SearchChanged(String),
    CompanyFilterChanged(Choice),
    StatusFilterChanged(StatusFilter),
    ResetAll,
    BulkExportCsv,
    GoToPage(usize),
    PrevPage,
    PageSizeChanged(usize),
    ToggleExpand(i64),
    /// The card grid scrolled by a row, from the wheel or its bar.
    Scrolled(row_snap::Scroll),
    OpenDetail(i64),
    CloseDetail,
    CategoryFilterChanged(Choice),
    SelectBin(i64),
    CloseBin,
    PlaceItemSelected(Choice),
    PlaceQuantityChanged(String),
    PlaceStock,
}

pub fn load(ctx: &Ctx) -> State {
    let service = WarehouseService::new(ctx.db.clone());
    let companies = service
        .list_companies()
        .map(|companies| {
            companies
                .into_iter()
                .filter_map(|company| {
                    Some(Choice {
                        id: company.id?,
                        label: company.name,
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    let mut state = State {
        companies,
        default_company: ctx.company_id,
        ..State::default()
    };
    refresh(&mut state, ctx);
    state
}

/// Reloads every warehouse with its bins and contents.
fn refresh(state: &mut State, ctx: &Ctx) {
    match WarehouseService::new(ctx.db.clone()).list_overviews() {
        Ok(warehouses) => state.warehouses = warehouses,
        Err(err) => state.error = Some(err.to_string()),
    }
}

/// Messages that change which cards the grid shows, so it starts over
/// at the first row.
fn restarts_scroll(message: &Message) -> bool {
    matches!(
        message,
        Message::SearchChanged(_)
            | Message::CompanyFilterChanged(_)
            | Message::StatusFilterChanged(_)
            | Message::ResetAll
            | Message::GoToPage(_)
            | Message::PrevPage
            | Message::PageSizeChanged(_)
    )
}

pub fn update(state: &mut State, message: Message, ctx: &Ctx) {
    if restarts_scroll(&message) {
        state.scroll.reset();
    }
    match message {
        Message::Scrolled(scroll) => state.scroll.scroll(scroll),
        Message::OpenForm => open_form(state, None),
        Message::NameChanged(value) => state.form.name = value,
        Message::CompanySelected(choice) => state.form.company = Some(choice),
        Message::LinesChanged(value) => state.form.lines = value,
        Message::PositionsChanged(value) => state.form.positions = value,
        Message::CapacityChanged(value) => state.form.capacity = value,
        Message::Create => save(state, ctx),
        Message::EditRow(id) => open_form(state, Some(id)),
        Message::CancelEdit => close_form(state),
        Message::Remove(id) => {
            state.message = None;
            state.error = None;
            state.pending_removal = Some(id);
        }
        Message::ConfirmRemove => confirm_removal(state, ctx),
        Message::CancelRemove => state.pending_removal = None,
        Message::SearchChanged(value) => {
            state.search = value;
            state.page = 0;
        }
        Message::CompanyFilterChanged(choice) => {
            state.company_filter = (choice.id != 0).then_some(choice.id);
            state.page = 0;
        }
        Message::StatusFilterChanged(filter) => {
            state.status_filter = filter;
            state.page = 0;
        }
        Message::ResetAll => {
            state.search.clear();
            state.company_filter = None;
            state.status_filter = StatusFilter::All;
            state.page = 0;
        }
        Message::BulkExportCsv => export_csv(state, ctx),
        Message::GoToPage(page) => state.page = page,
        Message::PrevPage => state.page = state.page.saturating_sub(1),
        Message::PageSizeChanged(size) => {
            state.page_size = size;
            state.page = 0;
        }
        Message::ToggleExpand(id) => {
            if !state.expanded.remove(&id) {
                state.expanded.insert(id);
            }
        }
        Message::OpenDetail(id) => open_detail(state, id, ctx),
        Message::CloseDetail => state.detail = None,
        Message::CategoryFilterChanged(choice) => {
            if let Some(detail) = &mut state.detail {
                detail.category = (choice.label != ALL_LABEL).then_some(choice.id);
            }
        }
        Message::SelectBin(bin) => {
            if let Some(detail) = &mut state.detail {
                detail.bin = Some(bin);
                detail.place_item = None;
                detail.place_quantity.clear();
                detail.place_error = None;
            }
        }
        Message::CloseBin => {
            if let Some(detail) = &mut state.detail {
                detail.bin = None;
            }
        }
        Message::PlaceItemSelected(choice) => {
            if let Some(detail) = &mut state.detail {
                detail.place_item = Some(choice);
            }
        }
        Message::PlaceQuantityChanged(value) => {
            if let Some(detail) = &mut state.detail {
                detail.place_quantity = value;
            }
        }
        Message::PlaceStock => place_stock(state, ctx),
    }
}

fn find(state: &State, id: i64) -> Option<&WarehouseOverview> {
    state.warehouses.iter().find(|o| o.warehouse.id == Some(id))
}

fn company_choice(state: &State, id: i64) -> Option<Choice> {
    state.companies.iter().find(|c| c.id == id).cloned()
}

/// Opens the panel empty (`None`) or prefilled from a warehouse.
fn open_form(state: &mut State, id: Option<i64>) {
    state.message = None;
    state.error = None;
    state.form_error = None;
    let existing = id.and_then(|id| find(state, id)).cloned();
    state.form = match &existing {
        Some(overview) => WarehouseForm {
            name: overview.warehouse.name.clone(),
            company: company_choice(state, overview.warehouse.company_id),
            lines: overview
                .layout
                .as_ref()
                .map(|l| l.lines.to_string())
                .unwrap_or_default(),
            positions: overview
                .layout
                .as_ref()
                .map(|l| l.positions.to_string())
                .unwrap_or_default(),
            capacity: overview
                .layout
                .as_ref()
                .and_then(|l| l.default_capacity)
                .map(quantity)
                .unwrap_or_default(),
        },
        None => WarehouseForm {
            company: company_choice(state, state.default_company),
            ..WarehouseForm::default()
        },
    };
    state.editing_id = existing.and_then(|o| o.warehouse.id);
    state.form_open = true;
}

fn close_form(state: &mut State) {
    state.form_open = false;
    state.editing_id = None;
    state.form = WarehouseForm::default();
    state.form_error = None;
}

/// A positive whole number, as typed.
fn parse_count(value: &str) -> Option<u32> {
    value.trim().parse().ok().filter(|n| *n > 0)
}

/// A typed quantity: thousands separators are ignored.
fn parse_quantity(value: &str) -> Option<Decimal> {
    let cleaned: String = value
        .chars()
        .filter(|c| *c != ',' && !c.is_whitespace())
        .collect();
    cleaned.parse().ok()
}

/// A quantity for display, without trailing zeros.
fn quantity(amount: Decimal) -> String {
    amount.normalize().to_string()
}

/// The panel's warehouse, or the first problem with what was typed.
fn form_draft(state: &State) -> Result<WarehouseDraft, String> {
    let form = &state.form;
    if form.name.trim().is_empty() {
        return Err(format!("{} is required.", WAREHOUSE_FIELDS[F_NAME]));
    }
    let Some(company) = &form.company else {
        return Err(format!("Choose a {}.", WAREHOUSE_FIELDS[F_COMPANY]));
    };
    let positive = |field: usize, value: &str| {
        parse_count(value).ok_or_else(|| {
            format!(
                "{} must be a whole number above zero.",
                WAREHOUSE_FIELDS[field]
            )
        })
    };
    let lines = positive(F_LINES, &form.lines)?;
    let positions = positive(F_POSITIONS, &form.positions)?;
    let default_capacity = if form.capacity.trim().is_empty() {
        None
    } else {
        match parse_quantity(&form.capacity) {
            Some(capacity) if capacity > Decimal::ZERO => Some(capacity),
            _ => {
                return Err(format!(
                    "{} must be a number above zero.",
                    WAREHOUSE_FIELDS[F_DEFAULT_CAPACITY]
                ))
            }
        }
    };
    Ok(WarehouseDraft {
        id: state.editing_id,
        company_id: company.id,
        name: form.name.trim().to_string(),
        lines,
        positions,
        default_capacity,
    })
}

fn save(state: &mut State, ctx: &Ctx) {
    state.message = None;
    state.error = None;
    state.form_error = None;
    let draft = match form_draft(state) {
        Ok(draft) => draft,
        Err(problem) => {
            state.form_error = Some(problem);
            return;
        }
    };
    match WarehouseService::new(ctx.db.clone()).save_warehouse(&draft) {
        Ok(_) => {
            refresh(state, ctx);
            close_form(state);
            state.message = Some(if draft.id.is_some() {
                "Saved changes.".into()
            } else {
                "Added warehouse.".into()
            });
        }
        Err(err) => state.form_error = Some(err.to_string()),
    }
}

/// Deletes the pending warehouse, or archives it when it holds stock.
fn confirm_removal(state: &mut State, ctx: &Ctx) {
    let Some(id) = state.pending_removal.take() else {
        return;
    };
    let Some((company_id, name)) =
        find(state, id).map(|o| (o.warehouse.company_id, o.warehouse.name.clone()))
    else {
        return;
    };
    match WarehouseService::new(ctx.db.clone()).remove_warehouse(company_id, id) {
        Ok(Removal::Deleted) => {
            if state.detail.as_ref().map(|d| d.warehouse_id) == Some(id) {
                state.detail = None;
            }
            if state.editing_id == Some(id) {
                close_form(state);
            }
            state.message = Some(format!("Deleted {name}."));
        }
        Ok(Removal::Archived) => {
            state.message = Some(format!(
                "{name} holds stock, so it was archived instead of deleted."
            ));
        }
        Err(err) => state.error = Some(err.to_string()),
    }
    refresh(state, ctx);
}

fn open_detail(state: &mut State, id: i64, ctx: &Ctx) {
    state.message = None;
    state.error = None;
    state.pending_removal = None;
    let Some(company_id) = find(state, id).map(|o| o.warehouse.company_id) else {
        return;
    };
    let items = WarehouseService::new(ctx.db.clone())
        .list_products(company_id)
        .map(|products| {
            products
                .into_iter()
                .filter_map(|p| {
                    Some(Choice {
                        id: p.id?,
                        label: format!("{} - {}", p.sku, p.name),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    state.detail = Some(Detail {
        warehouse_id: id,
        items,
        ..Detail::default()
    });
}

/// Records the Place stock form's quantity for the open bin.
fn place_stock(state: &mut State, ctx: &Ctx) {
    let Some(detail) = &mut state.detail else {
        return;
    };
    detail.place_error = None;
    let Some(bin) = detail.bin else {
        return;
    };
    let Some(item) = &detail.place_item else {
        detail.place_error = Some(format!("Choose an {}.", WAREHOUSE_FIELDS[F_ITEM]));
        return;
    };
    let Some(amount) = parse_quantity(&detail.place_quantity).filter(|q| !q.is_sign_negative())
    else {
        detail.place_error = Some(format!(
            "{} must be a number, zero or more.",
            WAREHOUSE_FIELDS[F_QUANTITY]
        ));
        return;
    };
    match WarehouseService::new(ctx.db.clone()).place_stock(bin, item.id, amount) {
        Ok(()) => {
            detail.place_item = None;
            detail.place_quantity.clear();
            refresh(state, ctx);
            state.message = Some("Placed stock.".into());
        }
        Err(err) => detail.place_error = Some(err.to_string()),
    }
}

/// Whether a warehouse passes the search box, company and status
/// filters -- shared by the cards and the CSV export.
fn matches_filters(
    overview: &WarehouseOverview,
    query: &str,
    company: Option<i64>,
    status: StatusFilter,
) -> bool {
    status.matches(overview.warehouse.is_active)
        && company.is_none_or(|id| overview.warehouse.company_id == id)
        && (query.is_empty()
            || [&overview.warehouse.name, &overview.company_name]
                .iter()
                .any(|value| value.to_lowercase().contains(query)))
}

fn filtered(state: &State) -> Vec<&WarehouseOverview> {
    let query = state.search.trim().to_lowercase();
    state
        .warehouses
        .iter()
        .filter(|o| matches_filters(o, &query, state.company_filter, state.status_filter))
        .collect()
}

fn export_csv(state: &mut State, ctx: &Ctx) {
    state.message = None;
    state.error = None;
    let rows: Vec<Vec<String>> = filtered(state)
        .iter()
        .map(|overview| {
            EXPORT_FIELDS
                .iter()
                .map(|&f| field_text(overview, f))
                .collect()
        })
        .collect();
    let headers: Vec<&str> = EXPORT_FIELDS.iter().map(|&f| WAREHOUSE_FIELDS[f]).collect();
    let path = ctx.export_path(EXPORT_FILE);
    match std::fs::write(&path, csv::export_csv(&headers, &rows)) {
        Ok(()) => {
            state.message = Some(format!(
                "Exported {} warehouse(s) to {}",
                rows.len(),
                path.display()
            ))
        }
        Err(err) => state.error = Some(err.to_string()),
    }
}

/// One field of a warehouse as plain text.
fn field_text(overview: &WarehouseOverview, field: usize) -> String {
    let layout = overview.layout.as_ref();
    match field {
        F_NAME => overview.warehouse.name.clone(),
        F_CODE => overview.warehouse.code.clone(),
        F_COMPANY => overview.company_name.clone(),
        F_LINES => layout.map(|l| l.lines.to_string()).unwrap_or_default(),
        F_POSITIONS => layout.map(|l| l.positions.to_string()).unwrap_or_default(),
        F_DEFAULT_CAPACITY => layout
            .and_then(|l| l.default_capacity)
            .map(quantity)
            .unwrap_or_default(),
        F_TOTAL_BINS => overview.total_bins().to_string(),
        F_USED_BINS => overview.used_bins().to_string(),
        F_EMPTY_BINS => overview.empty_bins().to_string(),
        F_FULL_BINS => overview.full_bins().to_string(),
        F_TOTAL_QUANTITY => quantity(overview.total_quantity()),
        F_AVAILABLE_CAPACITY => {
            if overview.total_capacity() > Decimal::ZERO {
                quantity(overview.available_capacity())
            } else {
                EMPTY_DASH.into()
            }
        }
        F_CATEGORIES => overview.category_totals().len().to_string(),
        F_UTILIZATION => utilization_text(overview),
        F_STATUS => status_label(overview.warehouse.is_active).into(),
        _ => String::new(),
    }
}

fn status_label(is_active: bool) -> &'static str {
    if is_active {
        ACTIVE_LABEL
    } else {
        ARCHIVED_LABEL
    }
}

/// `72%`, or a dash when no bin has a capacity.
fn utilization_text(overview: &WarehouseOverview) -> String {
    overview
        .utilization()
        .map(|u| format!("{}%", (u * Decimal::ONE_HUNDRED).round()))
        .unwrap_or_else(|| EMPTY_DASH.into())
}

/// Empty is the plain surface, Not Full the warning tone, Full danger.
fn fullness_tone(fullness: Fullness) -> BadgeTone {
    match fullness {
        Fullness::Empty => BadgeTone::Neutral,
        Fullness::NotFull => BadgeTone::Warning,
        Fullness::Full => BadgeTone::Negative,
    }
}

/// A page of filtered items and the numbers behind its footer.
struct Paged<T> {
    rows: Vec<T>,
    total: usize,
    pages: usize,
    page: usize,
}

fn paginate<T>(items: Vec<T>, page: usize, size: usize) -> Paged<T> {
    let total = items.len();
    let pages = total.div_ceil(size.max(1)).max(1);
    let page = page.min(pages - 1);
    let rows = items.into_iter().skip(page * size).take(size).collect();
    Paged {
        rows,
        total,
        pages,
        page,
    }
}

// -- view ------------------------------------------------------------------

pub fn view(state: &State, theme: Theme) -> Element<'_, Message> {
    layout(state, theme, None, |message| message)
}

/// Same as [`view`], except the title sits inline with `switcher` (a
/// type-switcher dropdown, e.g. "Maintain"'s section picker) -- `wrap`
/// converts this module's own `Message` into whatever the caller's
/// `switcher` produces, so the two can share one `Element` tree despite
/// having different message types.
pub fn page_header_view_with_switcher<'a, ParentMessage: Clone + 'a>(
    state: &'a State,
    theme: Theme,
    switcher: Element<'a, ParentMessage>,
    wrap: impl Fn(Message) -> ParentMessage + 'a,
) -> Element<'a, ParentMessage> {
    layout(state, theme, Some(switcher), wrap)
}

/// The header (or, on the detail page, just the switcher) above the
/// width-dependent content, which is rebuilt whenever the size changes.
fn layout<'a, M: Clone + 'a>(
    state: &'a State,
    theme: Theme,
    switcher: Option<Element<'a, M>>,
    wrap: impl Fn(Message) -> M + 'a,
) -> Element<'a, M> {
    let mut page = column![]
        .spacing(spacing::SPACE_20)
        .padding(spacing::SPACE_20)
        .width(Length::Fill)
        .height(Length::Fill);
    if state.detail.is_some() {
        if let Some(switcher) = switcher {
            page = page.push(switcher);
        }
    } else {
        page = page.push(header(theme, switcher, wrap(Message::OpenForm)));
    }
    let body: Element<'a, Message> =
        responsive(move |size| content(state, theme, size.width)).into();
    page.push(body.map(wrap)).into()
}

/// Title (with the switcher beside it) on the left, Add Warehouse on the right.
fn header<'a, M: Clone + 'a>(
    theme: Theme,
    switcher: Option<Element<'a, M>>,
    on_add: M,
) -> Element<'a, M> {
    let mut titles =
        row![labels::strong(theme, TITLE, headers::page_title_size()).wrapping(Wrapping::None)]
            .spacing(spacing::SPACE_12)
            .align_y(Alignment::Center);
    if let Some(switcher) = switcher {
        titles = titles.push(switcher);
    }
    row![
        container(titles).width(Length::Fill).clip(true),
        add_button(theme, on_add),
    ]
    .spacing(spacing::SPACE_12)
    .align_y(Alignment::Center)
    .into()
}

fn add_button<'a, M: Clone + 'a>(theme: Theme, on_press: M) -> Element<'a, M> {
    button_style::icon_label(icons::ADD, ADD_LABEL, [spacing::SPACE_10, spacing::MD])
        .style(button_style::mode(theme, true))
        .width(Length::Shrink)
        .on_press(on_press)
        .into()
}

/// The list or detail page beside (or, when narrow, replaced by) the
/// add/edit panel or the bin panel, for a content area `width` wide.
fn content(state: &State, theme: Theme, width: f32) -> Element<'_, Message> {
    let open_detail = state
        .detail
        .as_ref()
        .and_then(|detail| find(state, detail.warehouse_id).map(|overview| (detail, overview)));
    let bin_open = open_detail.is_some_and(|(detail, _)| detail.bin.is_some());
    let class = spacing::layout_class(width);
    let split = spacing::panel_split(width, state.form_open || bin_open, spacing::SPACE_20);
    let mut main = row![].spacing(spacing::SPACE_20).height(Length::Fill);
    if let Some(content_width) = split.content {
        let page = match open_detail {
            Some((detail, overview)) => detail_page(state, theme, detail, overview, content_width),
            None => warehouse_list(state, theme, content_width),
        };
        main = main.push(
            container(page)
                .width(Length::Fixed(content_width))
                .height(Length::Fill),
        );
    }
    if let Some(panel_width) = split.panel {
        let panel = match open_detail {
            Some((detail, overview)) if !state.form_open => {
                bin_panel(theme, detail, overview, class, panel_width)
            }
            _ => form_panel(state, theme, class, panel_width),
        };
        main = main.push(panel);
    }
    main.into()
}

/// Inline notices: the delete confirmation, the last result, an error.
fn notices(state: &State, theme: Theme) -> Option<Element<'_, Message>> {
    let mut items = column![].spacing(spacing::SPACE_8);
    let mut any = false;
    if let Some(overview) = state.pending_removal.and_then(|id| find(state, id)) {
        any = true;
        items = items.push(panels::padded(
            theme,
            row![
                text(format!(
                    "Delete {}? A warehouse that holds stock is archived instead.",
                    overview.warehouse.name
                ))
                .size(typography::label())
                .color(theme.warning_front())
                .width(Length::Fill),
                button_style::bold("Delete", [spacing::SPACE_6, spacing::SPACE_14])
                    .style(button_style::destructive(theme))
                    .on_press(Message::ConfirmRemove),
                button_style::bold("Keep", [spacing::SPACE_6, spacing::SPACE_14])
                    .style(button_style::outlined(theme, false))
                    .on_press(Message::CancelRemove),
            ]
            .spacing(spacing::SPACE_8)
            .align_y(Alignment::Center),
        ));
    }
    if let Some(message) = &state.message {
        any = true;
        items = items.push(text(message.clone()).color(theme.positive_front()));
    }
    if let Some(error) = &state.error {
        any = true;
        items = items.push(text(error.clone()).color(theme.negative_front()));
    }
    any.then(|| items.into())
}

/// Search, company and status filters, Reset and Export.
fn list_toolbar(state: &State, theme: Theme, class: spacing::LayoutClass) -> Element<'_, Message> {
    let all = Choice {
        id: 0,
        label: ALL_LABEL.into(),
    };
    let mut companies = vec![all.clone()];
    companies.extend(state.companies.iter().cloned());
    let selected = state
        .company_filter
        .and_then(|id| company_choice(state, id))
        .unwrap_or(all);
    let active = state
        .warehouses
        .iter()
        .filter(|o| o.warehouse.is_active)
        .count();
    let status_options = vec![
        (
            ALL_LABEL,
            state.warehouses.len(),
            Message::StatusFilterChanged(StatusFilter::All),
            state.status_filter == StatusFilter::All,
        ),
        (
            ACTIVE_LABEL,
            active,
            Message::StatusFilterChanged(StatusFilter::Active),
            state.status_filter == StatusFilter::Active,
        ),
        (
            ARCHIVED_LABEL,
            state.warehouses.len() - active,
            Message::StatusFilterChanged(StatusFilter::Inactive),
            state.status_filter == StatusFilter::Inactive,
        ),
    ];
    let filters = row![
        dropdown::filter(
            theme,
            WAREHOUSE_FIELDS[F_COMPANY],
            companies,
            Some(selected),
            Message::CompanyFilterChanged,
        ),
        dropdown::counted_filter(theme, WAREHOUSE_FIELDS[F_STATUS], status_options),
    ]
    .spacing(spacing::SPACE_12)
    .align_y(Alignment::Center);
    let tools = row![
        button_style::icon_tool(theme, icons::RESET, "Reset", Message::ResetAll, true),
        button_style::icon_tool(
            theme,
            icons::EXPORT,
            "Export CSV",
            Message::BulkExportCsv,
            true
        ),
    ]
    .spacing(spacing::SPACE_12)
    .align_y(Alignment::Center);
    toolbar::filter_bar(
        class,
        search::input(
            theme,
            "Search warehouse or company",
            &state.search,
            Message::SearchChanged,
        ),
        filters.into(),
        tools.into(),
    )
}

/// Toolbar, notices, the card grid (or an empty state) and its pager.
fn warehouse_list(state: &State, theme: Theme, width: f32) -> Element<'_, Message> {
    let class = spacing::layout_class(width);
    let cards = paginate(filtered(state), state.page, state.page_size);
    let mut list = column![list_toolbar(state, theme, class)]
        .spacing(spacing::SPACE_20)
        .height(Length::Fill);
    if let Some(notices) = notices(state, theme) {
        list = list.push(notices);
    }
    if state.warehouses.is_empty() {
        return list
            .push(panels::empty_state(
                theme,
                icons::WAREHOUSE,
                "No warehouses yet",
                Some("Add a warehouse and lay out its bins to start placing stock."),
                Some(add_button(theme, Message::OpenForm)),
            ))
            .into();
    }
    if cards.rows.is_empty() {
        return list
            .push(panels::empty_state(
                theme,
                icons::SEARCH,
                "No warehouses match these filters",
                None,
                Some(
                    button_style::bold("Reset", [spacing::SPACE_6, spacing::SPACE_14])
                        .style(button_style::outlined(theme, false))
                        .on_press(Message::ResetAll)
                        .into(),
                ),
            ))
            .into();
    }
    let shown = cards.rows.len();
    let rows = cards.rows;
    let (page, pages, total, page_size) = (cards.page, cards.pages, cards.total, state.page_size);
    list = list.push(responsive(move |size| {
        let reserve = if pages > 1 {
            pagination::footer_height(class)
        } else {
            0.0
        };
        let mut area = column![card_grid(
            state,
            theme,
            &rows,
            spacing::grid_columns(class),
            size.height - reserve,
        )];
        if pages > 1 {
            area = area.push(pagination::footer(pagination::Footer {
                theme,
                class,
                page,
                pages,
                shown,
                total,
                page_size,
                page_sizes: &CARD_PAGE_SIZES,
                on_previous: Message::PrevPage,
                on_page: Message::GoToPage,
                on_page_size: Message::PageSizeChanged,
            }));
        }
        area.into()
    }));
    list.into()
}

/// The cards in `height` pixels, scrolling by whole rows.
fn card_grid<'a>(
    state: &'a State,
    theme: Theme,
    rows: &[&'a WarehouseOverview],
    columns: usize,
    height: f32,
) -> Element<'a, Message> {
    row_snap::view(
        &state.scroll,
        theme,
        rows.iter()
            .map(|overview| {
                let expanded = state
                    .expanded
                    .contains(&overview.warehouse.id.unwrap_or_default());
                warehouse_card(theme, overview, expanded)
            })
            .collect(),
        columns,
        height,
        Message::Scrolled,
    )
}

/// The fullness pill, plus an Archived pill for an inactive warehouse.
fn warehouse_badges(theme: Theme, overview: &WarehouseOverview) -> Vec<Element<'static, Message>> {
    let fullness = overview.fullness();
    let mut pills = vec![badges::status_pill(
        theme,
        fullness.label(),
        fullness_tone(fullness),
    )];
    if !overview.warehouse.is_active {
        pills.push(badges::status_pill(
            theme,
            ARCHIVED_LABEL,
            BadgeTone::Neutral,
        ));
    }
    pills
}

/// Utilization label, percent and bar.
fn utilization(theme: Theme, overview: &WarehouseOverview) -> Element<'static, Message> {
    let fraction = overview
        .utilization()
        .and_then(|u| u.to_string().parse::<f32>().ok())
        .unwrap_or(0.0);
    column![
        row![
            labels::muted(
                theme,
                WAREHOUSE_FIELDS[F_UTILIZATION],
                typography::caption()
            )
            .width(Length::Fill),
            labels::strong(theme, utilization_text(overview), typography::caption()),
        ],
        panels::progress_bar(theme, fraction, fullness_tone(overview.fullness())),
    ]
    .spacing(spacing::SPACE_4)
    .into()
}

/// One category row per category: name, bins holding it, total.
fn category_table(theme: Theme, overview: &WarehouseOverview) -> Element<'static, Message> {
    let totals = overview.category_totals();
    if totals.is_empty() {
        return labels::muted(theme, "No stock placed yet.", typography::body()).into();
    }
    let headers: Vec<&str> = CATEGORY_COLUMNS
        .iter()
        .map(|&f| WAREHOUSE_FIELDS[f])
        .collect();
    table_style::grid(
        theme,
        &headers,
        &[3, 1, 1],
        &[1, 2],
        totals
            .into_iter()
            .map(|t| vec![t.category, t.bins.len().to_string(), quantity(t.quantity)])
            .collect(),
        None,
    )
}

/// One expandable warehouse card: the name (which opens the detail),
/// company and status, fullness, utilization, Edit/Delete and the
/// chevron that shows the category table.
fn warehouse_card(
    theme: Theme,
    overview: &WarehouseOverview,
    expanded: bool,
) -> Element<'static, Message> {
    let id = overview.warehouse.id.unwrap_or_default();
    let tone = fullness_tone(overview.fullness());
    let status_tone = if overview.warehouse.is_active {
        BadgeTone::Positive
    } else {
        BadgeTone::Neutral
    };
    let (chevron, tip) = if expanded {
        (icons::TREE_EXPANDED, "Hide categories")
    } else {
        (icons::TREE_COLLAPSED, "Show categories")
    };
    let title = iced::widget::button(labels::strong(
        theme,
        overview.warehouse.name.clone(),
        typography::panel_title(),
    ))
    .padding(0)
    .style(button_style::bare(theme))
    .on_press(Message::OpenDetail(id));
    let heading = row![
        badges::icon_box(theme, icons::WAREHOUSE, tone),
        column![
            title,
            row![
                labels::muted(theme, overview.company_name.clone(), typography::caption()),
                badges::status_dot(
                    theme,
                    status_label(overview.warehouse.is_active),
                    status_tone
                ),
            ]
            .spacing(spacing::SPACE_8)
            .align_y(Alignment::Center),
        ]
        .spacing(spacing::SPACE_2)
        .width(Length::Fill),
        row(warehouse_badges(theme, overview)).spacing(spacing::SPACE_4),
        button_style::icon_tool(theme, chevron, tip, Message::ToggleExpand(id), false),
    ]
    .spacing(spacing::SPACE_12)
    .align_y(Alignment::Center);
    let actions = row![
        labels::muted(
            theme,
            format!(
                "{} {}  ·  {} {}",
                overview.total_bins(),
                WAREHOUSE_FIELDS[F_TOTAL_BINS],
                quantity(overview.total_quantity()),
                WAREHOUSE_FIELDS[F_TOTAL_QUANTITY]
            ),
            typography::caption()
        )
        .width(Length::Fill),
        button_style::icon_tool(theme, icons::EDIT, "Edit", Message::EditRow(id), false),
        button_style::icon_tool(theme, icons::REMOVE, "Delete", Message::Remove(id), false),
    ]
    .spacing(spacing::SPACE_4)
    .align_y(Alignment::Center);
    let mut card = column![heading, utilization(theme, overview)].spacing(spacing::SPACE_12);
    if expanded {
        // The card keeps its grid height; a long table scrolls inside it.
        card = card.push(table_style::divider_line(theme));
        card = card.push(
            scrollable(container(category_table(theme, overview)).padding(scrollbar::gutter()))
                .direction(scrollbar::vertical())
                .height(Length::Fill),
        );
    } else {
        card = card.push(Space::with_height(Length::Fill));
    }
    container(card.push(actions))
        .padding(spacing::MD)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(panels::card(theme, states::RADIUS_LG))
        .into()
}

/// The add/edit panel, with a Back button when it replaces the page.
fn form_panel(
    state: &State,
    theme: Theme,
    class: spacing::LayoutClass,
    width: f32,
) -> Element<'_, Message> {
    let mut side = column![]
        .spacing(spacing::SPACE_20)
        .width(Length::Fixed(width))
        .height(Length::Fill);
    if class == spacing::LayoutClass::Narrow {
        side = side.push(button_style::back(theme, Message::CancelEdit));
    }
    side.push(warehouse_form(state, theme)).into()
}

/// A labelled text input for one of `WAREHOUSE_FIELDS`.
fn input_field<'a>(
    theme: Theme,
    field: usize,
    value: &'a str,
    on_input: fn(String) -> Message,
) -> Element<'a, Message> {
    let label = WAREHOUSE_FIELDS[field];
    forms::text_field(theme, label, text_input(label, value).on_input(on_input))
}

fn warehouse_form(state: &State, theme: Theme) -> Element<'_, Message> {
    let form = &state.form;
    // The company of an existing warehouse cannot change: its stock and
    // items belong to that company.
    let company: Element<'_, Message> = if state.editing_id.is_some() {
        forms::readonly_field(
            theme,
            WAREHOUSE_FIELDS[F_COMPANY],
            form.company
                .as_ref()
                .map(|c| c.label.clone())
                .unwrap_or_default(),
        )
    } else {
        forms::field_row(
            theme,
            WAREHOUSE_FIELDS[F_COMPANY],
            dropdown::form_select(
                theme,
                state.companies.as_slice(),
                form.company.clone(),
                Message::CompanySelected,
            )
            .placeholder(WAREHOUSE_FIELDS[F_COMPANY])
            .into(),
        )
    };
    let fields = vec![
        input_field(theme, F_NAME, &form.name, Message::NameChanged),
        company,
        input_field(theme, F_LINES, &form.lines, Message::LinesChanged),
        input_field(
            theme,
            F_POSITIONS,
            &form.positions,
            Message::PositionsChanged,
        ),
        input_field(
            theme,
            F_DEFAULT_CAPACITY,
            &form.capacity,
            Message::CapacityChanged,
        ),
    ];
    let title = if state.editing_id.is_some() {
        "Edit Warehouse"
    } else {
        ADD_LABEL
    };
    let mut body = column![
        labels::strong(theme, title, typography::panel_title()),
        labels::muted(
            theme,
            "Saving lays out one bin per line and position. Bins that hold stock are never removed.",
            typography::caption()
        ),
        scrollable(container(forms::field_grid(fields, 1)).padding(scrollbar::gutter()))
            .direction(scrollbar::vertical())
            .height(Length::Fill),
    ]
    .spacing(spacing::MD);
    if let Some(error) = &state.form_error {
        body = body.push(text(error.clone()).color(theme.negative_front()));
    }
    body = body.push(
        row![
            button_style::bold("Save", [spacing::SM, spacing::MD])
                .style(button_style::mode(theme, true))
                .on_press(Message::Create),
            button_style::bold("Cancel", [spacing::SM, spacing::MD])
                .style(button_style::outlined(theme, false))
                .on_press(Message::CancelEdit),
        ]
        .spacing(spacing::SM),
    );
    panels::side(theme, body)
}

// -- detail page -----------------------------------------------------------

/// The category filter's key for an item's category (0 = none).
fn category_key(category_id: Option<i64>) -> i64 {
    category_id.unwrap_or(0)
}

fn bin_matches(contents: &BinContents, category: Option<i64>) -> bool {
    category.is_none_or(|key| {
        contents
            .items
            .iter()
            .any(|item| category_key(item.category_id) == key)
    })
}

/// Back/Edit/Delete, the summary column and the layout with its
/// category list.
fn detail_page<'a>(
    state: &'a State,
    theme: Theme,
    detail: &'a Detail,
    overview: &'a WarehouseOverview,
    width: f32,
) -> Element<'a, Message> {
    let id = overview.warehouse.id.unwrap_or_default();
    let actions = vec![
        button_style::icon_label(icons::EDIT, "Edit", [spacing::SPACE_6, spacing::SPACE_14])
            .style(button_style::mode(theme, true))
            .on_press(Message::EditRow(id))
            .into(),
        button_style::icon_tool(theme, icons::REMOVE, "Delete", Message::Remove(id), true),
    ];
    let mut page = column![page_header::detail(
        theme,
        overview.warehouse.name.as_str(),
        format!("{}  ·  {}", overview.warehouse.code, overview.company_name),
        warehouse_badges(theme, overview),
        Message::CloseDetail,
        actions,
    )]
    .spacing(spacing::MD);
    if let Some(notices) = notices(state, theme) {
        page = page.push(notices);
    }
    let narrow = spacing::layout_class(width) == spacing::LayoutClass::Narrow;
    let main_width = if narrow {
        width
    } else {
        width - spacing::SUMMARY_PANEL_WIDTH - spacing::SPACE_20
    };
    let main = column![
        layout_card(theme, detail, overview, main_width),
        category_locations(theme, overview, detail.category),
    ]
    .spacing(spacing::MD)
    .width(Length::Fill);
    let summary = container(summary_panel(theme, overview, detail.category));
    let body: Element<'a, Message> = if narrow {
        column![summary.width(Length::Fill), main]
            .spacing(spacing::MD)
            .into()
    } else {
        row![
            summary.width(Length::Fixed(spacing::SUMMARY_PANEL_WIDTH)),
            main
        ]
        .spacing(spacing::SPACE_20)
        .into()
    };
    page = page.push(
        scrollable(container(body).padding(scrollbar::gutter()))
            .direction(scrollbar::vertical())
            .height(Length::Fill),
    );
    page.into()
}

/// The label/value summary, with the filtered category's total.
fn summary_panel(
    theme: Theme,
    overview: &WarehouseOverview,
    category: Option<i64>,
) -> Element<'static, Message> {
    let mut lines = Column::new().spacing(spacing::SPACE_10);
    for field in [
        F_TOTAL_BINS,
        F_USED_BINS,
        F_EMPTY_BINS,
        F_FULL_BINS,
        F_TOTAL_QUANTITY,
        F_AVAILABLE_CAPACITY,
        F_CATEGORIES,
        F_COMPANY,
    ] {
        lines = lines.push(labels::pair(
            theme,
            WAREHOUSE_FIELDS[field],
            field_text(overview, field),
        ));
    }
    if let Some(key) = category {
        let total = overview
            .category_totals()
            .into_iter()
            .find(|t| category_key(t.category_id) == key);
        if let Some(total) = total {
            lines = lines.push(table_style::divider_line(theme));
            lines = lines.push(labels::pair(
                theme,
                format!(
                    "{} · {}",
                    WAREHOUSE_FIELDS[F_TOTAL_QUANTITY], total.category
                ),
                quantity(total.quantity),
            ));
        }
    }
    panels::titled_card(theme, "Warehouse Summary", lines)
}

/// The category filter's options: All, then every category held here.
fn category_options(overview: &WarehouseOverview) -> Vec<Choice> {
    let mut options = vec![Choice {
        id: 0,
        label: ALL_LABEL.into(),
    }];
    options.extend(overview.category_totals().into_iter().map(|t| Choice {
        id: category_key(t.category_id),
        label: t.category,
    }));
    options
}

/// Title, legend and category filter over the bin grid.
fn layout_card<'a>(
    theme: Theme,
    detail: &'a Detail,
    overview: &'a WarehouseOverview,
    width: f32,
) -> Element<'a, Message> {
    let options = category_options(overview);
    let selected = detail
        .category
        .and_then(|key| options.iter().skip(1).find(|o| o.id == key).cloned())
        .or_else(|| options.first().cloned());
    let mut legend = Row::new().spacing(spacing::MD).align_y(Alignment::Center);
    for level in FULLNESS_LEVELS {
        legend = legend.push(badges::legend(theme, fullness_tone(level), level.label()));
    }
    let top = row![
        labels::strong(theme, "Warehouse layout", typography::panel_title())
            .wrapping(Wrapping::None)
            .width(Length::Fill),
        dropdown::filter(
            theme,
            WAREHOUSE_FIELDS[F_CATEGORY],
            options,
            selected,
            Message::CategoryFilterChanged,
        ),
    ]
    .spacing(spacing::SPACE_12)
    .align_y(Alignment::Center);
    let grid: Element<'a, Message> = match &overview.layout {
        Some(layout) if !overview.bins.is_empty() => bin_grid(
            theme,
            detail,
            overview,
            layout.lines,
            layout.positions,
            width,
        ),
        _ => panels::empty_state(
            theme,
            icons::LAYOUT_GRID,
            "No bin layout yet",
            Some("Edit the warehouse and set its lines and positions to lay out the bins."),
            Some(
                button_style::bold("Edit", [spacing::SPACE_6, spacing::SPACE_14])
                    .style(button_style::outlined(theme, false))
                    .on_press(Message::EditRow(overview.warehouse.id.unwrap_or_default()))
                    .into(),
            ),
        ),
    };
    panels::padded(
        theme,
        column![top, legend.wrap(), grid].spacing(spacing::MD),
    )
}

/// Width of one line group: its cells and the gaps between them.
fn group_width() -> f32 {
    spacing::grid_cell_size() * LINES_PER_GROUP as f32
        + spacing::SPACE_4 * (LINES_PER_GROUP - 1) as f32
}

/// The bins in groups of two lines ("L: 1-2", "L: 3-4", ...), each a
/// column of numbered positions; groups wrap to the width, and a narrow
/// page scrolls them sideways instead.
fn bin_grid<'a>(
    theme: Theme,
    detail: &'a Detail,
    overview: &'a WarehouseOverview,
    lines: u32,
    positions: u32,
    width: f32,
) -> Element<'a, Message> {
    let cells: HashMap<(u32, u32), &BinContents> = overview
        .bins
        .iter()
        .map(|contents| ((contents.bin.line, contents.bin.position), contents))
        .collect();
    let mut groups: Vec<Element<'a, Message>> = Vec::new();
    for first in (1..=lines).step_by(LINES_PER_GROUP as usize) {
        let last = (first + LINES_PER_GROUP - 1).min(lines);
        let heading = if first == last {
            format!("L: {first}")
        } else {
            format!("L: {first}-{last}")
        };
        let mut group = Column::new()
            .spacing(spacing::SPACE_4)
            .push(labels::strong(theme, heading, typography::caption()).wrapping(Wrapping::None));
        for position in 1..=positions {
            let mut line = Row::new().spacing(spacing::SPACE_4);
            for number in first..=last {
                line = line.push(match cells.get(&(number, position)) {
                    Some(contents) => bin_cell(theme, detail, contents),
                    None => Space::new(
                        Length::Fixed(spacing::grid_cell_size()),
                        Length::Fixed(spacing::grid_cell_size()),
                    )
                    .into(),
                });
            }
            group = group.push(line);
        }
        groups.push(group.into());
    }
    let gap = spacing::MD;
    if spacing::layout_class(width) == spacing::LayoutClass::Narrow {
        return scrollable(container(Row::with_children(groups).spacing(gap)).padding(
            iced::Padding {
                bottom: spacing::SCROLLBAR_WIDTH,
                ..iced::Padding::ZERO
            },
        ))
        .direction(scrollable::Direction::Horizontal(scrollbar::bar()))
        .width(Length::Fill)
        .into();
    }
    let per_row =
        (((width - spacing::MD * 2.0 + gap) / (group_width() + gap)).floor() as usize).max(1);
    let mut rows = Column::new().spacing(spacing::MD);
    let mut groups = groups.into_iter().peekable();
    while groups.peek().is_some() {
        rows = rows.push(Row::with_children(groups.by_ref().take(per_row)).spacing(gap));
    }
    rows.into()
}

/// One bin: its position number, colored by fullness, dimmed when the
/// category filter excludes it.
fn bin_cell<'a>(theme: Theme, detail: &Detail, contents: &BinContents) -> Element<'a, Message> {
    let id = contents.bin.id.unwrap_or_default();
    button_style::grid_cell(
        theme,
        contents.bin.position.to_string(),
        fullness_tone(contents.fullness()),
        !bin_matches(contents, detail.category),
        detail.bin == Some(id),
        Message::SelectBin(id),
    )
}

/// "Where is this category?": each category's total and its bins.
fn category_locations(
    theme: Theme,
    overview: &WarehouseOverview,
    category: Option<i64>,
) -> Element<'static, Message> {
    let mut lines = Column::new().spacing(spacing::SPACE_8);
    let totals = overview.category_totals();
    if totals.is_empty() {
        lines = lines.push(labels::muted(
            theme,
            "No stock placed yet.",
            typography::body(),
        ));
    }
    for total in totals {
        let chosen = category.is_some_and(|key| key == category_key(total.category_id));
        let line = text(format!(
            "{}: {} in {}",
            total.category,
            quantity(total.quantity),
            total.bins.join(", ")
        ))
        .size(typography::body());
        lines = lines.push(if chosen || category.is_none() {
            line.color(theme.neutral_font())
        } else {
            line.color(theme.neutral_front())
        });
    }
    panels::titled_card(theme, "Where is this category?", lines)
}

/// The open bin: capacity, used and free, its contents by category and
/// the Place stock form, with Close (or Back when it replaces the page).
fn bin_panel<'a>(
    theme: Theme,
    detail: &'a Detail,
    overview: &'a WarehouseOverview,
    class: spacing::LayoutClass,
    width: f32,
) -> Element<'a, Message> {
    let mut side = column![]
        .spacing(spacing::SPACE_20)
        .width(Length::Fixed(width))
        .height(Length::Fill);
    if class == spacing::LayoutClass::Narrow {
        side = side.push(button_style::back(theme, Message::CloseBin));
    }
    let Some(contents) = detail
        .bin
        .and_then(|id| overview.bins.iter().find(|b| b.bin.id == Some(id)))
    else {
        return side.into();
    };
    let dash_or = |value: Option<Decimal>| value.map(quantity).unwrap_or_else(|| EMPTY_DASH.into());
    let fullness = contents.fullness();
    let mut body = column![
        row![
            labels::strong(
                theme,
                format!("Bin {}", contents.bin.label),
                typography::panel_title()
            )
            .width(Length::Fill),
            badges::status_pill(theme, fullness.label(), fullness_tone(fullness)),
            button_style::icon_tool(theme, icons::CLOSE, "Close", Message::CloseBin, false),
        ]
        .spacing(spacing::SPACE_8)
        .align_y(Alignment::Center),
        labels::pair(
            theme,
            WAREHOUSE_FIELDS[F_CAPACITY],
            dash_or(contents.bin.capacity)
        ),
        labels::pair(theme, WAREHOUSE_FIELDS[F_USED], quantity(contents.used())),
        labels::pair(theme, WAREHOUSE_FIELDS[F_FREE], dash_or(contents.free())),
        table_style::divider_line(theme),
    ]
    .spacing(spacing::SPACE_10);
    if contents.items.is_empty() {
        body = body.push(labels::muted(
            theme,
            Fullness::Empty.label(),
            typography::body(),
        ));
    } else {
        let headers: Vec<&str> = CONTENT_COLUMNS
            .iter()
            .map(|&f| WAREHOUSE_FIELDS[f])
            .collect();
        let mut rows = Vec::new();
        for (category, items) in contents.by_category() {
            for item in items {
                rows.push(vec![
                    category.to_string(),
                    item.product_name.clone(),
                    quantity(item.quantity),
                ]);
            }
        }
        body = body.push(table_style::grid(
            theme,
            &headers,
            &[2, 3, 1],
            &[2],
            rows,
            None,
        ));
    }
    body = body
        .push(table_style::divider_line(theme))
        .push(place_form(theme, detail));
    side.push(panels::side(
        theme,
        scrollable(container(body).padding(scrollbar::gutter()))
            .direction(scrollbar::vertical())
            .height(Length::Fill),
    ))
    .into()
}

/// Item and quantity for the open bin; the quantity replaces what the
/// bin holds of that item, and zero removes it.
fn place_form<'a>(theme: Theme, detail: &'a Detail) -> Element<'a, Message> {
    let mut form = column![
        labels::strong(theme, "Place stock", typography::label()),
        labels::muted(
            theme,
            "Sets how much of the item this bin holds; 0 removes it.",
            typography::caption()
        ),
        forms::field_row(
            theme,
            WAREHOUSE_FIELDS[F_ITEM],
            dropdown::form_select(
                theme,
                detail.items.as_slice(),
                detail.place_item.clone(),
                Message::PlaceItemSelected,
            )
            .placeholder(WAREHOUSE_FIELDS[F_ITEM])
            .into(),
        ),
        input_field(
            theme,
            F_QUANTITY,
            &detail.place_quantity,
            Message::PlaceQuantityChanged
        ),
    ]
    .spacing(spacing::SPACE_8);
    if let Some(error) = &detail.place_error {
        form = form.push(text(error.clone()).color(theme.negative_front()));
    }
    form.push(
        button_style::icon_label(icons::PACKAGE, "Save", [spacing::SM, spacing::MD])
            .style(button_style::mode(theme, true))
            .on_press(Message::PlaceStock),
    )
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::inventory::categories::Category;
    use crate::backend::inventory::products::Product;
    use crate::backend::inventory::warehouses::Warehouse;
    use crate::backend::organization::companies::Company;
    use crate::backend::repositories::inventory_repository::{self, InventoryRepository};
    use crate::backend::repositories::organization_repository::{self, OrganizationRepository};
    use crate::backend::warehouse_layout::layout::{bin_label, Bin, BinItem, WarehouseLayout};
    use crate::backend::warehouse_layout::repository;
    use crate::database::database::Database;
    use rust_decimal_macros::dec;
    use std::path::PathBuf;

    const WIDTHS: [f32; 4] = [640.0, 900.0, 1366.0, 1920.0];
    const THEMES: [Theme; 2] = [Theme::Light, Theme::Dark];

    fn ctx_with_company() -> Ctx {
        let db = Database::open_in_memory().unwrap();
        db.run_migrations(organization_repository::MIGRATIONS)
            .unwrap();
        db.run_migrations(inventory_repository::MIGRATIONS).unwrap();
        db.run_migrations(repository::MIGRATIONS).unwrap();

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
                "memonbooks_warehouses_view_test_{:?}",
                std::thread::current().id()
            )),
            ..ctx
        }
    }

    /// Adds a warehouse through the panel, the way a user would.
    fn add(state: &mut State, ctx: &Ctx, name: &str, lines: &str, positions: &str) -> i64 {
        update(state, Message::OpenForm, ctx);
        update(state, Message::NameChanged(name.into()), ctx);
        update(state, Message::LinesChanged(lines.into()), ctx);
        update(state, Message::PositionsChanged(positions.into()), ctx);
        update(state, Message::CapacityChanged("10".into()), ctx);
        update(state, Message::Create, ctx);
        assert!(state.form_error.is_none(), "{:?}", state.form_error);
        state
            .warehouses
            .iter()
            .find(|o| o.warehouse.name == name)
            .unwrap()
            .warehouse
            .id
            .unwrap()
    }

    fn product(ctx: &Ctx, sku: &str, category: &str) -> i64 {
        let inventory = InventoryRepository::new(ctx.db.clone());
        let category_id = inventory
            .create_category(&Category::new(ctx.company_id, category))
            .unwrap()
            .id;
        let mut product = Product::new(ctx.company_id, sku, format!("Item {sku}"));
        product.category_id = category_id;
        inventory.create_product(&product).unwrap().id.unwrap()
    }

    /// A warehouse with `lines` x `positions` bins, every `step`-th one
    /// holding stock (capacity 10; some full).
    fn overview(
        id: i64,
        name: &str,
        lines: u32,
        positions: u32,
        step: usize,
        active: bool,
    ) -> WarehouseOverview {
        let mut bins = Vec::new();
        let mut index = 0;
        for line in 1..=lines {
            for position in 1..=positions {
                index += 1;
                let items = if step > 0 && index % step == 0 {
                    vec![BinItem {
                        product_id: 1,
                        product_name: "Television".into(),
                        category_id: Some((index % 3) as i64 + 1),
                        category: format!("Category {}", index % 3 + 1),
                        quantity: Decimal::from((index % 11) as i64 + 1),
                    }]
                } else {
                    Vec::new()
                };
                bins.push(BinContents {
                    bin: Bin {
                        id: Some(id * 10_000 + index as i64),
                        warehouse_id: id,
                        line,
                        position,
                        label: bin_label(line, position),
                        capacity: Some(dec!(10)),
                    },
                    items,
                });
            }
        }
        let mut warehouse = Warehouse::new(1, name, format!("WH-{id}"));
        warehouse.id = Some(id);
        warehouse.is_active = active;
        WarehouseOverview {
            warehouse,
            company_name: "Acme Trading Co".into(),
            layout: (lines > 0).then(|| WarehouseLayout::new(id, lines, positions, Some(dec!(10)))),
            bins,
        }
    }

    fn state_with(count: usize) -> State {
        State {
            warehouses: (0..count)
                .map(|i| {
                    let name = if i == 0 {
                        "Karachi Port Qasim Central Distribution Warehouse, Section B".to_string()
                    } else {
                        format!("Warehouse {i}")
                    };
                    overview(
                        i as i64 + 1,
                        &name,
                        (i % 4) as u32 * 3,
                        6,
                        i % 3,
                        i % 5 != 4,
                    )
                })
                .collect(),
            companies: vec![Choice {
                id: 1,
                label: "Acme Trading Co".into(),
            }],
            default_company: 1,
            expanded: HashSet::from([1, 2]),
            ..State::default()
        }
    }

    fn with_detail(mut state: State, id: i64, detail: Detail) -> State {
        state
            .warehouses
            .push(overview(id, "Detail", 24, 17, 2, true));
        state.detail = Some(Detail {
            warehouse_id: id,
            ..detail
        });
        state
    }

    /// The page (header and width-dependent content) at every test width
    /// and theme.
    fn build_everywhere(state: &State) {
        for theme in THEMES {
            let _ = view(state, theme);
            let switcher: Element<'_, Message> = text("switcher").into();
            let _ = page_header_view_with_switcher(state, theme, switcher, |m| m);
            for width in WIDTHS {
                let _ = content(state, theme, width);
            }
        }
    }

    #[test]
    fn field_changed_messages_update_the_corresponding_field() {
        let ctx = ctx_with_company();
        let mut state = load(&ctx);

        update(&mut state, Message::OpenForm, &ctx);
        update(
            &mut state,
            Message::NameChanged("Main Warehouse".into()),
            &ctx,
        );
        update(&mut state, Message::LinesChanged("4".into()), &ctx);
        update(&mut state, Message::PositionsChanged("12".into()), &ctx);
        update(&mut state, Message::CapacityChanged("50".into()), &ctx);

        assert_eq!(state.form.name, "Main Warehouse");
        assert_eq!(state.form.lines, "4");
        assert_eq!(state.form.positions, "12");
        assert_eq!(state.form.capacity, "50");
        assert_eq!(
            state.form.company.as_ref().map(|c| c.id),
            Some(ctx.company_id)
        );
    }

    #[test]
    fn creating_with_a_blank_name_is_rejected_before_reaching_the_service() {
        let ctx = ctx_with_company();
        let mut state = load(&ctx);
        update(&mut state, Message::OpenForm, &ctx);
        update(&mut state, Message::LinesChanged("2".into()), &ctx);
        update(&mut state, Message::PositionsChanged("2".into()), &ctx);

        update(&mut state, Message::Create, &ctx);

        assert_eq!(
            state.form_error,
            Some(format!("{} is required.", WAREHOUSE_FIELDS[F_NAME]))
        );
        assert!(state.warehouses.is_empty());
    }

    #[test]
    fn lines_positions_and_capacity_must_be_positive_numbers() {
        let ctx = ctx_with_company();
        let mut state = load(&ctx);
        update(&mut state, Message::OpenForm, &ctx);
        update(&mut state, Message::NameChanged("Main".into()), &ctx);
        for (lines, positions, capacity, field) in [
            ("0", "2", "", F_LINES),
            ("2", "x", "", F_POSITIONS),
            ("2", "2", "-5", F_DEFAULT_CAPACITY),
        ] {
            update(&mut state, Message::LinesChanged(lines.into()), &ctx);
            update(
                &mut state,
                Message::PositionsChanged(positions.into()),
                &ctx,
            );
            update(&mut state, Message::CapacityChanged(capacity.into()), &ctx);
            update(&mut state, Message::Create, &ctx);
            assert!(state
                .form_error
                .as_deref()
                .unwrap_or_default()
                .starts_with(WAREHOUSE_FIELDS[field]));
        }
        assert!(state.warehouses.is_empty());
        assert!(state.form_open);
    }

    #[test]
    fn creating_a_valid_warehouse_lays_out_its_bins_and_closes_the_form() {
        let ctx = ctx_with_company();
        let mut state = load(&ctx);

        let id = add(&mut state, &ctx, "Main Warehouse", "3", "4");

        assert!(state.error.is_none());
        assert_eq!(state.warehouses.len(), 1);
        let created = find(&state, id).unwrap();
        assert_eq!(created.total_bins(), 12);
        assert_eq!(created.company_name, "Acme Trading Co");
        assert!(!state.form_open);
        assert!(state.form.name.is_empty());
    }

    #[test]
    fn a_duplicate_name_in_the_same_company_is_reported_in_the_panel() {
        let ctx = ctx_with_company();
        let mut state = load(&ctx);
        add(&mut state, &ctx, "Main Warehouse", "1", "1");

        update(&mut state, Message::OpenForm, &ctx);
        update(
            &mut state,
            Message::NameChanged("main warehouse".into()),
            &ctx,
        );
        update(&mut state, Message::LinesChanged("1".into()), &ctx);
        update(&mut state, Message::PositionsChanged("1".into()), &ctx);
        update(&mut state, Message::Create, &ctx);

        assert!(state
            .form_error
            .as_deref()
            .unwrap_or_default()
            .contains("already exists"));
        assert_eq!(state.warehouses.len(), 1);
    }

    #[test]
    fn search_changed_updates_the_search_field() {
        let ctx = ctx_with_company();
        let mut state = State {
            page: 2,
            ..State::default()
        };

        update(&mut state, Message::SearchChanged("main".into()), &ctx);

        assert_eq!(state.search, "main");
        assert_eq!(state.page, 0);
    }

    #[test]
    fn edit_row_prefills_the_form_from_the_existing_warehouse() {
        let ctx = ctx_with_company();
        let mut state = load(&ctx);
        let id = add(&mut state, &ctx, "Main Warehouse", "3", "4");

        update(&mut state, Message::EditRow(id), &ctx);

        assert_eq!(state.editing_id, Some(id));
        assert_eq!(state.form.name, "Main Warehouse");
        assert_eq!(state.form.lines, "3");
        assert_eq!(state.form.positions, "4");
        assert_eq!(state.form.capacity, "10");
    }

    #[test]
    fn saving_an_edit_updates_the_existing_warehouse_in_place_and_clears_editing_state() {
        let ctx = ctx_with_company();
        let mut state = load(&ctx);
        let id = add(&mut state, &ctx, "Main Warehouse", "2", "2");

        update(&mut state, Message::EditRow(id), &ctx);
        update(
            &mut state,
            Message::NameChanged("Overflow Warehouse".into()),
            &ctx,
        );
        update(&mut state, Message::LinesChanged("3".into()), &ctx);
        update(&mut state, Message::Create, &ctx);

        assert!(state.form_error.is_none());
        assert!(state.editing_id.is_none());
        assert_eq!(state.warehouses.len(), 1);
        let saved = find(&state, id).unwrap();
        assert_eq!(saved.warehouse.name, "Overflow Warehouse");
        assert_eq!(saved.total_bins(), 6);
    }

    #[test]
    fn shrinking_past_a_bin_that_holds_stock_is_blocked_with_an_inline_message() {
        let ctx = ctx_with_company();
        let mut state = load(&ctx);
        let id = add(&mut state, &ctx, "Main Warehouse", "2", "2");
        let tv = product(&ctx, "TV-1", "Electronics");
        let bin = find(&state, id).unwrap().bins[3].bin.id.unwrap();
        WarehouseService::new(ctx.db.clone())
            .place_stock(bin, tv, dec!(3))
            .unwrap();

        update(&mut state, Message::EditRow(id), &ctx);
        update(&mut state, Message::LinesChanged("1".into()), &ctx);
        update(&mut state, Message::Create, &ctx);

        assert!(state
            .form_error
            .as_deref()
            .unwrap_or_default()
            .contains("L2-P2"));
        assert!(state.form_open);
    }

    #[test]
    fn cancel_edit_clears_editing_state_without_touching_the_stored_warehouse() {
        let ctx = ctx_with_company();
        let mut state = load(&ctx);
        let id = add(&mut state, &ctx, "Main Warehouse", "1", "1");

        update(&mut state, Message::EditRow(id), &ctx);
        update(
            &mut state,
            Message::NameChanged("Should Not Save".into()),
            &ctx,
        );
        update(&mut state, Message::CancelEdit, &ctx);

        assert!(state.editing_id.is_none());
        assert!(!state.form_open);
        assert_eq!(state.warehouses[0].warehouse.name, "Main Warehouse");
    }

    #[test]
    fn toggle_expand_opens_and_closes_a_cards_category_table() {
        let ctx = ctx_with_company();
        let mut state = State::default();

        update(&mut state, Message::ToggleExpand(4), &ctx);
        assert_eq!(state.expanded, HashSet::from([4]));
        update(&mut state, Message::ToggleExpand(5), &ctx);
        update(&mut state, Message::ToggleExpand(4), &ctx);
        assert_eq!(state.expanded, HashSet::from([5]));
    }

    #[test]
    fn delete_waits_for_confirmation_and_keep_cancels_it() {
        let ctx = ctx_with_company();
        let mut state = load(&ctx);
        let id = add(&mut state, &ctx, "Main Warehouse", "1", "1");

        update(&mut state, Message::Remove(id), &ctx);
        assert_eq!(state.pending_removal, Some(id));
        build_everywhere(&state);
        update(&mut state, Message::CancelRemove, &ctx);

        assert!(state.pending_removal.is_none());
        assert_eq!(state.warehouses.len(), 1);
    }

    #[test]
    fn confirming_a_delete_removes_a_warehouse_without_stock() {
        let ctx = ctx_with_company();
        let mut state = load(&ctx);
        let id = add(&mut state, &ctx, "Main Warehouse", "2", "2");
        add(&mut state, &ctx, "Overflow Warehouse", "1", "1");

        update(&mut state, Message::Remove(id), &ctx);
        update(&mut state, Message::ConfirmRemove, &ctx);

        assert!(state.error.is_none());
        assert_eq!(state.warehouses.len(), 1);
        assert_eq!(state.warehouses[0].warehouse.name, "Overflow Warehouse");
        assert!(state
            .message
            .as_deref()
            .unwrap_or_default()
            .contains("Deleted Main Warehouse"));
    }

    /// A warehouse referenced by a stock movement is archived, not
    /// deleted, so its history survives.
    #[test]
    fn confirming_a_delete_archives_a_warehouse_with_stock_movements() {
        use crate::backend::inventory::stock_movements::{MovementType, StockMovement};

        let ctx = ctx_with_company();
        let mut state = load(&ctx);
        let id = add(&mut state, &ctx, "Main Warehouse", "1", "1");
        let tv = product(&ctx, "TV-1", "Electronics");
        ctx.db
            .run_migrations(crate::backend::repositories::accounting_repository::MIGRATIONS)
            .unwrap();
        InventoryRepository::new(ctx.db.clone())
            .record_movement(&StockMovement::new(
                ctx.company_id,
                tv,
                id,
                chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
                MovementType::Purchase,
                dec!(100),
            ))
            .unwrap();

        update(&mut state, Message::Remove(id), &ctx);
        update(&mut state, Message::ConfirmRemove, &ctx);

        assert_eq!(state.warehouses.len(), 1);
        assert!(!state.warehouses[0].warehouse.is_active);
        assert!(state
            .message
            .as_deref()
            .unwrap_or_default()
            .contains("archived"));
    }

    #[test]
    fn bulk_export_csv_writes_the_filtered_warehouses_with_the_field_headers() {
        let ctx = ctx_with_temp_export_dir();
        let mut state = load(&ctx);
        add(&mut state, &ctx, "Main Warehouse", "1", "2");
        add(&mut state, &ctx, "Overflow Warehouse", "1", "1");
        state.search = "main".into();

        update(&mut state, Message::BulkExportCsv, &ctx);

        assert!(state.error.is_none());
        let contents = std::fs::read_to_string(ctx.export_path(EXPORT_FILE)).unwrap();
        assert!(contents.contains("Main Warehouse") && !contents.contains("Overflow Warehouse"));
        assert!(contents.starts_with(WAREHOUSE_FIELDS[F_NAME]));

        let _ = std::fs::remove_dir_all(&ctx.export_dir);
    }

    #[test]
    fn bulk_export_csv_with_no_filter_exports_every_warehouse() {
        let ctx = ctx_with_temp_export_dir();
        let mut state = load(&ctx);
        add(&mut state, &ctx, "Main Warehouse", "1", "1");
        add(&mut state, &ctx, "Overflow Warehouse", "1", "1");

        update(&mut state, Message::BulkExportCsv, &ctx);

        let contents = std::fs::read_to_string(ctx.export_path(EXPORT_FILE)).unwrap();
        assert!(contents.contains("Main Warehouse") && contents.contains("Overflow Warehouse"));

        let _ = std::fs::remove_dir_all(&ctx.export_dir);
    }

    #[test]
    fn reset_all_clears_search_company_and_status_filter() {
        let ctx = ctx_with_company();
        let mut state = State {
            search: "main".into(),
            company_filter: Some(3),
            status_filter: StatusFilter::Inactive,
            page: 2,
            ..State::default()
        };

        update(&mut state, Message::ResetAll, &ctx);

        assert!(state.search.is_empty());
        assert_eq!(state.company_filter, None);
        assert_eq!(state.status_filter, StatusFilter::All);
        assert_eq!(state.page, 0);
    }

    #[test]
    fn status_and_company_filters_and_search_narrow_the_cards() {
        let active = overview(1, "Main Warehouse", 1, 1, 0, true);
        let archived = overview(2, "Old Warehouse", 1, 1, 0, false);

        assert!(matches_filters(&active, "", None, StatusFilter::Active));
        assert!(!matches_filters(&archived, "", None, StatusFilter::Active));
        assert!(matches_filters(&archived, "", None, StatusFilter::Inactive));
        assert!(matches_filters(&active, "", Some(1), StatusFilter::All));
        assert!(!matches_filters(&active, "", Some(2), StatusFilter::All));
        assert!(matches_filters(&active, "acme", None, StatusFilter::All));
        assert!(matches_filters(&active, "main", None, StatusFilter::All));
        assert!(!matches_filters(
            &active,
            "karachi",
            None,
            StatusFilter::All
        ));
    }

    #[test]
    fn the_company_filter_message_sets_and_clears_the_filter() {
        let ctx = ctx_with_company();
        let mut state = State::default();

        update(
            &mut state,
            Message::CompanyFilterChanged(Choice {
                id: 7,
                label: "Beta".into(),
            }),
            &ctx,
        );
        assert_eq!(state.company_filter, Some(7));
        update(
            &mut state,
            Message::CompanyFilterChanged(Choice {
                id: 0,
                label: ALL_LABEL.into(),
            }),
            &ctx,
        );
        assert_eq!(state.company_filter, None);
    }

    #[test]
    fn opening_a_bin_and_placing_stock_updates_its_contents() {
        let ctx = ctx_with_company();
        let mut state = load(&ctx);
        let id = add(&mut state, &ctx, "Main Warehouse", "2", "3");
        let tv = product(&ctx, "TV-1", "Electronics");

        update(&mut state, Message::OpenDetail(id), &ctx);
        let bin = find(&state, id).unwrap().bins[0].bin.id.unwrap();
        update(&mut state, Message::SelectBin(bin), &ctx);
        let detail = state.detail.as_ref().unwrap();
        assert_eq!(detail.items.len(), 1);
        let item = detail.items[0].clone();

        update(&mut state, Message::PlaceStock, &ctx);
        assert!(state.detail.as_ref().unwrap().place_error.is_some());
        update(&mut state, Message::PlaceItemSelected(item), &ctx);
        update(&mut state, Message::PlaceQuantityChanged("7".into()), &ctx);
        update(&mut state, Message::PlaceStock, &ctx);

        assert!(state.detail.as_ref().unwrap().place_error.is_none());
        let warehouse = find(&state, id).unwrap();
        assert_eq!(warehouse.bins[0].used(), dec!(7));
        assert_eq!(warehouse.bins[0].items[0].product_id, tv);
        assert_eq!(warehouse.bins[0].fullness(), Fullness::NotFull);
        build_everywhere(&state);

        update(&mut state, Message::CloseBin, &ctx);
        assert!(state.detail.as_ref().unwrap().bin.is_none());
        update(&mut state, Message::CloseDetail, &ctx);
        assert!(state.detail.is_none());
    }

    #[test]
    fn the_category_filter_dims_bins_without_that_category() {
        let warehouse = overview(1, "Main", 2, 3, 1, true);
        let first = &warehouse.bins[0];
        let key = category_key(first.items[0].category_id);
        assert!(bin_matches(first, None));
        assert!(bin_matches(first, Some(key)));
        assert!(!bin_matches(first, Some(key + 100)));
        let options = category_options(&warehouse);
        assert_eq!(options[0].label, ALL_LABEL);
        assert_eq!(options.len(), 1 + warehouse.category_totals().len());

        let ctx = ctx_with_company();
        let mut state = with_detail(State::default(), 1, Detail::default());
        update(
            &mut state,
            Message::CategoryFilterChanged(options[1].clone()),
            &ctx,
        );
        assert_eq!(state.detail.as_ref().unwrap().category, Some(options[1].id));
        update(
            &mut state,
            Message::CategoryFilterChanged(options[0].clone()),
            &ctx,
        );
        assert_eq!(state.detail.as_ref().unwrap().category, None);
    }

    #[test]
    fn utilization_and_field_texts_follow_the_bins() {
        let warehouse = overview(1, "Main", 1, 4, 2, true);
        // Bins 2 and 4 hold 3 and 5 of 40 capacity.
        assert_eq!(utilization_text(&warehouse), "20%");
        assert_eq!(field_text(&warehouse, F_USED_BINS), "2");
        assert_eq!(field_text(&warehouse, F_EMPTY_BINS), "2");
        assert_eq!(field_text(&warehouse, F_AVAILABLE_CAPACITY), "32");
        assert_eq!(field_text(&warehouse, F_STATUS), ACTIVE_LABEL);
        let bare = overview(2, "Bare", 0, 0, 0, true);
        assert_eq!(utilization_text(&bare), EMPTY_DASH);
        assert_eq!(field_text(&bare, F_AVAILABLE_CAPACITY), EMPTY_DASH);
    }

    #[test]
    fn paging_clamps_a_stale_page_index() {
        let paged = paginate((0..30).collect::<Vec<_>>(), 9, 12);
        assert_eq!((paged.pages, paged.page), (3, 2));
        assert_eq!(paged.rows, (24..30).collect::<Vec<_>>());
    }

    #[test]
    fn the_list_builds_at_every_width_and_theme_with_few_and_many_cards() {
        for count in [1, 5, 40] {
            build_everywhere(&state_with(count));
        }
        let mut filtered_out = state_with(5);
        filtered_out.search = "no such warehouse".into();
        build_everywhere(&filtered_out);
    }

    #[test]
    fn the_card_grid_builds_in_every_theme_and_width_with_any_number_of_cards() {
        for theme in THEMES {
            for width in WIDTHS {
                for count in [0, 1, 6, 7, 40] {
                    let mut state = state_with(count);
                    state.scroll.scroll(row_snap::Scroll::Lines(-1.0));
                    let rows: Vec<&WarehouseOverview> = state.warehouses.iter().collect();
                    let columns = spacing::grid_columns(spacing::layout_class(width));
                    for height in [0.0, 300.0, 700.0, 1200.0] {
                        let _ = card_grid(&state, theme, &rows, columns, height);
                    }
                }
            }
        }
    }

    #[test]
    fn the_empty_list_builds_at_every_width_and_theme() {
        let mut state = State::default();
        build_everywhere(&state);
        state.message = Some("Deleted Main.".into());
        state.error = Some("database is locked".into());
        build_everywhere(&state);
    }

    #[test]
    fn the_add_form_builds_at_every_width_and_theme() {
        let mut state = state_with(3);
        state.form_open = true;
        build_everywhere(&state);
        state.form_error = Some("Lines must be a whole number above zero.".into());
        build_everywhere(&state);
    }

    #[test]
    fn the_edit_form_builds_at_every_width_and_theme() {
        let mut state = state_with(3);
        state.form_open = true;
        state.editing_id = Some(2);
        state.form.name = "Warehouse 1".into();
        state.form.company = state.companies.first().cloned();
        build_everywhere(&state);
        // The panel also opens over the detail page.
        let state = with_detail(state, 99, Detail::default());
        build_everywhere(&state);
    }

    #[test]
    fn the_detail_builds_with_empty_partial_and_many_bins() {
        for (lines, positions, step) in [(0, 0, 0), (1, 4, 0), (3, 5, 2), (24, 17, 1)] {
            let mut state = state_with(2);
            state
                .warehouses
                .push(overview(50, "Detail", lines, positions, step, true));
            state.detail = Some(Detail {
                warehouse_id: 50,
                ..Detail::default()
            });
            build_everywhere(&state);
        }
    }

    #[test]
    fn the_detail_builds_with_the_bin_panel_open_and_closed_and_a_category_filter() {
        let open = with_detail(
            state_with(2),
            7,
            Detail {
                bin: Some(7 * 10_000 + 2),
                category: Some(2),
                items: vec![Choice {
                    id: 1,
                    label: "TV-1 - Television".into(),
                }],
                place_error: Some("Quantity must be a number, zero or more.".into()),
                ..Detail::default()
            },
        );
        build_everywhere(&open);
        // An empty bin, then the panel closed with the filter still on.
        let mut empty_bin = open.clone();
        if let Some(detail) = &mut empty_bin.detail {
            detail.bin = Some(7 * 10_000 + 1);
        }
        build_everywhere(&empty_bin);
        let mut closed = open;
        if let Some(detail) = &mut closed.detail {
            detail.bin = None;
        }
        build_everywhere(&closed);
        let mut pending = closed;
        pending.pending_removal = Some(7);
        build_everywhere(&pending);
    }

    #[test]
    fn line_groups_are_two_lines_wide_and_the_cells_follow_the_breakpoints() {
        assert_eq!(
            group_width(),
            spacing::grid_cell_size() * 2.0 + spacing::SPACE_4
        );
        assert_eq!(fullness_tone(Fullness::Empty), BadgeTone::Neutral);
        assert_eq!(fullness_tone(Fullness::NotFull), BadgeTone::Warning);
        assert_eq!(fullness_tone(Fullness::Full), BadgeTone::Negative);
        let split = spacing::panel_split(1366.0, true, spacing::SPACE_20);
        assert_eq!(
            spacing::grid_columns(spacing::layout_class(split.content.unwrap())),
            2
        );
    }

    #[test]
    fn every_field_index_resolves_to_a_distinct_name() {
        let names: HashSet<&str> = WAREHOUSE_FIELDS.iter().copied().collect();
        assert_eq!(names.len(), WAREHOUSE_FIELDS.len());
        let warehouse = overview(1, "Main", 2, 2, 1, true);
        for field in EXPORT_FIELDS {
            assert!(!field_text(&warehouse, field).is_empty() || field == F_DEFAULT_CAPACITY);
        }
        assert_eq!(field_text(&warehouse, F_CODE), "WH-1");
    }
}
