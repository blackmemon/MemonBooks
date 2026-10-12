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
use crate::frontend::components::scrollbar;
use crate::frontend::components::spacing;
use crate::frontend::components::table as table_style;
use crate::frontend::components::toolbar::StatusFilter;
use crate::frontend::components::typography;
use crate::frontend::components::{panels, states};
use crate::frontend::views::Ctx;
use iced::widget::text::Wrapping;
use iced::widget::Row;
use iced::widget::{
    button, column, container, pick_list, responsive, row, scrollable, stack, text, text_input,
    tooltip, Space,
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
const VENDOR_FORM_TABS: [&str; 6] = [
    "Basic",
    "Contact",
    "Address",
    "Additional",
    "Status",
    "Customization",
];

/// Dropdown entry showing "Label (count)"; equality is by label only so the
/// selected entry still matches when the count changes.
#[derive(Debug, Clone)]
struct CountedOption {
    label: &'static str,
    count: usize,
    message: Message,
}

impl PartialEq for CountedOption {
    fn eq(&self, other: &Self) -> bool {
        self.label == other.label
    }
}

impl std::fmt::Display for CountedOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.label, self.count)
    }
}

fn filter_dropdown(
    theme: Theme,
    placeholder: &'static str,
    options: Vec<(&'static str, usize, Message, bool)>,
) -> Element<'static, Message> {
    let selected = options.iter().position(|option| option.3);
    let entries: Vec<CountedOption> = options
        .into_iter()
        .map(|(label, count, message, _)| CountedOption {
            label,
            count,
            message,
        })
        .collect();
    let current = selected.map(|index| entries[index].clone());
    pick_list(entries, current, |option| option.message)
        .placeholder(placeholder)
        .style(dropdown::style(theme))
        .menu_style(dropdown::dropdown_menu(theme))
        .into()
}

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
    mailing_address_input: String,
    payment_method_input: String,
    description_input: String,
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
    sort: Option<(&'static str, bool)>,
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
    MailingAddressChanged(String),
    PaymentMethodChanged(String),
    DescriptionChanged(String),
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
    SortBy(&'static str),
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
        Message::MailingAddressChanged(value) => state.mailing_address_input = value,
        Message::PaymentMethodChanged(value) => state.payment_method_input = value,
        Message::DescriptionChanged(value) => state.description_input = value,
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
            state.mailing_address_input.clear();
            state.payment_method_input.clear();
            state.description_input.clear();
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
                        state.mailing_address_input.clear();
                        state.payment_method_input.clear();
                        state.description_input.clear();
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
                    state.mailing_address_input.clear();
                    state.payment_method_input.clear();
                    state.description_input.clear();
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
                state.status_input = if vendor.is_active {
                    "Active"
                } else {
                    "Inactive"
                }
                .into();
                state.payment_terms_input = vendor.payment_terms_days.to_string();
                state.mailing_address_input.clear();
                state.payment_method_input.clear();
                state.description_input.clear();
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
            state.mailing_address_input.clear();
            state.payment_method_input.clear();
            state.description_input.clear();
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
        Message::SortBy(label) => {
            state.sort = match state.sort {
                Some((current, true)) if current == label => Some((label, false)),
                Some((current, false)) if current == label => None,
                _ => Some((label, true)),
            };
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
                        matches_filters(v, &query, state.status_filter, &state.vendor_type_filter)
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
    state.column_visible.get(index).copied().unwrap_or_else(|| {
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

fn matches_filters(vendor: &Vendor, query: &str, status: StatusFilter, vendor_type: &str) -> bool {
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
    let basic_information = forms::PreviewSection::new(
        "Basic Information",
        vec![
            forms::PreviewField::new("S.No", "Auto-generated"),
            forms::PreviewField::new("Vendor ID", vendor_id),
            forms::PreviewField::new("Vendor Name", value(&state.name_input)),
            forms::PreviewField::new("Company Name", "--"),
            forms::PreviewField::new("Vendor Type", value(&state.vendor_type_input)),
        ],
    );
    let contact_information = forms::PreviewSection::new(
        "Contact Information",
        vec![
            forms::PreviewField::new("Phone", value(&state.phone_input)),
            forms::PreviewField::new("Email", value(&state.email_input)),
            forms::PreviewField::new("Website", "--"),
        ],
    );
    let address_information = forms::PreviewSection::new(
        "Address Information",
        vec![
            forms::PreviewField::new("Mailing Address", "--"),
            forms::PreviewField::new("Shipping Address", "--"),
        ],
    );
    let additional_information = forms::PreviewSection::new(
        "Additional Information",
        vec![
            forms::PreviewField::new(
                "Payment Terms",
                format!("{} days", state.payment_terms_input),
            ),
            forms::PreviewField::new("Payment Method", "--"),
            forms::PreviewField::new("Tax ID", value(&state.tax_id_input)),
            forms::PreviewField::new("Description", "--"),
            forms::PreviewField::new("Attachments", "--"),
        ],
    );
    let status_audit = forms::PreviewSection::new(
        "Status & Audit",
        vec![
            forms::PreviewField::new("Status", status.clone()),
            forms::PreviewField::new("Create Timestamp", "--"),
            forms::PreviewField::new("Modified Timestamp", "--"),
        ],
    );
    let customization = forms::PreviewSection::new(
        "Customization",
        vec![forms::PreviewField::new("Custom Fields", "--")],
    );
    forms::record_preview(
        theme,
        forms::RecordPreview {
            name,
            subtitle: format!("Vendor · {}", value(&state.vendor_type_input)),
            status,
            identity,
            sections: vec![
                basic_information,
                contact_information,
                address_information,
                additional_information,
                status_audit,
                customization,
            ],
        },
        Message::Preview,
        Message::Preview,
    )
}

/// A button with a bold label; callers add the style and the press.
fn bold_button<'a, M: 'a>(
    label: impl text::IntoFragment<'a>,
    padding: impl Into<iced::Padding>,
) -> iced::widget::Button<'a, M> {
    button(
        text(label)
            .size(typography::label())
            .font(fonts::BOLD)
            .wrapping(Wrapping::None),
    )
    .padding(padding)
}

/// A bold-labelled button with an icon before the label.
fn icon_label_button<'a, M: 'a>(
    glyph: &'static str,
    label: &'a str,
    padding: impl Into<iced::Padding>,
) -> iced::widget::Button<'a, M> {
    button(
        row![
            icons::icon(glyph).size(typography::label()),
            text(label)
                .size(typography::label())
                .font(fonts::BOLD)
                .wrapping(Wrapping::None),
        ]
        .spacing(spacing::SPACE_6)
        .align_y(Alignment::Center),
    )
    .padding(padding)
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
        button(icons::icon(glyph).size(typography::emphasis()).center())
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

fn check_box(theme: Theme, state: CheckState, on_press: Message) -> Element<'static, Message> {
    checkbox_style::select_box(theme, state, on_press)
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
fn sort_of(sort: Option<(&'static str, bool)>, label: &str) -> Option<bool> {
    match sort {
        Some((current, ascending)) if current == label => Some(ascending),
        _ => None,
    }
}

fn column_label(
    theme: Theme,
    label: &'static str,
    sorted: Option<bool>,
    width: Length,
) -> Element<'static, Message> {
    let mut cell = row![text(label)
        .size(typography::label() + 1.0)
        .font(fonts::BOLD)
        .color(theme.neutral_front())]
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
        .style(|_iced_theme: &iced::Theme, _status| button::Style::default())
        .on_press(Message::SortBy(label))
        .into()
}

/// Horizontal geometry of the table: the fixed width of every shown
/// column plus how much the "Vendor Name" column grows so the table
/// fills the card when the columns would otherwise fall short of it.
struct TableLayout {
    row_width: f32,
    name_extra: f32,
}

impl TableLayout {
    /// Width of the header and of every row, edge padding included.
    fn total_width(&self) -> f32 {
        self.row_width + spacing::MD * 2.0
    }

    fn cell(&self, label: &str) -> Length {
        let extra = if label == "Vendor Name" {
            self.name_extra
        } else {
            0.0
        };
        Length::Fixed(column_width_px(label) + extra)
    }
}

fn table_layout(state: &State, available: f32) -> TableLayout {
    let (field_width, field_count) = VENDOR_HEADERS
        .iter()
        .enumerate()
        .skip(2)
        .filter(|(index, _)| column_visible(state, *index))
        .fold((0.0, 0.0), |(width, count), (_, label)| {
            (width + column_width_px(label), count + 1.0)
        });
    let row_width = column_width_px("S.No")
        + 44.0
        + spacing::ACTION_COLUMN_WIDTH
        + field_width
        + spacing::MD * (field_count + 2.0);
    let fixed_total = row_width + spacing::MD * 2.0;
    TableLayout {
        row_width,
        name_extra: (available - fixed_total).max(0.0),
    }
}

/// How the page's width is shared between the table card and the side
/// panel; `None` means that part is not shown.
#[derive(Debug, Clone, Copy, PartialEq)]
struct PageSplit {
    card: Option<f32>,
    panel: Option<f32>,
}

/// Wide and medium pages show the card beside the panel (the panel never
/// narrower than `PANEL_MIN_WIDTH`); a narrow page shows only one of them,
/// the panel replacing the card while it is open.
fn page_split(inner: f32, panel_open: bool) -> PageSplit {
    let class = spacing::layout_class(inner);
    match (panel_open, class) {
        (false, _) => PageSplit {
            card: Some(inner),
            panel: None,
        },
        (true, spacing::LayoutClass::Narrow) => PageSplit {
            card: None,
            panel: Some(inner),
        },
        (true, class) => {
            let share = if class == spacing::LayoutClass::Wide {
                spacing::PANEL_WIDE_SHARE
            } else {
                spacing::PANEL_MEDIUM_SHARE
            };
            let panel = (inner * share).max(spacing::PANEL_MIN_WIDTH);
            PageSplit {
                card: Some((inner - panel - spacing::SPACE_20).max(0.0)),
                panel: Some(panel),
            }
        }
    }
}

/// The filtered rows and paging numbers behind the table and its footer.
struct VendorPage<'a> {
    rows: Vec<&'a Vendor>,
    total: usize,
    pages: usize,
    page: usize,
    size: usize,
}

fn vendor_page(state: &State) -> VendorPage<'_> {
    let query = state.search.trim().to_lowercase();
    let vendor_type = if state.vendor_type_filter.is_empty() {
        "All"
    } else {
        state.vendor_type_filter.as_str()
    };
    let mut filtered: Vec<&Vendor> = state
        .vendors
        .iter()
        .filter(|vendor| matches_filters(vendor, &query, state.status_filter, vendor_type))
        .collect();
    // Default order (no header chosen, or S.No) is by vendor id.
    filtered.sort_by_key(|vendor| vendor.id);
    if let Some((label, ascending)) = state.sort {
        // S.No is the id order, so only its direction matters.
        if label == "S.No" {
            if !ascending {
                filtered.reverse();
            }
        } else {
            filtered.sort_by(|a, b| {
                let order = compare_by_column(a, b, label);
                if ascending {
                    order
                } else {
                    order.reverse()
                }
            });
        }
    }
    let total = filtered.len();
    let size = state.rows_per_page();
    let pages = total.div_ceil(size).max(1);
    let page = state.page.min(pages - 1);
    let rows = filtered.into_iter().skip(page * size).take(size).collect();
    VendorPage {
        rows,
        total,
        pages,
        page,
        size,
    }
}

fn results_label(shown: usize, total: usize) -> String {
    format!("Showing {shown} of {total} results")
}

/// The light header row above the rows: tri-state box and column labels,
/// padded exactly like a row so every label sits over its cells.
fn table_header(
    theme: Theme,
    sort: Option<(&'static str, bool)>,
    check: CheckState,
    on_toggle: Message,
    visible: &dyn Fn(usize) -> bool,
    layout: &TableLayout,
) -> Element<'static, Message> {
    let mut header = row![
        container(column_label(
            theme,
            "S.No",
            sort_of(sort, "S.No"),
            Length::Shrink
        ))
        .width(layout.cell("S.No"))
        .align_x(iced::alignment::Horizontal::Center),
        container(check_box(theme, check, on_toggle)).width(Length::Fixed(44.0)),
    ]
    .spacing(spacing::MD)
    .align_y(Alignment::Center);
    for (index, label) in VENDOR_HEADERS.iter().enumerate().skip(2) {
        if visible(index) {
            header = header.push(column_label(
                theme,
                label,
                sort_of(sort, label),
                layout.cell(label),
            ));
        }
    }
    header = header.push(Space::with_width(Length::Fixed(
        spacing::ACTION_COLUMN_WIDTH,
    )));
    container(header)
        .padding([spacing::SPACE_10, spacing::MD])
        .width(Length::Fixed(layout.total_width()))
        .style(table_style::header_row(theme, 0.0))
        .into()
}

/// One table row: select box, avatar cell, status pill and the edit
/// button; the whole row opens the detail panel.
fn vendor_row(
    theme: Theme,
    vendor: &Vendor,
    serial_number: usize,
    layout: &TableLayout,
    selected: bool,
    highlighted: bool,
    visible: &dyn Fn(usize) -> bool,
) -> Element<'static, Message> {
    let id = vendor.id.unwrap_or_default();
    let mut cells = row![
        container(text(serial_number.to_string()).size(typography::body()))
            .width(layout.cell("S.No"))
            .align_x(iced::alignment::Horizontal::Center),
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
            "Vendor Name" => labels::strong(theme, vendor.name.clone(), typography::label())
                .wrapping(Wrapping::None)
                .into(),
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
            .wrapping(Wrapping::None)
            .into(),
        };
        cells = cells.push(container(cell).width(layout.cell(label)).clip(true));
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
        .padding([0.0, spacing::MD])
        .width(Length::Fixed(layout.total_width()))
        .style(table_style::row(theme, highlighted))
        .on_press(Message::EditRow(id))
        .into()
}

fn compare_by_column(a: &Vendor, b: &Vendor, label: &str) -> std::cmp::Ordering {
    match label {
        "Vendor Name" => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        "Payment Terms" => a.payment_terms_days.cmp(&b.payment_terms_days),
        "Status" => a.is_active.cmp(&b.is_active),
        _ => vendor_column_value(a, label)
            .to_lowercase()
            .cmp(&vendor_column_value(b, label).to_lowercase()),
    }
}

fn vendor_column_value(vendor: &Vendor, label: &str) -> String {
    match label {
        "Vendor ID" => vendor
            .reference_code
            .clone()
            .or_else(|| vendor.id.map(|id| id.to_string()))
            .unwrap_or_default(),
        "Company Name" | "Website" | "Mailing Address" | "Shipping Address" | "Payment Method"
        | "Attachments" | "Description" | "Create Timestamp" | "Modified Timestamp"
        | "Custom Fields" | "Debit" | "Credit" | "Balance" => "--".into(),
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

/// Card footer: Previous, numbered pages, Next; result count and the
/// rows-per-page selector. A narrow card stacks the two groups on two
/// lines instead of overflowing.
fn footer(
    theme: Theme,
    class: spacing::LayoutClass,
    vendors: &VendorPage<'_>,
) -> Element<'static, Message> {
    let (page, pages) = (vendors.page, vendors.pages);
    let mut numbers = row![].spacing(spacing::SPACE_4).align_y(Alignment::Center);
    for item in page_items(page, pages) {
        numbers = numbers.push(match item {
            Some(index) => page_number(theme, index, index == page),
            None => labels::muted(theme, "...", typography::label()).into(),
        });
    }
    let pager = row![
        pager_button(theme, "Previous", (page > 0).then_some(Message::PrevPage)),
        numbers,
        pager_button(
            theme,
            "Next",
            (page + 1 < pages).then_some(Message::GoToPage(page + 1))
        ),
    ]
    .spacing(spacing::SPACE_12)
    .align_y(Alignment::Center);
    let summary = row![
        labels::muted(
            theme,
            results_label(vendors.rows.len(), vendors.total),
            typography::label()
        )
        .wrapping(Wrapping::None),
        pick_list(PAGE_SIZES, Some(vendors.size), Message::PageSizeChanged)
            .padding([spacing::SPACE_6, spacing::SPACE_12])
            .text_size(typography::label())
            .style(dropdown::style(theme))
            .menu_style(dropdown::dropdown_menu(theme)),
    ]
    .spacing(spacing::SPACE_12)
    .align_y(Alignment::Center);
    if class == spacing::LayoutClass::Narrow {
        column![pager, summary]
            .spacing(spacing::SPACE_12)
            .padding(spacing::SPACE_20)
            .into()
    } else {
        row![pager, Space::with_width(Length::Fill), summary]
            .align_y(Alignment::Center)
            .padding(spacing::SPACE_20)
            .into()
    }
}

/// The card's top block, two rows `SPACE_20` apart. Row 1 is the title
/// block with the Add button on the right; row 2 is the toolbar: search
/// (taking the spare width), the filter dropdowns, then the tool buttons.
/// A narrow card wraps the toolbar: search on its own line, filters and
/// tools below it.
fn card_header<'a, M: 'a>(
    class: spacing::LayoutClass,
    title: Element<'a, M>,
    search: Element<'a, M>,
    filters: Element<'a, M>,
    tools: Element<'a, M>,
    add: Element<'a, M>,
) -> Element<'a, M> {
    let line = |items: Vec<Element<'a, M>>| {
        Row::with_children(items)
            .spacing(spacing::SPACE_12)
            .align_y(Alignment::Center)
    };
    let page_header = line(vec![
        container(title).width(Length::Fill).clip(true).into(),
        add,
    ]);
    let toolbar: Element<'a, M> = if class == spacing::LayoutClass::Narrow {
        column![search, line(vec![filters, tools])]
            .spacing(spacing::SPACE_12)
            .into()
    } else {
        line(vec![search, filters, tools]).into()
    };
    column![page_header, toolbar]
        .spacing(spacing::SPACE_20)
        .padding(spacing::SPACE_20)
        .into()
}

/// A small muted label above a form input.
fn field<'a>(
    theme: Theme,
    icon: &'static str,
    label: &'a str,
    input: iced::widget::TextInput<'a, Message>,
) -> Element<'a, Message> {
    field_element(
        theme,
        icon,
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
    icon: &'static str,
    label: &'a str,
    input: Element<'a, Message>,
) -> Element<'a, Message> {
    forms::field_box(theme, icon, label, input)
}

fn readonly_field(
    theme: Theme,
    icon: &'static str,
    label: &'static str,
    value: impl Into<String>,
) -> Element<'static, Message> {
    field_element(
        theme,
        icon,
        label,
        text(value.into())
            .size(typography::body())
            .color(theme.neutral_font())
            .into(),
    )
}

/// Lays `fields` out `columns` to a line; a short last line is padded so
/// its fields keep the same width as the ones above.
fn field_grid<'a>(fields: Vec<Element<'a, Message>>, columns: usize) -> Element<'a, Message> {
    let mut grid = column![].spacing(spacing::MD).width(Length::Fill);
    let mut remaining = fields.into_iter();
    loop {
        let mut line = row![].spacing(spacing::SM).width(Length::Fill);
        let mut placed = 0;
        for field in remaining.by_ref().take(columns) {
            line = line.push(field);
            placed += 1;
        }
        if placed == 0 {
            break;
        }
        for _ in placed..columns {
            line = line.push(Space::with_width(Length::FillPortion(1)));
        }
        grid = grid.push(line);
    }
    grid.into()
}

/// The fields of the active detail tab, in reading order.
fn detail_fields(state: &State, theme: Theme) -> Vec<Element<'_, Message>> {
    let editing = state
        .editing_id
        .and_then(|id| state.vendors.iter().find(|r| r.id == Some(id)));
    let vendor_id = editing
        .and_then(|vendor| vendor.reference_code.clone())
        .unwrap_or_else(|| "Assigned on save".into());
    match state.form_tab {
        0 => vec![
            readonly_field(theme, icons::FIELD_OTHER, "S.No", "Auto-generated"),
            readonly_field(theme, icons::FIELD_OTHER, "Vendor ID", vendor_id),
            field(
                theme,
                icons::FIELD_NAME,
                "Vendor name",
                text_input("Vendor name", &state.name_input).on_input(Message::NameChanged),
            ),
            readonly_field(theme, icons::FIELD_COMPANY, "Company Name", "--"),
            field_element(
                theme,
                icons::FIELD_OTHER,
                "Vendor type",
                pick_list(
                    VENDOR_TYPES,
                    Some(state.vendor_type_input.as_str()),
                    |value| Message::VendorTypeChanged(value.to_string()),
                )
                .style(dropdown::style(theme))
                .padding(spacing::SM)
                .width(Length::Fill)
                .into(),
            ),
        ],
        1 => vec![
            field(
                theme,
                icons::FIELD_PHONE,
                "Phone (optional)",
                text_input("Phone (optional)", &state.phone_input).on_input(Message::PhoneChanged),
            ),
            field(
                theme,
                icons::FIELD_EMAIL,
                "Email (optional)",
                text_input("Email (optional)", &state.email_input).on_input(Message::EmailChanged),
            ),
            readonly_field(theme, icons::FIELD_WEBSITE, "Website", "--"),
        ],
        2 => vec![
            field(
                theme,
                icons::FIELD_ADDRESS,
                "Mailing Address",
                text_input("Mailing Address", &state.mailing_address_input)
                    .on_input(Message::MailingAddressChanged),
            ),
            readonly_field(theme, icons::FIELD_ADDRESS, "Shipping Address", "--"),
        ],
        3 => vec![
            field(
                theme,
                icons::FIELD_PAYMENT,
                "Payment Terms",
                text_input("30", &state.payment_terms_input).on_input(Message::PaymentTermsChanged),
            ),
            field(
                theme,
                icons::FIELD_PAYMENT,
                "Payment Method",
                text_input("Payment Method", &state.payment_method_input)
                    .on_input(Message::PaymentMethodChanged),
            ),
            field(
                theme,
                icons::FIELD_OTHER,
                "Tax ID (optional)",
                text_input("Tax ID (optional)", &state.tax_id_input)
                    .on_input(Message::TaxIdChanged),
            ),
            field(
                theme,
                icons::FIELD_NOTE,
                "Description",
                text_input("Description", &state.description_input)
                    .on_input(Message::DescriptionChanged),
            ),
            readonly_field(theme, icons::FIELD_OTHER, "Attachments", "--"),
        ],
        4 => vec![
            field_element(
                theme,
                icons::FIELD_STATUS,
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
            readonly_field(theme, icons::FIELD_OTHER, "Create Timestamp", "--"),
            readonly_field(theme, icons::FIELD_OTHER, "Modified Timestamp", "--"),
        ],
        _ => vec![readonly_field(
            theme,
            icons::FIELD_OTHER,
            "Custom Fields",
            "--",
        )],
    }
}

/// Name, contact line and status pill at the top of the detail panel.
fn vendor_summary(state: &State, theme: Theme) -> Element<'static, Message> {
    let name = if state.name_input.is_empty() {
        "New Vendor".to_string()
    } else {
        state.name_input.clone()
    };
    let archived = state.status_input == "Inactive";
    container(
        row![
            column![
                labels::strong(theme, name, typography::emphasis()),
                labels::muted(
                    theme,
                    format!(
                        "{}  ·  {}",
                        if state.phone_input.is_empty() {
                            "--"
                        } else {
                            &state.phone_input
                        },
                        if state.email_input.is_empty() {
                            "--"
                        } else {
                            &state.email_input
                        }
                    ),
                    typography::caption(),
                ),
            ]
            .spacing(spacing::SPACE_4)
            .width(Length::Fill),
            status_pill(
                theme,
                if archived { "Archived" } else { "Active" },
                if archived {
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
    .into()
}

/// Live, read-only echo of Mailing Address, Payment Method and Description
/// shown above the Basic Information tab -- it mirrors whatever is typed
/// into those fields further down the form, in both the "New Vendor" and
/// editing states.
fn vendor_live_preview(state: &State, theme: Theme) -> Element<'static, Message> {
    let value = |input: &str| {
        text(if input.is_empty() {
            "--".to_string()
        } else {
            input.to_string()
        })
        .size(typography::body())
        .color(theme.neutral_font())
        .into()
    };
    field_grid(
        vec![
            forms::field_box(
                theme,
                icons::FIELD_ADDRESS,
                "Mailing Address",
                value(&state.mailing_address_input),
            ),
            forms::field_box(
                theme,
                icons::FIELD_PAYMENT,
                "Payment Method",
                value(&state.payment_method_input),
            ),
            forms::field_box(
                theme,
                icons::FIELD_NOTE,
                "Description",
                value(&state.description_input),
            ),
        ],
        2,
    )
}

/// The Vendor Details card: two field columns, Save/Close.
fn detail_form(state: &State, theme: Theme) -> Element<'_, Message> {
    let footer = row![
        bold_button("Save", [spacing::SM, spacing::MD])
            .style(button_style::mode(theme, true))
            .on_press(Message::Create),
        bold_button("Close", [spacing::SM, spacing::MD])
            .style(button_style::outlined(theme, false))
            .on_press(Message::CancelEdit),
    ]
    .spacing(spacing::SM)
    .into();
    let summary = column![
        vendor_summary(state, theme),
        vendor_live_preview(state, theme)
    ]
    .spacing(spacing::MD)
    .width(Length::Fill);
    forms::detail_panel(forms::DetailPanel {
        theme,
        title: "Vendor Details",
        tab_labels: &VENDOR_FORM_TABS,
        active_tab: state.form_tab,
        on_select_tab: Message::FormTabChanged,
        summary: Some(summary.into()),
        content: field_grid(detail_fields(state, theme), 1),
        footer,
        on_preview: Message::Preview,
    })
}

/// The column picker in its own card.
fn columns_card(state: &State, theme: Theme) -> Element<'_, Message> {
    let optional_visibility = optional_column_visibility(state);
    container(data_table::columns_panel(
        theme,
        &VENDOR_OPTIONAL_COLUMNS,
        &optional_visibility,
        Message::ColumnToggled,
        Message::SelectAllColumns,
        Message::DeselectAllColumns,
        Message::ResetColumns,
    ))
    .padding(spacing::MD)
    .width(Length::Fill)
    .height(Length::Fill)
    .style(panels::flat_card(theme, states::RADIUS_LG))
    .into()
}

/// Everything right of (or, when narrow, instead of) the table: the detail
/// form and/or the column picker, with a Back button when it replaces the
/// table.
fn side_panel(
    state: &State,
    theme: Theme,
    class: spacing::LayoutClass,
    width: f32,
) -> Element<'_, Message> {
    let narrow = class == spacing::LayoutClass::Narrow;
    let mut side = column![]
        .spacing(spacing::SPACE_20)
        .width(Length::Fixed(width))
        .height(Length::Fill);
    if narrow {
        let close = if state.form_open {
            Message::CancelEdit
        } else {
            Message::ToggleColumns
        };
        side = side.push(
            icon_label_button(
                icons::ARROW_LEFT,
                "Back",
                [spacing::SPACE_6, spacing::SPACE_14],
            )
            .style(button_style::outlined(theme, false))
            .on_press(close),
        );
    }
    if state.form_open {
        side = side.push(detail_form(state, theme));
    }
    if state.columns_open {
        side = side.push(columns_card(state, theme));
    }
    side.into()
}

/// Search field with the magnifier glyph laid over its left inset; takes
/// all the width its row leaves over.
fn search_box(state: &State, theme: Theme) -> Element<'_, Message> {
    let input = text_input("Search vendors by name or email", &state.search)
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
    stack![
        input,
        container(
            icons::icon(icons::SEARCH)
                .size(typography::label())
                .color(theme.neutral_front()),
        )
        .padding([spacing::SPACE_10, spacing::SPACE_12])
    ]
    .width(Length::Fill)
    .into()
}

/// The vendor-type and status dropdowns, each showing its counts.
fn filter_row(state: &State, theme: Theme) -> Element<'static, Message> {
    let selected_vendor_type = if state.vendor_type_filter.is_empty() {
        "All"
    } else {
        state.vendor_type_filter.as_str()
    };
    let vendor_type_options: Vec<(&'static str, usize, Message, bool)> = VENDOR_TYPE_OPTIONS
        .iter()
        .map(|&label| {
            let count = if label == "All" {
                state.vendors.len()
            } else {
                state
                    .vendors
                    .iter()
                    .filter(|vendor| vendor.vendor_type == label)
                    .count()
            };
            (
                label,
                count,
                Message::VendorTypeFilterChanged(label.to_string()),
                label == selected_vendor_type,
            )
        })
        .collect();
    let active_vendor_count = state
        .vendors
        .iter()
        .filter(|vendor| vendor.is_active)
        .count();
    let status_options: Vec<(&'static str, usize, Message, bool)> = vec![
        (
            "All",
            state.vendors.len(),
            Message::StatusFilterChanged(StatusFilter::All),
            state.status_filter == StatusFilter::All,
        ),
        (
            "Active",
            active_vendor_count,
            Message::StatusFilterChanged(StatusFilter::Active),
            state.status_filter == StatusFilter::Active,
        ),
        (
            "Archived",
            state.vendors.len() - active_vendor_count,
            Message::StatusFilterChanged(StatusFilter::Inactive),
            state.status_filter == StatusFilter::Inactive,
        ),
    ];
    row![
        filter_dropdown(theme, "Vendor type", vendor_type_options),
        filter_dropdown(theme, "Status", status_options),
    ]
    .spacing(spacing::SPACE_12)
    .align_y(Alignment::Center)
    .into()
}

/// Reset, Columns and Export icon buttons.
fn tool_buttons(theme: Theme) -> Element<'static, Message> {
    row![
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
    ]
    .spacing(spacing::SPACE_12)
    .align_y(Alignment::Center)
    .into()
}

/// Header, table and footer inside one card, `card_width` wide.
fn vendor_card<'a, M: Clone + 'a>(
    state: &'a State,
    theme: Theme,
    card_width: f32,
    switcher: Option<Element<'a, M>>,
    wrap: impl Fn(Message) -> M + Clone + 'a,
) -> Element<'a, M> {
    let lift = |element: Element<'a, Message>| element.map(wrap.clone());
    let class = spacing::layout_class(card_width);
    let vendors = vendor_page(state);

    let mut titles =
        row![labels::strong(theme, TITLE, typography::panel_title()).wrapping(Wrapping::None),]
            .spacing(spacing::SPACE_12)
            .align_y(Alignment::Center);
    if let Some(switcher) = switcher {
        titles = titles.push(switcher);
    }
    let title: Element<'a, M> = titles.into();
    let add = lift(
        icon_label_button(icons::ADD, "Add", [spacing::SPACE_10, spacing::MD])
            .style(button_style::mode(theme, true))
            .on_press(Message::OpenForm)
            .into(),
    );
    let header = card_header(
        class,
        title,
        lift(search_box(state, theme)),
        lift(filter_row(state, theme)),
        lift(tool_buttons(theme)),
        add,
    );

    let row_ids: Vec<i64> = vendors
        .rows
        .iter()
        .map(|v| v.id.unwrap_or_default())
        .collect();
    let selected_here = row_ids
        .iter()
        .filter(|id| state.selected.contains(id))
        .count();
    let check = CheckState::from_counts(selected_here, row_ids.len());
    let toggle_all = Message::SelectAllChanged(row_ids, check != CheckState::Checked);
    let layout = table_layout(
        state,
        card_width - spacing::SCROLLBAR_WIDTH - spacing::SPACE_2,
    );
    let table = table_area(state, theme, &vendors, check, toggle_all, &layout);

    let mut card = column![header, lift(table)];
    if let Some(message) = &state.message {
        card = card.push(lift(
            container(text(message.clone()).color(theme.positive_front()))
                .padding([spacing::SPACE_6, spacing::SPACE_20])
                .into(),
        ));
    }
    if let Some(error) = &state.error {
        card = card.push(lift(
            container(text(error.clone()).color(theme.negative_front()))
                .padding([spacing::SPACE_6, spacing::SPACE_20])
                .into(),
        ));
    }
    card = card.push(lift(
        container(Space::with_height(Length::Fixed(spacing::SPACE_1)))
            .width(Length::Fill)
            .style(table_style::divider(theme))
            .into(),
    ));
    card = card.push(lift(footer(theme, class, &vendors)));
    container(card)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(1)
        .style(table_style::card(theme, states::RADIUS_LG))
        .into()
}

/// Keeps the header fixed vertically while the rows scroll inside the
/// card; the shared horizontal scroll keeps the header aligned with them.
fn table_area(
    state: &State,
    theme: Theme,
    vendors: &VendorPage<'_>,
    check: CheckState,
    toggle_all: Message,
    layout: &TableLayout,
) -> Element<'static, Message> {
    let visible_column = |index: usize| column_visible(state, index);
    let header_row = table_header(
        theme,
        state.sort,
        check,
        toggle_all,
        &visible_column,
        layout,
    );
    let mut rows = column![];
    for (index, vendor) in vendors.rows.iter().enumerate() {
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
            vendors.page * vendors.size + index + 1,
            layout,
            state.selected.contains(&id),
            state.selected.contains(&id) || (state.form_open && state.editing_id == vendor.id),
            &visible_column,
        ));
    }
    let rows = scrollable(rows.width(Length::Fixed(layout.total_width())))
        .direction(scrollbar::vertical())
        .height(Length::Fill);
    scrollable(
        container(
            column![header_row, rows]
                .width(Length::Fixed(layout.total_width()))
                .spacing(spacing::SPACE_1)
                .height(Length::Fill),
        )
        .width(Length::Fixed(layout.total_width()))
        .height(Length::Shrink),
    )
    .direction(scrollable::Direction::Horizontal(scrollbar::bar()))
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

/// The whole Vendors page for a page area `width` wide.
fn page<'a, M: Clone + 'a>(
    state: &'a State,
    theme: Theme,
    width: f32,
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
    let inner = (width - spacing::SPACE_20 * 2.0).max(0.0);
    let class = spacing::layout_class(inner);
    let split = page_split(inner, state.form_open || state.columns_open);
    let mut main = row![].spacing(spacing::SPACE_20).height(Length::Fill);
    if let Some(card_width) = split.card {
        main = main.push(vendor_card(
            state,
            theme,
            card_width,
            switcher,
            wrap.clone(),
        ));
    }
    if let Some(panel_width) = split.panel {
        main = main.push(lift(side_panel(state, theme, class, panel_width)));
    }
    container(main)
        .padding(spacing::SPACE_20)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

pub fn view(state: &State, theme: Theme) -> Element<'_, Message> {
    body(state, theme, || None, |message| message)
}

/// Same as [`view`], except the title sits inline with the switcher (a
/// type-switcher dropdown, e.g. "Maintain"'s section picker) in the
/// card header -- `wrap` converts this module's own `Message` into
/// whatever the caller's `switcher` produces, so the two can share one
/// `Element` tree despite having different message types. `switcher`
/// builds a fresh dropdown each time the page is laid out.
pub fn page_header_view_with_switcher<'a, ParentMessage: Clone + 'a>(
    state: &'a State,
    theme: Theme,
    switcher: impl Fn() -> Element<'a, ParentMessage> + 'a,
    wrap: impl Fn(Message) -> ParentMessage + Clone + 'a,
) -> Element<'a, ParentMessage> {
    body(state, theme, move || Some(switcher()), wrap)
}

/// Rebuilds the page whenever the available size changes.
fn body<'a, M: Clone + 'a>(
    state: &'a State,
    theme: Theme,
    switcher: impl Fn() -> Option<Element<'a, M>> + 'a,
    wrap: impl Fn(Message) -> M + Clone + 'a,
) -> Element<'a, M> {
    responsive(move |size| page(state, theme, size.width, switcher(), wrap.clone())).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::organization::companies::Company;
    use crate::backend::repositories::organization_repository::{self, OrganizationRepository};
    use crate::backend::repositories::party_repository;
    use crate::database::database::Database;
    use std::path::PathBuf;

    #[test]
    fn clicking_a_header_sorts_ascending_then_descending_then_resets() {
        let mut state = State::default();
        state.vendors = ["b", "C", "a"]
            .iter()
            .map(|name| Vendor::new(1, *name))
            .collect();
        let names = |state: &State| -> Vec<String> {
            vendor_page(state)
                .rows
                .iter()
                .map(|v| v.name.clone())
                .collect()
        };
        let ctx = ctx_with_company();
        update(&mut state, Message::SortBy("Vendor Name"), &ctx);
        assert_eq!(names(&state), ["a", "b", "C"]);
        update(&mut state, Message::SortBy("Vendor Name"), &ctx);
        assert_eq!(names(&state), ["C", "b", "a"]);
        update(&mut state, Message::SortBy("Vendor Name"), &ctx);
        assert_eq!(names(&state), ["b", "C", "a"]);
    }

    const WIDTHS: [f32; 4] = [640.0, 900.0, 1366.0, 1920.0];
    const THEMES: [Theme; 2] = [Theme::Light, Theme::Dark];

    fn ctx_with_company() -> Ctx {
        let db = Database::open_in_memory().unwrap();
        db.run_migrations(organization_repository::MIGRATIONS)
            .unwrap();
        db.run_migrations(party_repository::MIGRATIONS).unwrap();
        let company_id = OrganizationRepository::new(db.clone())
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

    fn vendors(count: usize, long_name: bool) -> Vec<Vendor> {
        (0..count)
            .map(|index| {
                let name = if long_name {
                    "A Very Long Vendor Name Trading And Distribution Company Limited ".repeat(3)
                } else {
                    format!("Vendor {index}")
                };
                let mut vendor = Vendor::new(1, name);
                vendor.id = Some(index as i64 + 1);
                vendor.is_active = index % 3 != 0;
                vendor
            })
            .collect()
    }

    fn state_with(vendors: Vec<Vendor>) -> State {
        State {
            vendors,
            column_visible: default_column_visibility(),
            ..Default::default()
        }
    }

    #[test]
    fn one_active_vendor_shows_one_row_and_a_one_of_one_footer() {
        let ctx = ctx_with_company();
        let mut state = load(&ctx);
        update(&mut state, Message::OpenForm, &ctx);
        update(
            &mut state,
            Message::NameChanged("Acme Supplies".into()),
            &ctx,
        );
        update(&mut state, Message::Create, &ctx);
        assert!(state.error.is_none(), "{:?}", state.error);

        // Reload from the database, as the app does when the page opens.
        let state = load(&ctx);
        assert_eq!(state.vendors.len(), 1);
        let page = vendor_page(&state);

        assert_eq!(page.rows.len(), 1);
        assert_eq!(page.total, 1);
        assert_eq!(
            results_label(page.rows.len(), page.total),
            "Showing 1 of 1 results"
        );
    }

    #[test]
    fn the_default_state_filters_nothing_out() {
        let state = state_with(vendors(1, false));
        let page = vendor_page(&state);

        assert_eq!(state.status_filter, StatusFilter::All);
        assert_eq!(page.rows.len(), 1);
        assert_eq!(
            results_label(page.rows.len(), page.total),
            "Showing 1 of 1 results"
        );
    }

    #[test]
    fn a_status_filter_with_no_matches_empties_the_table_until_reset() {
        let mut state = state_with(vendors(1, false));
        state.vendors[0].is_active = true;
        let ctx = ctx_with_company();

        update(
            &mut state,
            Message::StatusFilterChanged(StatusFilter::Inactive),
            &ctx,
        );
        let page = vendor_page(&state);
        assert_eq!(
            results_label(page.rows.len(), page.total),
            "Showing 0 of 0 results"
        );

        update(&mut state, Message::ResetAll, &ctx);
        assert_eq!(vendor_page(&state).rows.len(), 1);
    }

    #[test]
    fn paging_clamps_a_stale_page_index() {
        let mut state = state_with(vendors(30, false));
        state.page_size = 10;
        state.page = 99;
        let page = vendor_page(&state);

        assert_eq!(page.pages, 3);
        assert_eq!(page.page, 2);
        assert_eq!(page.rows.len(), 10);
    }

    #[test]
    fn layout_class_switches_at_the_shared_breakpoints() {
        use spacing::{layout_class, LayoutClass};
        assert_eq!(layout_class(799.0), LayoutClass::Narrow);
        assert_eq!(layout_class(800.0), LayoutClass::Medium);
        assert_eq!(layout_class(1199.0), LayoutClass::Medium);
        assert_eq!(layout_class(1200.0), LayoutClass::Wide);
    }

    #[test]
    fn the_panel_replaces_the_table_when_narrow_and_sits_beside_it_otherwise() {
        let closed = page_split(600.0, false);
        assert_eq!((closed.card, closed.panel), (Some(600.0), None));

        let narrow = page_split(600.0, true);
        assert_eq!((narrow.card, narrow.panel), (None, Some(600.0)));

        for inner in [800.0, 1000.0, 1326.0, 1880.0] {
            let split = page_split(inner, true);
            let (card, panel) = (split.card.unwrap(), split.panel.unwrap());
            assert!(panel >= spacing::PANEL_MIN_WIDTH, "panel clips at {inner}");
            assert!((card + panel + spacing::SPACE_20 - inner).abs() < 0.01);
        }
        let wide = page_split(1500.0, true);
        assert!((wide.panel.unwrap() - 500.0).abs() < 0.01);
    }

    #[test]
    fn the_name_column_grows_to_fill_spare_width_and_never_shrinks() {
        let state = state_with(vendors(1, false));
        let tight = table_layout(&state, 0.0);
        let roomy = table_layout(&state, tight.total_width() + 300.0);

        assert_eq!(tight.name_extra, 0.0);
        assert!((roomy.name_extra - 300.0).abs() < 0.01);
        assert_eq!(roomy.cell("Vendor ID"), tight.cell("Vendor ID"));
    }

    #[test]
    fn the_page_builds_at_every_width_theme_panel_state_and_record_count() {
        for theme in THEMES {
            for width in WIDTHS {
                for (count, long_name) in [(0, false), (1, false), (60, false), (3, true)] {
                    for (form, columns) in
                        [(false, false), (true, false), (false, true), (true, true)]
                    {
                        for tab in 0..VENDOR_FORM_TABS.len() {
                            let mut state = state_with(vendors(count, long_name));
                            state.form_open = form;
                            state.columns_open = columns;
                            state.form_tab = tab;
                            state.page_size = 10;
                            let _: Element<'_, Message> =
                                page(&state, theme, width, None, |message| message);
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn the_page_builds_with_a_switcher_a_selection_and_the_preview_open() {
        for theme in THEMES {
            let mut state = state_with(vendors(2, false));
            state.selected.insert(1);
            state.editing_id = Some(1);
            state.form_open = true;
            let switcher: Element<'_, Message> = Space::with_width(Length::Fixed(220.0)).into();
            let _: Element<'_, Message> =
                page(&state, theme, 1366.0, Some(switcher), |message| message);

            state.show_preview = true;
            let _: Element<'_, Message> = page(&state, theme, 640.0, None, |message| message);
            let _: Element<'_, Message> = view(&state, theme);
        }
    }

    #[test]
    fn the_header_builds_in_two_rows_at_every_width_theme_and_panel_state() {
        for theme in THEMES {
            for width in WIDTHS {
                for panel_open in [false, true] {
                    let mut state = state_with(vendors(1, true));
                    state.form_open = panel_open;
                    for card_width in [width, page_split(width, panel_open).card.unwrap_or(width)] {
                        let _: Element<'_, Message> =
                            vendor_card(&state, theme, card_width, None, |message| message);
                    }
                }
            }
        }
        for class in [
            spacing::LayoutClass::Narrow,
            spacing::LayoutClass::Medium,
            spacing::LayoutClass::Wide,
        ] {
            let item = || -> Element<'static, Message> { Space::with_width(Length::Fill).into() };
            let _: Element<'_, Message> =
                card_header(class, item(), item(), item(), item(), item());
        }
    }

    #[test]
    fn the_field_grid_pads_a_short_last_line() {
        let fields = (0..5)
            .map(|index| {
                readonly_field(Theme::Light, icons::FIELD_OTHER, "Label", index.to_string())
            })
            .collect();
        let _: Element<'_, Message> = field_grid(fields, 2);
        let _: Element<'_, Message> = field_grid(Vec::new(), 2);
    }
}
