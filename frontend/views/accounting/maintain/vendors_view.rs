use crate::backend::core::validation::Validate;
use crate::backend::parties::vendors::{Vendor, VENDOR_TYPES};
use crate::backend::services::party_service::PartyService;
use crate::database::exports::csv;
use crate::frontend::components::badges::{self, BadgeTone};
use crate::frontend::components::button as button_style;
use crate::frontend::components::checkbox::{self as checkbox_style, CheckState};
use crate::frontend::components::colors::Theme;
use crate::frontend::components::data_table;
use crate::frontend::components::dropdown;
use crate::frontend::components::fonts;
use crate::frontend::components::forms;
use crate::frontend::components::icons;
use crate::frontend::components::inputs;
use crate::frontend::components::labels;
use crate::frontend::components::spacing;
use crate::frontend::components::table as table_style;
use crate::frontend::components::toolbar::StatusFilter;
use crate::frontend::components::typography;
use crate::frontend::components::{panels, states, tabs};
use crate::frontend::views::Ctx;
use iced::widget::{
    button, column, container, pick_list, row, scrollable, stack, text, text_input, tooltip, Space,
};
use iced::{Alignment, Element, Length};
use std::collections::HashSet;

const VENDOR_HEADERS: [&str; 21] = [
    "S.No",
    "",
    "Vendor ID",
    "Vendor Name",
    "Company Name",
    "Phone",
    "Email",
    "Website",
    "Mailing Address",
    "Shipping Address",
    "Payment Terms",
    "Debit",
    "Credit",
    "Balance",
    "Payment Method",
    "Attachments",
    "Description",
    "Status",
    "Create Timestamp",
    "Modified Timestamp",
    "Custom Fields",
];
const VENDOR_ALWAYS_SELECTED: [&str; 10] = [
    "S.No",
    "Vendor ID",
    "Vendor Name",
    "Company Name",
    "Phone",
    "Email",
    "Mailing Address",
    "Debit",
    "Credit",
    "Balance",
];
const VENDOR_DEFAULT_VISIBLE: [&str; 10] = [
    "S.No",
    "Vendor ID",
    "Vendor Name",
    "Company Name",
    "Phone",
    "Email",
    "Mailing Address",
    "Debit",
    "Credit",
    "Balance",
];
const VENDOR_VISIBLE_COLUMNS: [(&str, bool); 21] = [
    ("S.No", true),
    ("", true),
    ("Vendor ID", true),
    ("Vendor Name", true),
    ("Company Name", true),
    ("Phone", true),
    ("Email", true),
    ("Website", false),
    ("Mailing Address", true),
    ("Shipping Address", false),
    ("Payment Terms", false),
    ("Debit", true),
    ("Credit", true),
    ("Balance", true),
    ("Payment Method", false),
    ("Attachments", false),
    ("Description", false),
    ("Status", false),
    ("Create Timestamp", false),
    ("Modified Timestamp", false),
    ("Custom Fields", false),
];
const VENDOR_OPTIONAL_COLUMNS: [&str; 10] = [
    "Website",
    "Shipping Address",
    "Payment Terms",
    "Payment Method",
    "Attachments",
    "Description",
    "Status",
    "Create Timestamp",
    "Modified Timestamp",
    "Custom Fields",
];
const VENDOR_TYPE_OPTIONS: [&str; 6] = [
    "All",
    "Supplier",
    "Contractor",
    "Distributor",
    "Manufacturer",
    "Other",
];
const VENDOR_STATUS_OPTIONS: [&str; 2] = ["Active", "Inactive"];
const PAGE_SIZES: [usize; 4] = [10, 25, 50, 100];
const DEFAULT_PAGE_SIZE: usize = 50;
const VENDOR_FORM_TABS: [&str; 2] = ["Details", "Payment"];

#[derive(Debug, Clone, Default)]
pub struct State {
    vendors: Vec<Vendor>,
    name_input: String,
    email_input: String,
    phone_input: String,
    tax_id_input: String,
    vendor_type_input: String,
    status_input: String,
    payment_terms_input: String,
    form_tab: usize,
    search: String,
    vendor_type_filter: String,
    editing_id: Option<i64>,
    selected: HashSet<i64>,
    status_filter: StatusFilter,
    columns_open: bool,
    column_visible: Vec<bool>,
    form_open: bool,
    show_preview: bool,
    page: usize,
    page_size: usize,
    message: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    NameChanged(String),
    EmailChanged(String),
    PhoneChanged(String),
    TaxIdChanged(String),
    VendorTypeChanged(String),
    VendorTypeFilterChanged(String),
    StatusChanged(String),
    PaymentTermsChanged(String),
    FormTabChanged(usize),
    OpenForm,
    Preview,
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
    match service.list_vendors(ctx.company_id) {
        Ok(vendors) => State {
            vendors,
            column_visible: default_column_visibility(),
            ..Default::default()
        },
        Err(err) => State {
            error: Some(err.to_string()),
            column_visible: default_column_visibility(),
            ..Default::default()
        },
    }
}

pub fn update(state: &mut State, message: Message, ctx: &Ctx) {
    match message {
        Message::NameChanged(value) => state.name_input = value,
        Message::EmailChanged(value) => state.email_input = value,
        Message::PhoneChanged(value) => state.phone_input = value,
        Message::TaxIdChanged(value) => state.tax_id_input = value,
        Message::VendorTypeChanged(value) => state.vendor_type_input = value,
        Message::VendorTypeFilterChanged(value) => {
            state.vendor_type_filter = value;
            state.page = 0;
        }
        Message::StatusChanged(value) => state.status_input = value,
        Message::PaymentTermsChanged(value) => state.payment_terms_input = value,
        Message::FormTabChanged(index) => state.form_tab = index.min(VENDOR_FORM_TABS.len() - 1),
        Message::Preview => state.show_preview = !state.show_preview,
        Message::OpenForm => {
            state.message = None;
            state.error = None;
            state.editing_id = None;
            state.name_input.clear();
            state.email_input.clear();
            state.phone_input.clear();
            state.tax_id_input.clear();
            state.vendor_type_input = "Other".into();
            state.status_input = "Active".into();
            state.payment_terms_input = "30".into();
            state.form_tab = 0;
            state.form_open = true;
            state.show_preview = false;
        }
        Message::Create => {
            state.message = None;
            state.error = None;
            let Ok(payment_terms_days) = state.payment_terms_input.trim().parse::<i32>() else {
                state.error = Some("Payment terms must be a whole number of days.".into());
                return;
            };

            if let Some(id) = state.editing_id {
                let Some(existing) = state.vendors.iter().find(|v| v.id == Some(id)).cloned()
                else {
                    state.error = Some("that vendor no longer exists".into());
                    state.editing_id = None;
                    return;
                };
                let vendor = Vendor {
                    name: state.name_input.clone(),
                    email: optional_input(&state.email_input),
                    phone: optional_input(&state.phone_input),
                    tax_id: optional_input(&state.tax_id_input),
                    vendor_type: state.vendor_type_input.clone(),
                    is_active: state.status_input == "Active",
                    payment_terms_days,
                    ..existing
                };
                if let Err(err) = vendor.validate() {
                    state.error = Some(err.to_string());
                    return;
                }
                let service = PartyService::new(ctx.db.clone());
                match service.update_vendor(&vendor) {
                    Ok(()) => {
                        if let Some(slot) = state.vendors.iter_mut().find(|v| v.id == Some(id)) {
                            *slot = vendor;
                        }
                        state.editing_id = None;
                        state.form_open = false;
                        state.name_input.clear();
                        state.email_input.clear();
                        state.phone_input.clear();
                        state.tax_id_input.clear();
                        state.vendor_type_input = "Other".into();
                        state.status_input = "Active".into();
                        state.payment_terms_input = "30".into();
                        state.form_tab = 0;
                        state.message = Some("Saved changes.".into());
                        state.show_preview = false;
                    }
                    Err(err) => state.error = Some(err.to_string()),
                }
                return;
            }

            let mut vendor = Vendor::new(ctx.company_id, state.name_input.clone());
            vendor.email = optional_input(&state.email_input);
            vendor.phone = optional_input(&state.phone_input);
            vendor.tax_id = optional_input(&state.tax_id_input);
            vendor.vendor_type = state.vendor_type_input.clone();
            vendor.is_active = state.status_input == "Active";
            vendor.payment_terms_days = payment_terms_days;
            if let Err(err) = vendor.validate() {
                state.error = Some(err.to_string());
                return;
            }
            let service = PartyService::new(ctx.db.clone());
            match service.create_vendor(&vendor) {
                Ok(created) => {
                    state.vendors.push(created);
                    state.name_input.clear();
                    state.email_input.clear();
                    state.phone_input.clear();
                    state.tax_id_input.clear();
                    state.vendor_type_input = "Other".into();
                    state.status_input = "Active".into();
                    state.payment_terms_input = "30".into();
                    state.form_tab = 0;
                    state.form_open = false;
                    state.show_preview = false;
                }
                Err(err) => state.error = Some(err.to_string()),
            }
        }
        Message::EditRow(id) => {
            state.message = None;
            state.error = None;
            if let Some(vendor) = state.vendors.iter().find(|v| v.id == Some(id)) {
                state.editing_id = Some(id);
                state.form_open = true;
                state.show_preview = false;
                state.name_input = vendor.name.clone();
                state.email_input = vendor.email.clone().unwrap_or_default();
                state.phone_input = vendor.phone.clone().unwrap_or_default();
                state.tax_id_input = vendor.tax_id.clone().unwrap_or_default();
                state.vendor_type_input = vendor.vendor_type.clone();
                state.status_input = if vendor.is_active { "Active" } else { "Inactive" }.into();
                state.payment_terms_input = vendor.payment_terms_days.to_string();
                state.form_tab = 0;
            }
        }
        Message::CancelEdit => {
            state.editing_id = None;
            state.form_open = false;
            state.show_preview = false;
            state.name_input.clear();
            state.email_input.clear();
            state.phone_input.clear();
            state.tax_id_input.clear();
            state.vendor_type_input.clear();
            state.status_input.clear();
            state.payment_terms_input.clear();
            state.form_tab = 0;
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
                match service.delete_vendor(id) {
                    Ok(()) => {
                        deleted += 1;
                        state.vendors.retain(|v| v.id != Some(id));
                        state.selected.remove(&id);
                    }
                    Err(err) => failures.push(format!("vendor #{id}: {err}")),
                }
            }
            if !failures.is_empty() {
                state.error = Some(format!(
                    "Deleted {deleted}; {} could not be deleted -- {}",
                    failures.len(),
                    failures.join("; ")
                ));
            } else if deleted > 0 {
                state.message = Some(format!("Deleted {deleted} vendor(s)."));
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
            let selected: Vec<&Vendor> = if state.selected.is_empty() {
                state
                    .vendors
                    .iter()
                    .filter(|v| {
                        matches_filters(
                            v,
                            &query,
                            state.status_filter,
                            &state.vendor_type_filter,
                        )
                    })
                    .collect()
            } else {
                state
                    .vendors
                    .iter()
                    .filter(|v| state.selected.contains(&v.id.unwrap_or_default()))
                    .collect()
            };
            let headers = [
                "Name",
                "Email",
                "Phone",
                "Vendor Type",
                "Payment Terms (days)",
                "Status",
            ];
            let rows: Vec<Vec<String>> = selected
                .iter()
                .map(|v| {
                    vec![
                        v.name.clone(),
                        v.email.clone().unwrap_or_default(),
                        v.phone.clone().unwrap_or_default(),
                        v.vendor_type.clone(),
                        v.payment_terms_days.to_string(),
                        if v.is_active {
                            "Active".to_string()
                        } else {
                            "Inactive".to_string()
                        },
                    ]
                })
                .collect();
            let path = ctx.export_path("vendors_export.csv");
            match std::fs::write(&path, csv::export_csv(&headers, &rows)) {
                Ok(()) => {
                    state.message = Some(format!(
                        "Exported {} vendor(s) to {}",
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
            state.vendor_type_filter = "All".into();
            state.page = 0;
        }
        Message::ToggleColumns => state.columns_open = !state.columns_open,
        Message::ColumnToggled(index) => {
            if let Some(label) = VENDOR_OPTIONAL_COLUMNS.get(index) {
                if state.column_visible.len() != VENDOR_HEADERS.len() {
                    state.column_visible = default_column_visibility();
                }
                if let Some(column_index) = VENDOR_HEADERS.iter().position(|header| header == label)
                {
                    let was_visible = column_visible(state, column_index);
                    state.column_visible[column_index] = !was_visible;
                }
            }
        }
        Message::SelectAllColumns => {
            state.column_visible = default_column_visibility();
            for label in VENDOR_OPTIONAL_COLUMNS {
                if let Some(index) = VENDOR_HEADERS.iter().position(|header| *header == label) {
                    state.column_visible[index] = true;
                }
            }
        }
        Message::DeselectAllColumns => {
            state.column_visible = default_column_visibility();
            for label in VENDOR_OPTIONAL_COLUMNS {
                if let Some(index) = VENDOR_HEADERS.iter().position(|header| *header == label) {
                    state.column_visible[index] = false;
                }
            }
        }
        Message::ResetColumns => state.column_visible = default_column_visibility(),
    }
}

/// Archive (`active == false`) or restore (`true`) every selected
/// vendor by flipping `is_active`; the selection is kept so the user can
/// see what changed.
fn set_selected_active(state: &mut State, ctx: &Ctx, active: bool) {
    state.message = None;
    state.error = None;
    let service = PartyService::new(ctx.db.clone());
    let mut changed = 0;
    let mut failures = Vec::new();
    for id in state.selected.clone() {
        let Some(slot) = state.vendors.iter_mut().find(|r| r.id == Some(id)) else {
            continue;
        };
        if slot.is_active == active {
            continue;
        }
        let mut updated = slot.clone();
        updated.is_active = active;
        match service.update_vendor(&updated) {
            Ok(()) => {
                *slot = updated;
                changed += 1;
            }
            Err(err) => failures.push(format!("vendor #{id}: {err}")),
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
        state.message = Some(format!("{verb} {changed} vendor(s)."));
    }
}

fn optional_input(value: &str) -> Option<String> {
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_string())
}

fn default_column_visibility() -> Vec<bool> {
    VENDOR_HEADERS
        .iter()
        .map(|label| {
            label.is_empty()
                || VENDOR_DEFAULT_VISIBLE.contains(label)
                    && VENDOR_VISIBLE_COLUMNS
                        .iter()
                        .any(|(column, visible)| column == label && *visible)
        })
        .collect()
}

fn column_visible(state: &State, index: usize) -> bool {
    let Some(label) = VENDOR_HEADERS.get(index) else {
        return false;
    };
    if label.is_empty() || VENDOR_ALWAYS_SELECTED.contains(label) {
        return true;
    }
    state
        .column_visible
        .get(index)
        .copied()
        .unwrap_or_else(|| {
            VENDOR_VISIBLE_COLUMNS
                .iter()
                .find(|(column, _)| column == label)
                .is_some_and(|(_, visible)| *visible)
        })
}

fn optional_column_visibility(state: &State) -> Vec<bool> {
    VENDOR_OPTIONAL_COLUMNS
        .iter()
        .map(|label| {
            VENDOR_HEADERS
                .iter()
                .position(|header| header == label)
                .is_some_and(|index| column_visible(state, index))
        })
        .collect()
}

/// Whether a vendor passes both the search box and the status filter --
/// shared by the table's live filtering in `view` and by
/// `BulkExportCsv`'s "no selection -> export everything currently
/// visible" fallback, so the two can never disagree.
fn matches_filters(
    vendor: &Vendor,
    query: &str,
    status: StatusFilter,
    vendor_type: &str,
) -> bool {
    status.matches(vendor.is_active)
        && (vendor_type.is_empty() || vendor_type == "All" || vendor.vendor_type == vendor_type)
        && (query.is_empty()
            || vendor.name.to_lowercase().contains(query)
            || vendor
                .email
                .as_deref()
                .unwrap_or_default()
                .to_lowercase()
                .contains(query))
}

const TITLE: &str = "Vendors";
const SUBTITLE: &str = "Manage your vendors and their contact details.";
fn column_width_px(label: &str) -> f32 {
    match label {
        "Vendor ID" => 140.0,
        "Vendor Name" => 220.0,
        "Company Name" => 180.0,
        "Phone" => 120.0,
        "Email" => 220.0,
        "Website" => 180.0,
        "Mailing Address" | "Shipping Address" => 200.0,
        "Payment Terms" => 120.0,
        "Debit" | "Credit" => 100.0,
        "Balance" => 110.0,
        "Payment Method" | "Attachments" => 140.0,
        "Description" => 200.0,
        "Status" => 100.0,
        "Create Timestamp" | "Modified Timestamp" => 180.0,
        "Custom Fields" => 160.0,
        _ => 68.0,
    }
}

fn column_width(label: &str) -> Length {
    Length::Fixed(column_width_px(label))
}

fn table_widths(state: &State) -> (f32, f32) {
    let (field_width, field_count) = VENDOR_HEADERS
        .iter()
        .enumerate()
        .skip(2)
        .filter(|(index, _)| column_visible(state, *index))
        .fold((0.0, 0.0), |(width, count), (_, label)| {
            (width + column_width_px(label), count + 1.0)
        });
    let row_width = 68.0
        + 44.0
        + spacing::ACTION_COLUMN_WIDTH
        + field_width
        + spacing::MD * (field_count + 2.0);
    let table_width = row_width + spacing::MD * 2.0 + spacing::SCROLLBAR_WIDTH;
    (table_width, row_width)
}

fn vendor_preview(state: &State, theme: Theme) -> Element<'static, Message> {
    let saved = state
        .editing_id
        .and_then(|id| state.vendors.iter().find(|vendor| vendor.id == Some(id)));
    let name = if state.name_input.is_empty() {
        "New Vendor".into()
    } else {
        state.name_input.clone()
    };
    let vendor_id = saved
        .and_then(|vendor| vendor.reference_code.clone())
        .unwrap_or_else(|| "Assigned on save".into());
    let status = if state.status_input.is_empty() {
        "Active".into()
    } else {
        state.status_input.clone()
    };
    let value = |input: &str| {
        if input.is_empty() {
            "--".to_string()
        } else {
            input.to_string()
        }
    };
    let identity = vec![
        forms::PreviewField::new("Vendor ID", vendor_id.clone()),
        forms::PreviewField::new("Phone", value(&state.phone_input)),
        forms::PreviewField::new("Email", value(&state.email_input)),
    ];
    let details = forms::PreviewSection::new(
        "Vendor Details",
        vec![
            forms::PreviewField::new("Vendor ID", vendor_id),
            forms::PreviewField::new("Vendor Name", value(&state.name_input)),
            forms::PreviewField::new("Company Name", "--"),
            forms::PreviewField::new("Phone", value(&state.phone_input)),
            forms::PreviewField::new("Email", value(&state.email_input)),
            forms::PreviewField::new("Website", "--"),
            forms::PreviewField::new("Mailing Address", "--"),
            forms::PreviewField::new("Shipping Address", "--"),
            forms::PreviewField::new("Description", "--"),
            forms::PreviewField::new("Custom Fields", "--"),
            forms::PreviewField::new("Vendor Type", value(&state.vendor_type_input)),
        ],
    );
    let payment = forms::PreviewSection::new(
        "Payment & Account",
        vec![
            forms::PreviewField::new("Payment Terms", format!("{} days", state.payment_terms_input)),
            forms::PreviewField::new("Debit", "--"),
            forms::PreviewField::new("Credit", "--"),
            forms::PreviewField::new("Balance", "--"),
            forms::PreviewField::new("Payment Method", "--"),
            forms::PreviewField::new("Attachments", "--"),
            forms::PreviewField::new("Status", status.clone()),
            forms::PreviewField::new("Create Timestamp", "--"),
            forms::PreviewField::new("Modified Timestamp", "--"),
        ],
    );
    forms::record_preview(
        theme,
        forms::RecordPreview {
            name,
            subtitle: format!("Vendor · {}", value(&state.vendor_type_input)),
            status,
            identity,
            sections: vec![details, payment],
        },
        Message::Preview,
        Message::Preview,
    )
}

pub fn view(state: &State, theme: Theme) -> Element<'_, Message> {
    body(state, theme, None, |message| message)
}

/// Same as [`view`], except the title sits inline with `switcher` (a
/// type-switcher dropdown, e.g. "Maintain"'s section picker) in the
/// card header -- `wrap` converts this module's own `Message` into
/// whatever the caller's `switcher` produces, so the two can share one
/// `Element` tree despite having different message types.
pub fn page_header_view_with_switcher<'a, ParentMessage: Clone + 'a>(
    state: &'a State,
    theme: Theme,
    switcher: Element<'a, ParentMessage>,
    wrap: impl Fn(Message) -> ParentMessage + Clone + 'a,
) -> Element<'a, ParentMessage> {
    body(state, theme, Some(switcher), wrap)
}

/// A button with a bold label; callers add the style and the press.
fn bold_button<'a, M: 'a>(
    label: &'a str,
    padding: impl Into<iced::Padding>,
) -> iced::widget::Button<'a, M> {
    button(text(label).size(typography::label()).font(fonts::BOLD)).padding(padding)
}

/// Small count pill: primary tint when `active`, neutral otherwise.
fn count_badge(theme: Theme, label: String, active: bool) -> Element<'static, Message> {
    container(text(label).size(typography::caption()).color(if active {
        theme.primary()
    } else {
        theme.neutral_front()
    }))
    .padding([spacing::SPACE_2, spacing::SPACE_8])
    .style(badges::count(theme, active))
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
    let mut label_text = text(label)
        .size(typography::label())
        .color(tabs::text_color(theme, active));
    if active {
        label_text = label_text.font(fonts::BOLD);
    }
    column![
        button(
            row![label_text, count_badge(theme, count.to_string(), active)]
                .spacing(spacing::SPACE_8)
                .align_y(Alignment::Center)
        )
        .style(tabs::tab_button(theme))
        .padding([spacing::SPACE_6, spacing::SPACE_4])
        .on_press(on_press),
        container(Space::with_height(Length::Fixed(
            spacing::TAB_UNDERLINE_HEIGHT
        )))
        .width(Length::Fill)
        .style(tabs::underline(theme, active)),
    ]
    .spacing(spacing::SPACE_2)
    .width(Length::Shrink)
    .into()
}

/// Small icon-only button with a hover tooltip: outlined when `bordered`,
/// bare (tinted on hover) otherwise.
fn icon_button(
    theme: Theme,
    glyph: &'static str,
    tip: &'static str,
    message: Message,
    bordered: bool,
) -> Element<'static, Message> {
    let outlined = button_style::outlined(theme, false);
    let plain = button_style::plain(theme);
    tooltip(
        button(text(glyph).size(typography::emphasis()).center())
            .width(Length::Fixed(spacing::ICON_BUTTON_SIZE))
            .height(Length::Fixed(spacing::ICON_BUTTON_SIZE))
            .style(move |iced_theme, status| {
                if bordered {
                    outlined(iced_theme, status)
                } else {
                    plain(iced_theme, status)
                }
            })
            .on_press(message),
        container(text(tip).size(typography::caption()))
            .padding([spacing::SPACE_4, spacing::SPACE_8])
            .style(panels::flat_card(theme, states::RADIUS_MD)),
        tooltip::Position::Bottom,
    )
    .into()
}

/// Square select box in one of three states -- the header's tri-state
/// box and every row's box.
fn check_box(theme: Theme, state: CheckState, on_press: Message) -> Element<'static, Message> {
    let glyph = match state {
        CheckState::Checked => icons::CHECK,
        CheckState::Partial => icons::INDETERMINATE,
        CheckState::Unchecked => "",
    };
    container(
        button(text(glyph).size(typography::micro()).center())
            .width(Length::Fixed(typography::checkbox_size()))
            .height(Length::Fixed(typography::checkbox_size()))
            .padding(0)
            .style(checkbox_style::tri_state(theme, state))
            .on_press(on_press),
    )
    .width(Length::Fixed(spacing::CHECKBOX_COLUMN_WIDTH))
    .into()
}

/// Small tinted status pill.
fn status_pill(theme: Theme, label: &'static str, tone: BadgeTone) -> Element<'static, Message> {
    container(
        text(label)
            .size(typography::caption())
            .font(fonts::BOLD)
            .color(badges::tone_colors(theme, tone).1),
    )
    .padding([spacing::SPACE_2, spacing::SPACE_10])
    .style(badges::pill(theme, tone))
    .into()
}

/// Muted semibold column label, with the sort arrow when `sorted`.
fn column_label(
    theme: Theme,
    label: &'static str,
    sorted: bool,
    width: Length,
) -> Element<'static, Message> {
    let mut cell = row![text(label)
        .size(typography::caption())
        .font(fonts::BOLD)
        .color(theme.neutral_front())]
    .spacing(spacing::SPACE_4)
    .align_y(Alignment::Center);
    if sorted {
        cell = cell.push(labels::muted(
            theme,
            icons::SORT_ASCENDING,
            typography::micro(),
        ));
    }
    container(cell).width(width).into()
}

/// The light header row above the rows: tri-state box, column labels,
/// and room on the right for the scrollbar gutter so labels stay aligned
/// with the cells below.
fn table_header(
    theme: Theme,
    check: CheckState,
    on_toggle: Message,
    visible: &dyn Fn(usize) -> bool,
    row_width: f32,
) -> Element<'static, Message> {
    let mut header = row![
        container(text("S.No").size(typography::caption()).font(fonts::BOLD))
            .width(column_width("S.No")),
        container(check_box(theme, check, on_toggle)).width(Length::Fixed(44.0)),
    ]
        .spacing(spacing::MD)
        .align_y(Alignment::Center);
    for (index, label) in VENDOR_HEADERS.iter().enumerate().skip(2) {
        if visible(index) {
            header = header.push(column_label(theme, label, false, column_width(label)));
        }
    }
    header = header.push(Space::with_width(Length::Fixed(
        spacing::ACTION_COLUMN_WIDTH,
    )));
    container(header)
        .padding(iced::Padding {
            top: spacing::SPACE_10,
            right: spacing::MD + spacing::SCROLLBAR_WIDTH,
            bottom: spacing::SPACE_10,
            left: spacing::MD,
        })
        .width(Length::Fixed(row_width + spacing::MD * 2.0 + spacing::SCROLLBAR_WIDTH))
        .style(table_style::header_row(theme, 0.0))
        .into()
}

/// One table row: select box, avatar cell, status pill and the edit
/// button; the whole row opens the detail panel.
fn vendor_row(
    theme: Theme,
    vendor: &Vendor,
    serial_number: usize,
    row_width: f32,
    selected: bool,
    highlighted: bool,
    visible: &dyn Fn(usize) -> bool,
) -> Element<'static, Message> {
    let id = vendor.id.unwrap_or_default();
    let mut cells = row![
        container(text(serial_number.to_string()).size(typography::body()))
            .width(column_width("S.No")),
        container(check_box(
            theme,
            if selected {
                CheckState::Checked
            } else {
                CheckState::Unchecked
            },
            Message::RowSelected(id, !selected),
        ))
        .width(Length::Fixed(44.0)),
    ]
    .spacing(spacing::MD)
    .height(Length::Fixed(spacing::TABLE_ROW_HEIGHT))
    .align_y(Alignment::Center);
    for (index, label) in VENDOR_HEADERS.iter().enumerate().skip(2) {
        if !visible(index) {
            continue;
        }
        let cell: Element<'static, Message> = match *label {
            "Vendor Name" => labels::strong(theme, vendor.name.clone(), typography::label()).into(),
            "Status" => {
                let (label, tone) = if vendor.is_active {
                    ("Active", BadgeTone::Positive)
                } else {
                    ("Archived", BadgeTone::Neutral)
                };
                status_pill(theme, label, tone)
            }
            _ => labels::muted(
                theme,
                vendor_column_value(vendor, label),
                typography::body(),
            )
            .into(),
        };
        cells = cells.push(container(cell).width(column_width(label)));
    }
    cells = cells.push(
        container(icon_button(
            theme,
            icons::EDIT,
            "Edit",
            Message::EditRow(id),
            false,
        ))
        .width(Length::Fixed(spacing::ACTION_COLUMN_WIDTH))
        .align_x(iced::alignment::Horizontal::Right),
    );
    button(cells)
        .padding(0)
        .width(Length::Fixed(row_width))
        .style(table_style::row(theme, highlighted))
        .on_press(Message::EditRow(id))
        .into()
}

fn vendor_column_value(vendor: &Vendor, label: &str) -> String {
    match label {
        "Vendor ID" => vendor
            .reference_code
            .clone()
            .or_else(|| vendor.id.map(|id| id.to_string()))
            .unwrap_or_default(),
        "Company Name" | "Website" | "Mailing Address" | "Shipping Address"
        | "Payment Method" | "Attachments" | "Description" | "Create Timestamp"
        | "Modified Timestamp" | "Custom Fields" | "Debit" | "Credit" | "Balance" => {
            "--".into()
        }
        "Phone" => vendor.phone.clone().unwrap_or_default(),
        "Email" => vendor.email.clone().unwrap_or_default(),
        "Payment Terms" => format!("{} days", vendor.payment_terms_days),
        _ => String::new(),
    }
}

/// Page numbers to show: every page when there are few, otherwise the
/// first, last and the neighbours of the current page, with `None`
/// standing for a gap ("...").
fn page_items(page: usize, pages: usize) -> Vec<Option<usize>> {
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

/// Outlined Previous / Next button, dimmed when there is nowhere to go.
fn pager_button(
    theme: Theme,
    label: &'static str,
    target: Option<Message>,
) -> Element<'static, Message> {
    bold_button(label, [spacing::SPACE_6, spacing::SPACE_14])
        .style(button_style::outlined(theme, false))
        .on_press_maybe(target)
        .into()
}

/// A numbered page button: primary tint box when current, bare otherwise.
fn page_number(theme: Theme, index: usize, current: bool) -> Element<'static, Message> {
    let outlined = button_style::outlined(theme, true);
    let plain = button_style::plain(theme);
    button(
        text((index + 1).to_string())
            .size(typography::label())
            .center(),
    )
    .width(Length::Fixed(spacing::PAGE_BUTTON_WIDTH))
    .padding([spacing::SPACE_4, 0.0])
    .style(move |iced_theme, status| {
        if current {
            outlined(iced_theme, status)
        } else {
            plain(iced_theme, status)
        }
    })
    .on_press(Message::GoToPage(index))
    .into()
}

/// Card footer: Previous, numbered pages, Next on the left; result count
/// and the rows-per-page selector on the right.
fn footer(
    theme: Theme,
    page: usize,
    pages: usize,
    shown: usize,
    total: usize,
    size: usize,
) -> Element<'static, Message> {
    let mut numbers = row![].spacing(spacing::SPACE_4).align_y(Alignment::Center);
    for item in page_items(page, pages) {
        numbers = numbers.push(match item {
            Some(index) => page_number(theme, index, index == page),
            None => labels::muted(theme, "...", typography::label()).into(),
        });
    }
    row![
        pager_button(theme, "Previous", (page > 0).then_some(Message::PrevPage)),
        numbers,
        pager_button(
            theme,
            "Next",
            (page + 1 < pages).then_some(Message::GoToPage(page + 1))
        ),
        Space::with_width(Length::Fill),
        labels::muted(
            theme,
            format!("Showing {shown} of {total} results"),
            typography::label()
        ),
        pick_list(PAGE_SIZES, Some(size), Message::PageSizeChanged)
            .padding([spacing::SPACE_6, spacing::SPACE_12])
            .text_size(typography::label())
            .style(dropdown::style(theme))
            .menu_style(dropdown::dropdown_menu(theme)),
    ]
    .spacing(spacing::SPACE_12)
    .align_y(Alignment::Center)
    .padding(spacing::MD)
    .into()
}

/// A small muted label above a form input.
fn field<'a>(
    theme: Theme,
    label: &'a str,
    input: iced::widget::TextInput<'a, Message>,
) -> Element<'a, Message> {
    field_element(
        theme,
        label,
        input
            .style(inputs::text(theme))
            .width(Length::Fill)
            .padding(spacing::SM)
            .into(),
    )
}

fn field_element<'a>(
    theme: Theme,
    label: &'a str,
    input: Element<'a, Message>,
) -> Element<'a, Message> {
    column![
        labels::muted(theme, label, typography::caption()),
        input,
    ]
    .spacing(spacing::SPACE_4)
    .width(Length::FillPortion(1))
    .into()
}

fn readonly_field(
    theme: Theme,
    label: &'static str,
    value: impl Into<String>,
) -> Element<'static, Message> {
    field_element(
        theme,
        label,
        text(value.into())
            .size(typography::body())
            .color(theme.neutral_font())
            .into(),
    )
}

fn body<'a, M: Clone + 'a>(
    state: &'a State,
    theme: Theme,
    switcher: Option<Element<'a, M>>,
    wrap: impl Fn(Message) -> M + Clone + 'a,
) -> Element<'a, M> {
    let lift = |element: Element<'a, Message>| element.map(wrap.clone());
    if state.show_preview {
        let mut preview = column![];
        if let Some(switcher) = switcher {
            preview = preview.push(switcher);
        }
        return preview
            .push(lift(vendor_preview(state, theme)))
            .spacing(spacing::MD)
            .padding(spacing::SPACE_20)
            .width(Length::Fill)
            .height(Length::Fill)
            .into();
    }
    let visible_column = |index: usize| column_visible(state, index);

    let query = state.search.trim().to_lowercase();
    let vendor_type = if state.vendor_type_filter.is_empty() {
        "All"
    } else {
        state.vendor_type_filter.as_str()
    };
    let filtered: Vec<&Vendor> = state
        .vendors
        .iter()
        .filter(|vendor| matches_filters(vendor, &query, state.status_filter, vendor_type))
        .collect();
    let total = filtered.len();
    let size = state.rows_per_page();
    let pages = total.div_ceil(size).max(1);
    let page = state.page.min(pages - 1);
    let visible: Vec<&Vendor> = filtered
        .iter()
        .skip(page * size)
        .take(size)
        .copied()
        .collect();

    // Card header: title, count and subtitle on the left; search, the
    // utility icon buttons and the solid Add button on the right.
    let mut titles = row![
        labels::strong(theme, TITLE, typography::panel_title()),
        lift(count_badge(
            theme,
            format!("{} Total", state.vendors.len()),
            true
        )),
    ]
    .spacing(spacing::SPACE_12)
    .align_y(Alignment::Center);
    if let Some(switcher) = switcher {
        titles = titles.push(switcher);
    }
    let search_input = text_input("Search vendors by name or email", &state.search)
        .style(inputs::text(theme))
        .on_input(Message::SearchChanged)
        .padding(iced::Padding {
            top: spacing::SM,
            right: spacing::SPACE_12,
            bottom: spacing::SM,
            left: spacing::SEARCH_ICON_INSET,
        })
        .size(typography::label())
        .width(Length::Fill);
    let search_box = stack![
        search_input,
        container(labels::muted(theme, icons::SEARCH, typography::label()))
            .padding([spacing::SPACE_10, spacing::SPACE_12])
    ]
    .width(Length::Fixed(spacing::SEARCH_WIDTH));
    let selected_vendor_type = if state.vendor_type_filter.is_empty() {
        "All"
    } else {
        state.vendor_type_filter.as_str()
    };
    let controls = row![
        search_box,
        pick_list(
            VENDOR_TYPE_OPTIONS,
            Some(selected_vendor_type),
            |value| Message::VendorTypeFilterChanged(value.to_string())
        )
        .placeholder("Vendor type")
        .style(dropdown::style(theme))
        .menu_style(dropdown::dropdown_menu(theme)),
        icon_button(theme, icons::RESET, "Reset all", Message::ResetAll, true),
        icon_button(
            theme,
            icons::COLUMNS,
            "Columns",
            Message::ToggleColumns,
            true
        ),
        icon_button(
            theme,
            icons::EXPORT,
            "Export CSV",
            Message::BulkExportCsv,
            true
        ),
        bold_button("Add Vendor", [spacing::SPACE_10, spacing::MD])
            .style(button_style::mode(theme, true))
            .on_press(Message::OpenForm),
    ]
    .spacing(spacing::SPACE_12)
    .align_y(Alignment::Center);
    let card_header = row![
        column![
            titles,
            labels::muted(theme, SUBTITLE, typography::caption())
        ]
        .spacing(spacing::SPACE_4),
        Space::with_width(Length::Fill),
        lift(controls.into()),
    ]
    .align_y(Alignment::Center)
    .padding(spacing::MD);

    // Tabs: All / Active / Archived, each with its count.
    let active_count = state.vendors.iter().filter(|r| r.is_active).count();
    let tab_row = row![
        status_tab(
            theme,
            "All",
            state.vendors.len(),
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
            state.vendors.len() - active_count,
            state.status_filter == StatusFilter::Inactive,
            Message::StatusFilterChanged(StatusFilter::Inactive)
        ),
    ]
    .spacing(spacing::LG);

    let row_ids: Vec<i64> = visible.iter().map(|v| v.id.unwrap_or_default()).collect();
    let selected_here = row_ids
        .iter()
        .filter(|id| state.selected.contains(id))
        .count();
    let check = CheckState::from_counts(selected_here, row_ids.len());
    let toggle_all = Message::SelectAllChanged(row_ids, check != CheckState::Checked);
    let (table_width, row_width) = table_widths(state);
    let header_row = table_header(theme, check, toggle_all, &visible_column, row_width);

    let mut rows = column![];
    for (index, vendor) in visible.iter().enumerate() {
        if index > 0 {
            rows = rows.push(
                container(Space::with_height(Length::Fixed(spacing::SPACE_1)))
                    .width(Length::Fill)
                    .style(table_style::divider(theme)),
            );
        }
        let id = vendor.id.unwrap_or_default();
        rows = rows.push(vendor_row(
            theme,
            vendor,
            page * size + index + 1,
            row_width,
            state.selected.contains(&id),
            state.selected.contains(&id) || (state.form_open && state.editing_id == vendor.id),
            &visible_column,
        ));
    }
    let table_area = scrollable(
        column![header_row, rows.width(Length::Fixed(row_width))]
            .width(Length::Fixed(table_width))
            .spacing(spacing::SPACE_1),
    )
        .direction(scrollable::Direction::Both {
            vertical: scrollable::Scrollbar::new()
                .width(spacing::SCROLLBAR_WIDTH)
                .scroller_width(spacing::SCROLLBAR_WIDTH)
                .spacing(0.0),
            horizontal: scrollable::Scrollbar::new()
                .width(spacing::SCROLLBAR_WIDTH)
                .scroller_width(spacing::SCROLLBAR_WIDTH)
                .spacing(0.0),
        })
        .height(Length::Fill);

    let mut card = column![
        card_header,
        lift(container(tab_row).padding([0.0, spacing::MD]).into())
    ];
    // Removed the conditional block for bulk actions as it is no longer needed
    card = card.push(lift(table_area.into()));
    if let Some(message) = &state.message {
        card = card.push(lift(
            container(text(message.clone()).color(theme.positive_front()))
                .padding([spacing::SPACE_6, spacing::MD])
                .into(),
        ));
    }
    if let Some(error) = &state.error {
        card = card.push(lift(
            container(text(error.clone()).color(theme.negative_front()))
                .padding([spacing::SPACE_6, spacing::MD])
                .into(),
        ));
    }
    card = card.push(lift(
        container(Space::with_height(Length::Fixed(spacing::SPACE_1)))
            .width(Length::Fill)
            .style(table_style::divider(theme))
            .into(),
    ));
    card = card.push(lift(footer(theme, page, pages, visible.len(), total, size)));
    let card = container(card)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(panels::card(theme, states::RADIUS_LG));

    // Detail panel: header with close, a summary card when editing, the
    // fields in a two-column grid, then Save / Cancel.
    let editing = state
        .editing_id
        .and_then(|id| state.vendors.iter().find(|r| r.id == Some(id)));
    let summary_name = if state.name_input.is_empty() {
        "New Vendor".to_string()
    } else {
        state.name_input.clone()
    };
    let summary = container(
        row![
            column![
                labels::strong(theme, summary_name, typography::emphasis()),
                labels::muted(
                    theme,
                    format!(
                        "{}  ·  {}",
                        if state.phone_input.is_empty() { "--" } else { &state.phone_input },
                        if state.email_input.is_empty() { "--" } else { &state.email_input }
                    ),
                    typography::caption(),
                ),
            ]
            .spacing(spacing::SPACE_4)
            .width(Length::Fill),
            status_pill(
                theme,
                if state.status_input == "Inactive" {
                    "Archived"
                } else {
                    "Active"
                },
                if state.status_input == "Inactive" {
                    BadgeTone::Neutral
                } else {
                    BadgeTone::Positive
                },
            ),
        ]
        .spacing(spacing::MD)
        .align_y(Alignment::Center),
    )
    .padding(spacing::MD)
    .width(Length::Fill)
    .style(panels::flat_card(theme, states::RADIUS_MD))
    .into();
    let mut form = column![].spacing(spacing::MD);
    match state.form_tab {
        0 => {
            let vendor_id = editing
                .and_then(|vendor| vendor.reference_code.clone())
                .unwrap_or_else(|| "Assigned on save".into());
            form = form
                .push(
                    row![
                        readonly_field(theme, "Vendor ID", vendor_id),
                        field(
                            theme,
                            "Vendor name",
                            text_input("Vendor name", &state.name_input)
                                .on_input(Message::NameChanged)
                        ),
                    ]
                    .spacing(spacing::SM),
                )
                .push(
                    row![
                        readonly_field(theme, "Company Name", "--"),
                        field(
                            theme,
                            "Phone (optional)",
                            text_input("Phone (optional)", &state.phone_input)
                                .on_input(Message::PhoneChanged)
                        ),
                    ]
                    .spacing(spacing::SM),
                )
                .push(
                    row![
                        field(
                            theme,
                            "Email (optional)",
                            text_input("Email (optional)", &state.email_input)
                                .on_input(Message::EmailChanged)
                        ),
                        readonly_field(theme, "Website", "--"),
                    ]
                    .spacing(spacing::SM),
                )
                .push(
                    row![
                        readonly_field(theme, "Mailing Address", "--"),
                        readonly_field(theme, "Shipping Address", "--"),
                    ]
                    .spacing(spacing::SM),
                )
                .push(
                    row![
                        field(
                            theme,
                            "Tax ID (optional)",
                            text_input("Tax ID (optional)", &state.tax_id_input)
                                .on_input(Message::TaxIdChanged)
                        ),
                        readonly_field(theme, "Description", "--"),
                    ]
                    .spacing(spacing::SM),
                )
                .push(
                    row![
                        field_element(
                            theme,
                            "Vendor type",
                            pick_list(VENDOR_TYPES, Some(state.vendor_type_input.as_str()), |value| {
                                Message::VendorTypeChanged(value.to_string())
                            })
                            .style(dropdown::style(theme))
                            .padding(spacing::SM)
                            .width(Length::Fill)
                            .into(),
                        ),
                        readonly_field(theme, "Custom Fields", "--"),
                    ]
                    .spacing(spacing::SM),
                );
        }
        _ => {
            form = form
                .push(
                    row![
                        field(
                            theme,
                            "Payment Terms",
                            text_input("30", &state.payment_terms_input)
                                .on_input(Message::PaymentTermsChanged)
                        ),
                        readonly_field(theme, "Debit", "--"),
                    ]
                    .spacing(spacing::SM),
                )
                .push(
                    row![
                        readonly_field(theme, "Credit", "--"),
                        readonly_field(theme, "Balance", "--"),
                    ]
                    .spacing(spacing::SM),
                )
                .push(
                    row![
                        readonly_field(theme, "Payment Method", "--"),
                        readonly_field(theme, "Attachments", "--"),
                    ]
                    .spacing(spacing::SM),
                )
                .push(
                    row![
                        field_element(
                            theme,
                            "Status",
                            pick_list(
                                VENDOR_STATUS_OPTIONS,
                                Some(state.status_input.as_str()),
                                |value| Message::StatusChanged(value.to_string()),
                            )
                            .style(dropdown::style(theme))
                            .padding(spacing::SM)
                            .width(Length::Fill)
                            .into(),
                        ),
                        readonly_field(theme, "Create Timestamp", "--"),
                    ]
                    .spacing(spacing::SM),
                )
                .push(row![
                    readonly_field(theme, "Modified Timestamp", "--"),
                    Space::with_width(Length::Fill),
                ]);
        }
    }
    let footer = row![
        bold_button("Save", [spacing::SM, spacing::MD])
            .style(button_style::mode(theme, true))
            .on_press(Message::Create),
        bold_button("Cancel", [spacing::SM, spacing::MD])
            .style(button_style::outlined(theme, false))
            .on_press(Message::CancelEdit),
    ]
    .spacing(spacing::SM)
    .into();
    let form = forms::detail_panel(forms::DetailPanel {
        theme,
        title: "Vendor Details",
        tab_labels: &VENDOR_FORM_TABS,
        active_tab: state.form_tab,
        on_select_tab: Message::FormTabChanged,
        summary: Some(summary),
        content: form.into(),
        footer,
        on_preview: Message::Preview,
        on_close: Message::CancelEdit,
    });

    // The right-hand third: the detail panel and/or the columns picker,
    // hidden until one of them is opened.
    let mut side = column![].spacing(spacing::SPACE_20).height(Length::Fill);
    if state.form_open {
        side = side.push(form);
    }
    if state.columns_open {
        let optional_visibility = optional_column_visibility(state);
        side = side.push(data_table::columns_panel(
            theme,
            &VENDOR_OPTIONAL_COLUMNS,
            &optional_visibility,
            Message::ColumnToggled,
            Message::SelectAllColumns,
            Message::DeselectAllColumns,
            Message::ResetColumns,
        ));
    }

    let panel_open = state.form_open || state.columns_open;
    let mut main = row![container(card).width(if panel_open {
        Length::FillPortion(2)
    } else {
        Length::Fill
    })]
    .spacing(spacing::SPACE_20)
    .height(Length::Fill);
    if panel_open {
        main = main.push(lift(
            container(side)
                .padding(spacing::MD)
                .width(Length::FillPortion(1))
                .height(Length::Fill)
                .style(panels::flat_card(theme, states::RADIUS_LG))
                .into(),
        ));
    }
    container(main)
        .padding(spacing::SPACE_20)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vendor_columns_keep_serial_and_selection_first_and_pin_required_columns() {
        assert_eq!(VENDOR_HEADERS[0], "S.No");
        assert_eq!(VENDOR_HEADERS[1], "");
        let state = State::default();
        for label in VENDOR_ALWAYS_SELECTED {
            let index = VENDOR_HEADERS.iter().position(|header| *header == label).unwrap();
            assert!(column_visible(&state, index), "{label} should stay visible");
        }
        for label in VENDOR_DEFAULT_VISIBLE {
            assert!(VENDOR_ALWAYS_SELECTED.contains(&label));
        }
        assert!(!column_visible(
            &state,
            VENDOR_HEADERS
                .iter()
                .position(|header| *header == "Website")
                .unwrap()
        ));
    }

    #[test]
    fn vendor_type_filter_matches_only_the_selected_type() {
        let mut vendor = Vendor::new(1, "Acme Supplies");
        vendor.vendor_type = "Contractor".into();
        assert!(matches_filters(&vendor, "", StatusFilter::All, "All"));
        assert!(matches_filters(
            &vendor,
            "",
            StatusFilter::All,
            "Contractor"
        ));
        assert!(!matches_filters(
            &vendor,
            "",
            StatusFilter::All,
            "Supplier"
        ));
    }

    #[test]
    fn detail_and_payment_tabs_build_with_the_full_vendor_schema() {
        let state = State {
            form_open: true,
            vendor_type_input: "Other".into(),
            payment_terms_input: "30".into(),
            status_input: "Active".into(),
            ..State::default()
        };
        for form_tab in 0..VENDOR_FORM_TABS.len() {
            let mut tab_state = state.clone();
            tab_state.form_tab = form_tab;
            let _: Element<'_, Message> = view(&tab_state, Theme::Light);
        }
        let mut preview_state = state;
        preview_state.name_input = "Acme Supplies".into();
        preview_state.phone_input = "+1 555 0100".into();
        preview_state.vendor_type_input = "Supplier".into();
        preview_state.show_preview = true;
        let _: Element<'_, Message> = view(&preview_state, Theme::Light);
    }
}