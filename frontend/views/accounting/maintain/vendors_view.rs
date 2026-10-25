//! Vendors screen: the table, filters, form panel, preview and the
//! messages that drive them. The non-UI logic (column names and
//! indexes, export rows, sorting, cell text, formatting) lives in
//! `backend/accounting/vendors.rs`; styles in `frontend/components/`.

use crate::backend::accounting::vendors::{
    attachment_names, compare_by_column, dash, day, default_column_visibility, export_headers,
    export_row, money, optional_input, side, sort_of, vendor_column_value, COL_ATTACHMENTS,
    COL_BALANCE, COL_COMPANY_NAME, COL_CREATE_TIMESTAMP, COL_CREDIT, COL_CUSTOM_FIELDS, COL_DEBIT,
    COL_DESCRIPTION, COL_EMAIL, COL_MAILING_ADDRESS, COL_MODIFIED_TIMESTAMP, COL_PAYMENT_METHOD,
    COL_PAYMENT_TERMS, COL_PHONE, COL_SELECT, COL_SERIAL, COL_SHIPPING_ADDRESS, COL_STATUS,
    COL_VENDOR_ID, COL_VENDOR_NAME, COL_WEBSITE, DEFAULT_PAGE_SIZE, FIRST_DATA_COLUMN, PAGE_SIZES,
    VENDOR_ALWAYS_SELECTED, VENDOR_HEADERS, VENDOR_OPTIONAL_COLUMNS, VENDOR_OPTIONAL_LABELS,
    VENDOR_STATUS_OPTIONS, VENDOR_TYPE_LABEL, VENDOR_TYPE_OPTIONS, VENDOR_VISIBLE_COLUMNS,
};
use crate::backend::core::reference_code;
use crate::backend::core::validation::Validate;
use crate::backend::parties::vendors::{
    CustomField, Vendor, VendorAttachment, VENDOR_PAYMENT_METHODS, VENDOR_TYPES,
};
use crate::backend::purchases::vendor_ledger::{self, VendorLedger};
use crate::backend::services::party_service::PartyService;
use crate::backend::services::purchase_service::PurchaseService;
use crate::database::exports::csv;
use crate::frontend::components::badges::{self, BadgeTone};
use crate::frontend::components::button as button_style;
use crate::frontend::components::checkbox::{self as checkbox_style, CheckState};
use crate::frontend::components::colors::Theme;
use crate::frontend::components::table as table_style;
use crate::frontend::components::toolbar::{self, StatusFilter};
use crate::frontend::components::{
    data_table, dropdown, forms, icons, labels, pagination, panels, scrollbar, search, spacing,
    typography,
};
use crate::frontend::views::Ctx;
use iced::widget::text::Wrapping;
use iced::widget::{
    button, column, container, responsive, row, scrollable, text, text_editor, text_input, Space,
};
use iced::{Alignment, Element, Length};
use rust_decimal::Decimal;
use std::collections::HashSet;
use std::path::Path;

const VENDOR_FORM_TABS: [&str; 6] = [
    "Basic",
    "Contact",
    "Address",
    "Additional",
    VENDOR_HEADERS[COL_STATUS],
    "Customization",
];

#[derive(Debug, Default)]
pub struct State {
    vendors: Vec<Vendor>,
    form: VendorForm,
    search: String,
    vendor_type_filter: String,
    editing_id: Option<i64>,
    selected: HashSet<i64>,
    status_filter: StatusFilter,
    columns_open: bool,
    column_visible: Vec<bool>,
    form_open: bool,
    show_preview: bool,
    /// Accounting data behind the preview; `None` for an unsaved vendor.
    ledger: Option<VendorLedger>,
    ledger_error: Option<String>,
    preview_tab: usize,
    ledger_newest_first: bool,
    page: usize,
    page_size: usize,
    /// The sorted column (an index into `VENDOR_HEADERS`) and whether
    /// it is ascending.
    sort: Option<(usize, bool)>,
    message: Option<String>,
    error: Option<String>,
}

/// Everything typed into the Vendor Details panel.
#[derive(Debug, Default)]
struct VendorForm {
    /// Vendor ID shown in the form; generated when "New Vendor" opens so
    /// it is the same code that gets saved.
    reference_code: String,
    name: String,
    company_name: String,
    email: String,
    phone: String,
    tax_id: String,
    vendor_type: String,
    status: String,
    payment_terms: String,
    website: String,
    mailing_address: String,
    shipping_address: String,
    payment_method: String,
    description: text_editor::Content,
    custom_fields: Vec<CustomField>,
    attachments: Vec<VendorAttachment>,
    /// The attachment being renamed and its draft display name.
    renaming: Option<(usize, String)>,
    tab: usize,
}

impl VendorForm {
    /// A blank form for a new vendor, with the usual defaults.
    fn new_vendor(reference_code: String) -> Self {
        Self {
            reference_code,
            vendor_type: "Other".into(),
            status: "Active".into(),
            payment_terms: "30".into(),
            ..Self::default()
        }
    }

    fn from_vendor(vendor: &Vendor) -> Self {
        let text = |value: &Option<String>| value.clone().unwrap_or_default();
        Self {
            reference_code: text(&vendor.reference_code),
            name: vendor.name.clone(),
            company_name: text(&vendor.company_name),
            email: text(&vendor.email),
            phone: text(&vendor.phone),
            tax_id: text(&vendor.tax_id),
            vendor_type: vendor.vendor_type.clone(),
            status: if vendor.is_active {
                "Active"
            } else {
                "Inactive"
            }
            .into(),
            payment_terms: vendor.payment_terms_days.to_string(),
            website: text(&vendor.website),
            mailing_address: text(&vendor.mailing_address),
            shipping_address: text(&vendor.shipping_address),
            payment_method: text(&vendor.payment_method),
            description: text_editor::Content::with_text(&text(&vendor.description)),
            custom_fields: vendor.custom_fields.clone(),
            attachments: vendor.attachments.clone(),
            renaming: None,
            tab: 0,
        }
    }

    /// The description as typed, without the editor's trailing newline.
    fn description_text(&self) -> String {
        self.description.text().trim_end_matches('\n').to_string()
    }

    /// `vendor` with every field of this form written over it. Custom
    /// fields left completely blank are dropped.
    fn apply(&self, vendor: Vendor, payment_terms_days: i32) -> Vendor {
        Vendor {
            name: self.name.clone(),
            company_name: optional_input(&self.company_name),
            email: optional_input(&self.email),
            phone: optional_input(&self.phone),
            tax_id: optional_input(&self.tax_id),
            vendor_type: self.vendor_type.clone(),
            is_active: self.status == "Active",
            payment_terms_days,
            website: optional_input(&self.website),
            mailing_address: optional_input(&self.mailing_address),
            shipping_address: optional_input(&self.shipping_address),
            payment_method: optional_input(&self.payment_method),
            description: optional_input(&self.description_text()),
            custom_fields: self
                .custom_fields
                .iter()
                .filter(|field| !field.name.trim().is_empty() || !field.value.trim().is_empty())
                .cloned()
                .collect(),
            attachments: self.attachments.clone(),
            ..vendor
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    NameChanged(String),
    CompanyNameChanged(String),
    EmailChanged(String),
    PhoneChanged(String),
    TaxIdChanged(String),
    VendorTypeChanged(String),
    VendorTypeFilterChanged(String),
    StatusChanged(String),
    PaymentTermsChanged(String),
    WebsiteChanged(String),
    MailingAddressChanged(String),
    ShippingAddressChanged(String),
    PaymentMethodChanged(String),
    DescriptionEdited(text_editor::Action),
    CustomFieldAdded,
    CustomFieldNameChanged(usize, String),
    CustomFieldValueChanged(usize, String),
    CustomFieldRemoved(usize),
    AddAttachment,
    AttachmentRenameStarted(usize),
    AttachmentRenameChanged(String),
    AttachmentRenameDone,
    AttachmentRemoved(usize),
    FormTabChanged(usize),
    OpenForm,
    Preview,
    PreviewTabChanged(usize),
    ToggleLedgerOrder,
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
    /// Sort by the column at this index into `VENDOR_HEADERS`.
    SortBy(usize),
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
    let form = &mut state.form;
    match message {
        Message::NameChanged(value) => form.name = value,
        Message::CompanyNameChanged(value) => form.company_name = value,
        Message::EmailChanged(value) => form.email = value,
        Message::PhoneChanged(value) => form.phone = value,
        Message::TaxIdChanged(value) => form.tax_id = value,
        Message::VendorTypeChanged(value) => form.vendor_type = value,
        Message::VendorTypeFilterChanged(value) => {
            state.vendor_type_filter = value;
            state.page = 0;
        }
        Message::StatusChanged(value) => form.status = value,
        Message::PaymentTermsChanged(value) => form.payment_terms = value,
        Message::WebsiteChanged(value) => form.website = value,
        Message::MailingAddressChanged(value) => form.mailing_address = value,
        Message::ShippingAddressChanged(value) => form.shipping_address = value,
        Message::PaymentMethodChanged(value) => form.payment_method = value,
        Message::DescriptionEdited(action) => form.description.perform(action),
        Message::CustomFieldAdded => form.custom_fields.push(CustomField::default()),
        Message::CustomFieldNameChanged(index, value) => {
            if let Some(field) = form.custom_fields.get_mut(index) {
                field.name = value;
            }
        }
        Message::CustomFieldValueChanged(index, value) => {
            if let Some(field) = form.custom_fields.get_mut(index) {
                field.value = value;
            }
        }
        Message::CustomFieldRemoved(index) => {
            if index < form.custom_fields.len() {
                form.custom_fields.remove(index);
            }
        }
        Message::AddAttachment => {
            if let Some(path) = rfd::FileDialog::new().pick_file() {
                attach_file(state, ctx, &path);
            }
        }
        Message::AttachmentRenameStarted(index) => {
            form.renaming = form
                .attachments
                .get(index)
                .map(|attachment| (index, attachment.display_name.clone()));
        }
        Message::AttachmentRenameChanged(value) => {
            if let Some((_, draft)) = &mut form.renaming {
                *draft = value;
            }
        }
        Message::AttachmentRenameDone => {
            if let Some((index, draft)) = form.renaming.take() {
                let draft = draft.trim();
                if let (Some(attachment), false) =
                    (form.attachments.get_mut(index), draft.is_empty())
                {
                    attachment.display_name = draft.to_string();
                }
            }
        }
        Message::AttachmentRemoved(index) => {
            if index < form.attachments.len() {
                form.attachments.remove(index);
                form.renaming = None;
            }
        }
        Message::FormTabChanged(index) => form.tab = index.min(VENDOR_FORM_TABS.len() - 1),
        Message::Preview => {
            state.show_preview = !state.show_preview;
            if state.show_preview {
                refresh_ledger(state, ctx);
            }
        }
        Message::PreviewTabChanged(index) => state.preview_tab = index.min(PREVIEW_TABS.len() - 1),
        Message::ToggleLedgerOrder => state.ledger_newest_first = !state.ledger_newest_first,
        Message::OpenForm => {
            state.message = None;
            state.error = None;
            state.editing_id = None;
            state.form = VendorForm::new_vendor(reference_code::generate_code("VEND"));
            state.form_open = true;
            state.show_preview = false;
        }
        Message::Create => save(state, ctx),
        Message::EditRow(id) => {
            state.message = None;
            state.error = None;
            if let Some(vendor) = state.vendors.iter().find(|v| v.id == Some(id)) {
                state.form = VendorForm::from_vendor(vendor);
                state.editing_id = Some(id);
                state.form_open = true;
                state.show_preview = false;
            }
        }
        Message::CancelEdit => {
            close_form(state);
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
        Message::SortBy(column) => {
            state.sort = match state.sort {
                Some((current, true)) if current == column => Some((column, false)),
                Some((current, false)) if current == column => None,
                _ => Some((column, true)),
            };
            state.page = 0;
        }
        Message::PrevPage => state.page = state.page.saturating_sub(1),
        Message::GoToPage(index) => state.page = index,
        Message::BulkExportCsv => export_csv(state, ctx),
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
            if let Some(&column) = VENDOR_OPTIONAL_COLUMNS.get(index) {
                if state.column_visible.len() != VENDOR_HEADERS.len() {
                    state.column_visible = default_column_visibility();
                }
                let was_visible = column_visible(state, column);
                state.column_visible[column] = !was_visible;
            }
        }
        Message::SelectAllColumns => set_optional_columns(state, true),
        Message::DeselectAllColumns => set_optional_columns(state, false),
        Message::ResetColumns => state.column_visible = default_column_visibility(),
    }
}

/// Creates the vendor from the form, or saves the form over the vendor
/// being edited, then closes the form.
fn save(state: &mut State, ctx: &Ctx) {
    state.message = None;
    state.error = None;
    let Ok(payment_terms_days) = state.form.payment_terms.trim().parse::<i32>() else {
        state.error = Some("Payment terms must be a whole number of days.".into());
        return;
    };
    let base = match state.editing_id {
        Some(id) => match state.vendors.iter().find(|v| v.id == Some(id)) {
            Some(existing) => existing.clone(),
            None => {
                state.error = Some("that vendor no longer exists".into());
                state.editing_id = None;
                return;
            }
        },
        None => {
            let mut vendor = Vendor::new(ctx.company_id, "");
            if !state.form.reference_code.is_empty() {
                vendor.reference_code = Some(state.form.reference_code.clone());
            }
            vendor
        }
    };
    let vendor = state.form.apply(base, payment_terms_days);
    if let Err(err) = vendor.validate() {
        state.error = Some(err.to_string());
        return;
    }
    let service = PartyService::new(ctx.db.clone());
    let saved = match vendor.id {
        Some(id) => service.update_vendor(&vendor).map(|()| {
            if let Some(slot) = state.vendors.iter_mut().find(|v| v.id == Some(id)) {
                *slot = vendor;
            }
            state.message = Some("Saved changes.".into());
        }),
        None => service
            .create_vendor(&vendor)
            .map(|created| state.vendors.push(created)),
    };
    match saved {
        Ok(()) => close_form(state),
        Err(err) => state.error = Some(err.to_string()),
    }
}

fn close_form(state: &mut State) {
    state.editing_id = None;
    state.form_open = false;
    state.show_preview = false;
    state.form = VendorForm::default();
}

/// Copies `source` into the uploads directory and lists it on the form;
/// saving the vendor stores the link.
fn attach_file(state: &mut State, ctx: &Ctx, source: &Path) {
    let display_name = source
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_default();
    let file_name = format!(
        "{}-{}-{display_name}",
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default(),
        state.form.attachments.len()
    );
    match std::fs::copy(source, ctx.upload_path(&file_name)) {
        Ok(size_bytes) => {
            state.form.attachments.push(VendorAttachment {
                file_name,
                display_name,
                size_bytes,
            });
            state.message = Some("Attachment added; save the vendor to keep it.".into());
        }
        Err(err) => state.error = Some(format!("could not attach {}: {err}", source.display())),
    }
}

/// Writes the filtered (or, when any are selected, the selected) vendors
/// to `vendors_export.csv`.
fn export_csv(state: &mut State, ctx: &Ctx) {
    state.message = None;
    state.error = None;
    let query = state.search.trim().to_lowercase();
    let selected: Vec<&Vendor> = if state.selected.is_empty() {
        state
            .vendors
            .iter()
            .filter(|v| matches_filters(v, &query, state.status_filter, &state.vendor_type_filter))
            .collect()
    } else {
        state
            .vendors
            .iter()
            .filter(|v| state.selected.contains(&v.id.unwrap_or_default()))
            .collect()
    };
    let rows: Vec<Vec<String>> = selected.iter().map(|v| export_row(v)).collect();
    let path = ctx.export_path("vendors_export.csv");
    match std::fs::write(&path, csv::export_csv(&export_headers(), &rows)) {
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

fn column_visible(state: &State, column: usize) -> bool {
    if column >= VENDOR_HEADERS.len() {
        return false;
    }
    if column == COL_SELECT || VENDOR_ALWAYS_SELECTED.contains(&column) {
        return true;
    }
    state
        .column_visible
        .get(column)
        .copied()
        .unwrap_or_else(|| VENDOR_VISIBLE_COLUMNS.contains(&column))
}

/// The data columns (after the serial number and select box) shown now.
fn visible_columns(state: &State) -> Vec<usize> {
    (FIRST_DATA_COLUMN..VENDOR_HEADERS.len())
        .filter(|column| column_visible(state, *column))
        .collect()
}

fn optional_column_visibility(state: &State) -> Vec<bool> {
    VENDOR_OPTIONAL_COLUMNS
        .iter()
        .map(|column| column_visible(state, *column))
        .collect()
}

fn set_optional_columns(state: &mut State, visible: bool) {
    state.column_visible = default_column_visibility();
    for column in VENDOR_OPTIONAL_COLUMNS {
        state.column_visible[column] = visible;
    }
}

fn matches_filters(vendor: &Vendor, query: &str, status: StatusFilter, vendor_type: &str) -> bool {
    let contains = |value: &Option<String>| {
        value
            .as_deref()
            .unwrap_or_default()
            .to_lowercase()
            .contains(query)
    };
    status.matches(vendor.is_active)
        && (vendor_type.is_empty() || vendor_type == "All" || vendor.vendor_type == vendor_type)
        && (query.is_empty()
            || vendor.name.to_lowercase().contains(query)
            || contains(&vendor.company_name)
            || contains(&vendor.email)
            || contains(&vendor.website)
            || contains(&vendor.mailing_address)
            || contains(&vendor.shipping_address)
            || contains(&vendor.description))
}

const TITLE: &str = "Vendors";

fn column_width(column: usize) -> f32 {
    match column {
        COL_VENDOR_ID => 140.0,
        COL_VENDOR_NAME => 220.0,
        COL_COMPANY_NAME => 180.0,
        COL_PHONE => 120.0,
        COL_EMAIL => 220.0,
        COL_WEBSITE => 180.0,
        COL_MAILING_ADDRESS | COL_SHIPPING_ADDRESS => 200.0,
        COL_PAYMENT_TERMS => 120.0,
        COL_DEBIT | COL_CREDIT => 100.0,
        COL_BALANCE => 110.0,
        COL_PAYMENT_METHOD | COL_ATTACHMENTS => 140.0,
        COL_DESCRIPTION => 200.0,
        COL_STATUS => 100.0,
        COL_CREATE_TIMESTAMP | COL_MODIFIED_TIMESTAMP => 180.0,
        COL_CUSTOM_FIELDS => 160.0,
        _ => 68.0,
    }
}

fn vendor_preview(state: &State, theme: Theme) -> Element<'static, Message> {
    let form = &state.form;
    let header = |column: usize| VENDOR_HEADERS[column];
    let field = |column: usize, value: String| forms::PreviewField::new(header(column), value);
    let saved = state
        .editing_id
        .and_then(|id| state.vendors.iter().find(|vendor| vendor.id == Some(id)));
    let name = if form.name.is_empty() {
        "New Vendor".into()
    } else {
        form.name.clone()
    };
    let vendor_id = saved
        .and_then(|vendor| vendor.reference_code.clone())
        .unwrap_or_else(|| "Assigned on save".into());
    let status = if form.status.is_empty() {
        "Active".into()
    } else {
        form.status.clone()
    };
    let identity = vec![
        field(COL_VENDOR_ID, vendor_id.clone()),
        field(COL_PHONE, dash(&form.phone)),
        field(COL_EMAIL, dash(&form.email)),
    ];
    let basic_information = forms::PreviewSection::new(
        "Basic Information",
        vec![
            field(COL_SERIAL, "Auto-generated".into()),
            field(COL_VENDOR_ID, vendor_id),
            field(COL_VENDOR_NAME, dash(&form.name)),
            field(COL_COMPANY_NAME, dash(&form.company_name)),
            forms::PreviewField::new(VENDOR_TYPE_LABEL, dash(&form.vendor_type)),
        ],
    );
    let contact_information = forms::PreviewSection::new(
        "Contact Information",
        vec![
            field(COL_PHONE, dash(&form.phone)),
            field(COL_EMAIL, dash(&form.email)),
            field(COL_WEBSITE, dash(&form.website)),
        ],
    );
    let address_information = forms::PreviewSection::new(
        "Address Information",
        vec![
            field(COL_MAILING_ADDRESS, dash(&form.mailing_address)),
            field(COL_SHIPPING_ADDRESS, dash(&form.shipping_address)),
        ],
    );
    let additional_information = forms::PreviewSection::new(
        "Additional Information",
        vec![
            field(COL_PAYMENT_TERMS, format!("{} days", form.payment_terms)),
            field(COL_PAYMENT_METHOD, dash(&form.payment_method)),
            forms::PreviewField::new("Tax ID", dash(&form.tax_id)),
            field(COL_DESCRIPTION, dash(&form.description_text())),
            field(COL_ATTACHMENTS, dash(&attachment_names(&form.attachments))),
        ],
    );
    let status_audit = forms::PreviewSection::new(
        "Status & Audit",
        vec![
            field(COL_STATUS, status.clone()),
            field(COL_CREATE_TIMESTAMP, "--".into()),
            field(COL_MODIFIED_TIMESTAMP, "--".into()),
        ],
    );
    let custom_fields = if form.custom_fields.is_empty() {
        vec![field(COL_CUSTOM_FIELDS, "--".into())]
    } else {
        form.custom_fields
            .iter()
            .map(|custom| forms::PreviewField::new(dash(&custom.name), custom.value.clone()))
            .collect()
    };
    let customization = forms::PreviewSection::new("Customization", custom_fields);
    forms::record_preview(
        theme,
        forms::RecordPreview {
            name,
            subtitle: format!("Vendor · {}", dash(&form.vendor_type)),
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
        Some(financial_summary(state, theme)),
        Some(accounting_sections(state, theme)),
        Message::Preview,
        Message::Preview,
    )
}

const PREVIEW_TABS: [&str; 3] = ["Transactions", "Vendor Ledger", "Trial Balance"];

/// Loads the read-only AP ledger for the vendor being previewed. Nothing
/// is posted; an unsaved vendor simply has no ledger.
fn refresh_ledger(state: &mut State, ctx: &Ctx) {
    state.ledger = None;
    state.ledger_error = None;
    state.preview_tab = 0;
    let Some(vendor_id) = state.editing_id else {
        return;
    };
    match PurchaseService::new(ctx.db.clone()).vendor_ledger(ctx.company_id, vendor_id) {
        Ok(ledger) => state.ledger = Some(ledger),
        Err(err) => state.ledger_error = Some(err.to_string()),
    }
}

/// Payable / receivable / settled, from the sign of the outstanding balance.
fn balance_label(outstanding: Decimal) -> (&'static str, BadgeTone) {
    if outstanding > Decimal::ZERO {
        ("Payable", BadgeTone::Warning)
    } else if outstanding < Decimal::ZERO {
        ("Receivable", BadgeTone::Positive)
    } else {
        ("Settled", BadgeTone::Neutral)
    }
}

fn muted(theme: Theme, message: impl text::IntoFragment<'static>) -> Element<'static, Message> {
    labels::muted(theme, message, typography::body()).into()
}

/// Totals at the top of the preview, all derived from the vendor's ledger.
fn financial_summary(state: &State, theme: Theme) -> Element<'static, Message> {
    let Some(ledger) = &state.ledger else {
        let message = state
            .ledger_error
            .clone()
            .unwrap_or_else(|| "Financial summary appears once the vendor is saved.".into());
        return panels::titled_card(theme, "Financial Summary", muted(theme, message));
    };
    let currency = ledger.currency.as_str();
    let with_currency = |amount: Decimal| format!("{currency} {}", money(amount));
    let outstanding = ledger.outstanding();
    let (label, tone) = balance_label(outstanding);
    let last = ledger
        .last_transaction_date()
        .map(day)
        .unwrap_or_else(|| "No transactions".into());
    let stats = row![
        labels::stat(theme, "Total Debit", with_currency(ledger.total_debit())),
        labels::stat(theme, "Total Credit", with_currency(ledger.total_credit())),
        // Vendors carry no balance of their own at the moment: the ledger
        // starts from zero and every movement is a posted transaction.
        labels::stat(theme, "Opening Balance", with_currency(Decimal::ZERO)),
        labels::stat(theme, "Last Transaction", last),
    ]
    .spacing(spacing::MD);
    let outstanding_line = row![
        labels::muted(theme, "Outstanding Balance", typography::caption()),
        labels::strong(
            theme,
            with_currency(outstanding.abs()),
            typography::subtitle()
        ),
        badges::status_pill(theme, label, tone),
    ]
    .spacing(spacing::SPACE_10)
    .align_y(Alignment::Center);
    panels::titled_card(
        theme,
        "Financial Summary",
        column![outstanding_line, stats].spacing(spacing::MD),
    )
}

/// Tabbed transaction history / ledger / trial balance plus the open
/// balances panel, shown below the vendor's profile sections.
fn accounting_sections(state: &State, theme: Theme) -> Element<'static, Message> {
    let Some(ledger) = &state.ledger else {
        return column![].into();
    };
    let tab_body = match state.preview_tab {
        0 => transaction_history(state, ledger, theme),
        1 => vendor_ledger_tab(ledger, theme),
        _ => vendor_trial_balance(ledger, theme),
    };
    let tabs = forms::tab_bar(
        theme,
        &PREVIEW_TABS,
        state.preview_tab,
        Message::PreviewTabChanged,
    );
    column![
        panels::titled_card(
            theme,
            "Accounting",
            column![tabs, tab_body].spacing(spacing::MD)
        ),
        outstanding_balances(ledger, theme),
    ]
    .spacing(spacing::MD)
    .into()
}

fn transaction_history(
    state: &State,
    ledger: &VendorLedger,
    theme: Theme,
) -> Element<'static, Message> {
    if ledger.entries.is_empty() {
        return muted(theme, "No transactions recorded for this vendor.");
    }
    let mut entries: Vec<_> = ledger.entries.iter().collect();
    if state.ledger_newest_first {
        entries.reverse();
    }
    let rows = entries
        .iter()
        .map(|e| {
            vec![
                day(e.date),
                format!("{} (#{})", e.reference, e.transaction_id),
                e.kind.clone(),
                e.description.clone(),
                side(e.debit),
                side(e.credit),
                money(e.running_balance),
            ]
        })
        .collect();
    let order = if state.ledger_newest_first {
        "Date: newest first"
    } else {
        "Date: oldest first"
    };
    column![
        button(text(order).size(typography::label()))
            .padding([spacing::SPACE_4, spacing::SM])
            .style(button_style::outlined(theme, false))
            .on_press(Message::ToggleLedgerOrder),
        table_style::grid(
            theme,
            &vendor_ledger::HISTORY_COLUMNS,
            &[2, 3, 2, 4, 2, 2, 2],
            &[4, 5, 6],
            rows,
            None,
        ),
    ]
    .spacing(spacing::SM)
    .into()
}

/// T-account layout: debits on the left, credits on the right, with the
/// opening and closing balance and each entry traceable to its source.
fn vendor_ledger_tab(ledger: &VendorLedger, theme: Theme) -> Element<'static, Message> {
    if ledger.entries.is_empty() {
        return muted(theme, "No ledger entries for this vendor.");
    }
    let t_side = |title: &str, debit_side: bool| {
        let rows: Vec<Vec<String>> = ledger
            .entries
            .iter()
            .filter(|e| {
                if debit_side {
                    !e.debit.is_zero()
                } else {
                    !e.credit.is_zero()
                }
            })
            .map(|e| {
                vec![
                    day(e.date),
                    format!("{} (#{})", e.reference, e.transaction_id),
                    money(if debit_side { e.debit } else { e.credit }),
                ]
            })
            .collect();
        let total = if debit_side {
            ledger.total_debit()
        } else {
            ledger.total_credit()
        };
        column![
            labels::strong(theme, title.to_string(), typography::label()),
            table_style::grid(
                theme,
                &vendor_ledger::T_ACCOUNT_COLUMNS,
                &[2, 3, 2],
                &[2],
                rows,
                Some(vec!["Total".into(), String::new(), money(total)]),
            ),
        ]
        .spacing(spacing::SPACE_6)
        .width(Length::FillPortion(1))
    };
    let closing = ledger.outstanding();
    let (label, tone) = balance_label(closing);
    let summary = row![
        labels::muted(
            theme,
            format!("Opening balance: {}", money(Decimal::ZERO)),
            typography::body()
        ),
        Space::with_width(Length::Fill),
        labels::strong(
            theme,
            format!(
                "Closing balance: {} {}",
                ledger.currency,
                money(closing.abs())
            ),
            typography::body()
        ),
        badges::status_pill(theme, label, tone),
    ]
    .spacing(spacing::SM)
    .align_y(Alignment::Center);
    column![
        row![t_side("Debit (Dr)", true), t_side("Credit (Cr)", false)].spacing(spacing::LG),
        summary,
    ]
    .spacing(spacing::MD)
    .into()
}

/// Only the accounts this vendor's transactions touched, with their
/// debit/credit totals for the vendor's reporting period.
fn vendor_trial_balance(ledger: &VendorLedger, theme: Theme) -> Element<'static, Message> {
    let (Some(from), Some(to)) = (
        ledger.first_transaction_date(),
        ledger.last_transaction_date(),
    ) else {
        return muted(
            theme,
            "No accounts affected: this vendor has no transactions.",
        );
    };
    let rows = ledger
        .accounts
        .iter()
        .map(|a| {
            let net = a.net();
            vec![
                a.code.clone(),
                a.name.clone(),
                if net > Decimal::ZERO {
                    money(net)
                } else {
                    String::new()
                },
                if net < Decimal::ZERO {
                    money(-net)
                } else {
                    String::new()
                },
            ]
        })
        .collect();
    let debit: Decimal = ledger
        .accounts
        .iter()
        .map(|a| a.net().max(Decimal::ZERO))
        .sum();
    let credit: Decimal = ledger
        .accounts
        .iter()
        .map(|a| (-a.net()).max(Decimal::ZERO))
        .sum();
    column![
        muted(theme, format!("Period: {} to {}", day(from), day(to))),
        table_style::grid(
            theme,
            &vendor_ledger::TRIAL_BALANCE_COLUMNS,
            &[2, 6, 2, 2],
            &[2, 3],
            rows,
            Some(vec!["".into(), "Total".into(), money(debit), money(credit)]),
        ),
    ]
    .spacing(spacing::SM)
    .into()
}

/// Open bills from the payables subledger with overdue/aging information.
/// It breaks down the summary's balance rather than adding to it.
fn outstanding_balances(ledger: &VendorLedger, theme: Theme) -> Element<'static, Message> {
    if ledger.open_payables.is_empty() {
        return panels::titled_card(
            theme,
            "Outstanding Balances",
            muted(theme, "No open bills."),
        );
    }
    let today = chrono::Local::now().date_naive();
    let rows = ledger
        .open_payables
        .iter()
        .map(|p| {
            vec![
                p.reference.clone().unwrap_or_default(),
                day(p.due_date),
                if p.is_overdue(today) {
                    format!("Overdue {}d", (today - p.due_date).num_days())
                } else {
                    "Not due".into()
                },
                money(p.amount),
                money(p.balance()),
            ]
        })
        .collect();
    let aging = ledger.aging(today);
    let overdue = aging.days_1_to_30 + aging.days_31_to_60 + aging.days_61_to_90 + aging.over_90;
    let total = aging.total();
    let overdue_line = if overdue.is_zero() {
        badges::status_pill(theme, "Nothing overdue", BadgeTone::Positive)
    } else {
        badges::status_pill(
            theme,
            format!("Overdue {} {}", ledger.currency, money(overdue)),
            BadgeTone::Negative,
        )
    };
    panels::titled_card(
        theme,
        "Outstanding Balances",
        column![
            overdue_line,
            table_style::grid(
                theme,
                &vendor_ledger::OPEN_BILL_COLUMNS,
                &[3, 2, 2, 2, 2],
                &[3, 4],
                rows,
                Some(vec![
                    "Total".into(),
                    String::new(),
                    String::new(),
                    String::new(),
                    format!("{} {}", ledger.currency, money(total)),
                ]),
            ),
        ]
        .spacing(spacing::SM),
    )
}

/// The header label of `column`: sorts by it when pressed.
fn column_label(
    sort: Option<(usize, bool)>,
    theme: Theme,
    column: usize,
    width: Length,
) -> Element<'static, Message> {
    table_style::sort_header(
        theme,
        VENDOR_HEADERS[column],
        sort_of(sort, column),
        width,
        Message::SortBy(column),
    )
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

    fn cell(&self, column: usize) -> Length {
        let extra = if column == COL_VENDOR_NAME {
            self.name_extra
        } else {
            0.0
        };
        Length::Fixed(column_width(column) + extra)
    }
}

fn table_layout(state: &State, available: f32) -> TableLayout {
    let columns = visible_columns(state);
    let field_width: f32 = columns.iter().map(|column| column_width(*column)).sum();
    let field_count = columns.len() as f32;
    let row_width = column_width(COL_SERIAL)
        + spacing::SELECT_COLUMN_WIDTH
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
    if let Some((column, ascending)) = state.sort {
        // S.No is the id order, so only its direction matters.
        if column == COL_SERIAL {
            if !ascending {
                filtered.reverse();
            }
        } else {
            filtered.sort_by(|a, b| {
                let order = compare_by_column(a, b, column);
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

/// The light header row above the rows: tri-state box and column labels,
/// padded exactly like a row so every label sits over its cells.
fn table_header(
    state: &State,
    theme: Theme,
    check: CheckState,
    on_toggle: Message,
    layout: &TableLayout,
) -> Element<'static, Message> {
    let mut header = row![
        container(column_label(state.sort, theme, COL_SERIAL, Length::Shrink))
            .width(layout.cell(COL_SERIAL))
            .align_x(iced::alignment::Horizontal::Center),
        container(checkbox_style::select_box(theme, check, on_toggle))
            .width(Length::Fixed(spacing::SELECT_COLUMN_WIDTH)),
    ]
    .spacing(spacing::MD)
    .align_y(Alignment::Center);
    for column in visible_columns(state) {
        header = header.push(column_label(state.sort, theme, column, layout.cell(column)));
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

/// One table row: select box, the visible cells, status pill and the
/// edit button; the whole row opens the detail panel.
fn vendor_row(
    theme: Theme,
    vendor: &Vendor,
    serial_number: usize,
    layout: &TableLayout,
    selected: bool,
    highlighted: bool,
    columns: &[usize],
) -> Element<'static, Message> {
    let id = vendor.id.unwrap_or_default();
    let mut cells = row![
        container(text(serial_number.to_string()).size(typography::body()))
            .width(layout.cell(COL_SERIAL))
            .align_x(iced::alignment::Horizontal::Center),
        container(checkbox_style::select_box(
            theme,
            if selected {
                CheckState::Checked
            } else {
                CheckState::Unchecked
            },
            Message::RowSelected(id, !selected),
        ))
        .width(Length::Fixed(spacing::SELECT_COLUMN_WIDTH)),
    ]
    .spacing(spacing::MD)
    .height(Length::Fixed(spacing::table_row_height()))
    .align_y(Alignment::Center);
    for &column in columns {
        let cell: Element<'static, Message> = match column {
            COL_VENDOR_NAME => labels::strong(theme, vendor.name.clone(), typography::label())
                .wrapping(Wrapping::None)
                .into(),
            COL_STATUS => {
                let (label, tone) = if vendor.is_active {
                    ("Active", BadgeTone::Positive)
                } else {
                    ("Archived", BadgeTone::Neutral)
                };
                badges::status_pill(theme, label, tone)
            }
            _ => labels::muted(
                theme,
                vendor_column_value(vendor, column),
                typography::body(),
            )
            .wrapping(Wrapping::None)
            .into(),
        };
        cells = cells.push(container(cell).width(layout.cell(column)).clip(true));
    }
    cells = cells.push(
        container(button_style::icon_tool(
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
        dropdown::counted_filter(theme, "Vendor type", vendor_type_options),
        dropdown::counted_filter(theme, VENDOR_HEADERS[COL_STATUS], status_options),
    ]
    .spacing(spacing::SPACE_12)
    .align_y(Alignment::Center)
    .into()
}

/// Reset, Columns and Export icon buttons.
fn tool_buttons(theme: Theme) -> Element<'static, Message> {
    row![
        button_style::icon_tool(theme, icons::RESET, "Reset all", Message::ResetAll, true),
        button_style::icon_tool(
            theme,
            icons::COLUMNS,
            "Columns",
            Message::ToggleColumns,
            true
        ),
        button_style::icon_tool(
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

/// A labelled detail-form dropdown over `options`.
fn select_field<'a>(
    theme: Theme,
    label: &'a str,
    options: &'static [&'static str],
    selected: &'a str,
    on_select: fn(String) -> Message,
) -> Element<'a, Message> {
    forms::field_row(
        theme,
        label,
        dropdown::form_select(
            theme,
            options,
            (!selected.is_empty()).then_some(selected),
            move |value: &str| on_select(value.to_string()),
        )
        .placeholder(label)
        .into(),
    )
}

/// A labelled detail-form text input bound to `value`.
fn input_field<'a>(
    theme: Theme,
    label: &'a str,
    value: &'a str,
    on_input: fn(String) -> Message,
) -> Element<'a, Message> {
    forms::text_field(theme, label, text_input(label, value).on_input(on_input))
}

/// The fields of the active detail tab, in reading order.
fn detail_fields(state: &State, theme: Theme) -> Vec<Element<'_, Message>> {
    let form = &state.form;
    let header = |column: usize| VENDOR_HEADERS[column];
    match form.tab {
        0 => vec![
            forms::disabled_field(theme, header(COL_VENDOR_ID), &form.reference_code),
            input_field(theme, "Vendor name", &form.name, Message::NameChanged),
            input_field(
                theme,
                header(COL_COMPANY_NAME),
                &form.company_name,
                Message::CompanyNameChanged,
            ),
            select_field(
                theme,
                "Vendor type",
                &VENDOR_TYPES,
                &form.vendor_type,
                Message::VendorTypeChanged,
            ),
        ],
        1 => vec![
            input_field(
                theme,
                "Phone (optional)",
                &form.phone,
                Message::PhoneChanged,
            ),
            input_field(
                theme,
                "Email (optional)",
                &form.email,
                Message::EmailChanged,
            ),
            input_field(
                theme,
                header(COL_WEBSITE),
                &form.website,
                Message::WebsiteChanged,
            ),
        ],
        2 => vec![
            input_field(
                theme,
                header(COL_MAILING_ADDRESS),
                &form.mailing_address,
                Message::MailingAddressChanged,
            ),
            input_field(
                theme,
                header(COL_SHIPPING_ADDRESS),
                &form.shipping_address,
                Message::ShippingAddressChanged,
            ),
        ],
        3 => vec![
            forms::text_field(
                theme,
                header(COL_PAYMENT_TERMS),
                text_input("30", &form.payment_terms).on_input(Message::PaymentTermsChanged),
            ),
            select_field(
                theme,
                header(COL_PAYMENT_METHOD),
                &VENDOR_PAYMENT_METHODS,
                &form.payment_method,
                Message::PaymentMethodChanged,
            ),
            input_field(
                theme,
                "Tax ID (optional)",
                &form.tax_id,
                Message::TaxIdChanged,
            ),
            forms::text_area_field(
                theme,
                header(COL_DESCRIPTION),
                &form.description,
                Message::DescriptionEdited,
            ),
            attachments_block(form, theme),
        ],
        4 => vec![
            select_field(
                theme,
                header(COL_STATUS),
                &VENDOR_STATUS_OPTIONS,
                &form.status,
                Message::StatusChanged,
            ),
            forms::disabled_field(theme, header(COL_CREATE_TIMESTAMP), "--"),
            forms::disabled_field(theme, header(COL_MODIFIED_TIMESTAMP), "--"),
        ],
        _ => vec![forms::custom_fields(forms::CustomFields {
            theme,
            title: header(COL_CUSTOM_FIELDS),
            fields: form
                .custom_fields
                .iter()
                .map(|field| (field.name.as_str(), field.value.as_str()))
                .collect(),
            on_name: Message::CustomFieldNameChanged,
            on_value: Message::CustomFieldValueChanged,
            on_remove: Message::CustomFieldRemoved,
            on_add: Message::CustomFieldAdded,
        })],
    }
}

/// The form's attached files with their Add, Rename and Remove buttons.
fn attachments_block(form: &VendorForm, theme: Theme) -> Element<'_, Message> {
    forms::attachments(forms::Attachments {
        theme,
        title: VENDOR_HEADERS[COL_ATTACHMENTS],
        files: form
            .attachments
            .iter()
            .map(|attachment| forms::AttachmentRow {
                name: &attachment.display_name,
                size_bytes: attachment.size_bytes,
            })
            .collect(),
        renaming: form
            .renaming
            .as_ref()
            .map(|(index, draft)| (*index, draft.as_str())),
        on_add: Message::AddAttachment,
        on_edit: Message::AttachmentRenameStarted,
        on_rename: Message::AttachmentRenameChanged,
        on_rename_done: Message::AttachmentRenameDone,
        on_remove: Message::AttachmentRemoved,
    })
}

/// The Vendor Details card: two field columns, Save/Close.
fn detail_form(state: &State, theme: Theme) -> Element<'_, Message> {
    let footer = row![
        button_style::bold("Save", [spacing::SM, spacing::MD])
            .style(button_style::mode(theme, true))
            .on_press(Message::Create),
        button_style::bold("Close", [spacing::SM, spacing::MD])
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
        active_tab: state.form.tab,
        on_select_tab: Message::FormTabChanged,
        summary: Some(summary.into()),
        content: forms::field_grid(detail_fields(state, theme), 1),
        footer,
        on_preview: Message::Preview,
    })
}

/// Name, contact line and status pill at the top of the detail panel.
fn vendor_summary(state: &State, theme: Theme) -> Element<'static, Message> {
    let form = &state.form;
    let name = if form.name.is_empty() {
        "New Vendor".to_string()
    } else {
        form.name.clone()
    };
    let (status, tone) = if form.status == "Inactive" {
        ("Archived", BadgeTone::Neutral)
    } else {
        ("Active", BadgeTone::Positive)
    };
    forms::summary_card(
        theme,
        name,
        format!("{}  ·  {}", dash(&form.phone), dash(&form.email)),
        badges::status_pill(theme, status, tone),
    )
}

/// Live, read-only echo of Mailing Address, Payment Method and Description
/// shown above the Basic Information tab -- it mirrors whatever is typed
/// into those fields further down the form, in both the "New Vendor" and
/// editing states.
fn vendor_live_preview(state: &State, theme: Theme) -> Element<'static, Message> {
    let form = &state.form;
    let card = |glyph: &'static str, column: usize, value: String| {
        forms::field_box(
            theme,
            glyph,
            VENDOR_HEADERS[column],
            text(dash(&value))
                .size(typography::body())
                .color(theme.neutral_font())
                .into(),
        )
    };
    forms::field_grid(
        vec![
            card(
                icons::FIELD_ADDRESS,
                COL_MAILING_ADDRESS,
                form.mailing_address.clone(),
            ),
            card(
                icons::FIELD_PAYMENT,
                COL_PAYMENT_METHOD,
                form.payment_method.clone(),
            ),
            card(icons::FIELD_NOTE, COL_DESCRIPTION, form.description_text()),
        ],
        2,
    )
}

/// The column picker in its own card.
fn columns_card(state: &State, theme: Theme) -> Element<'_, Message> {
    let optional_visibility = optional_column_visibility(state);
    panels::side(
        theme,
        data_table::columns_panel(
            theme,
            &VENDOR_OPTIONAL_LABELS,
            &optional_visibility,
            Message::ColumnToggled,
            Message::SelectAllColumns,
            Message::DeselectAllColumns,
            Message::ResetColumns,
        ),
    )
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
            button_style::icon_label(
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
        button_style::icon_label(icons::ADD, "New Add", [spacing::SPACE_10, spacing::MD])
            .style(button_style::mode(theme, true))
            .on_press(Message::OpenForm)
            .into(),
    );
    let header = toolbar::card_header(
        class,
        title,
        lift(search::input(
            theme,
            "Search vendors by name or email",
            &state.search,
            Message::SearchChanged,
        )),
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

    let mut body = column![lift(table)].height(Length::Fill);
    if let Some(message) = &state.message {
        body = body.push(lift(
            container(text(message.clone()).color(theme.positive_front()))
                .padding([spacing::SPACE_6, spacing::SPACE_20])
                .into(),
        ));
    }
    if let Some(error) = &state.error {
        body = body.push(lift(
            container(text(error.clone()).color(theme.negative_front()))
                .padding([spacing::SPACE_6, spacing::SPACE_20])
                .into(),
        ));
    }
    let footer = lift(pagination::footer(pagination::Footer {
        theme,
        class,
        page: vendors.page,
        pages: vendors.pages,
        shown: vendors.rows.len(),
        total: vendors.total,
        page_size: vendors.size,
        page_sizes: &PAGE_SIZES,
        on_previous: Message::PrevPage,
        on_page: Message::GoToPage,
        on_page_size: Message::PageSizeChanged,
    }));
    panels::page_card(theme, header, body.into(), Some(footer))
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
    let columns = visible_columns(state);
    let header_row = table_header(state, theme, check, toggle_all, layout);
    let mut rows = column![];
    for (index, vendor) in vendors.rows.iter().enumerate() {
        if index > 0 {
            rows = rows.push(table_style::divider_line(theme));
        }
        let id = vendor.id.unwrap_or_default();
        rows = rows.push(vendor_row(
            theme,
            vendor,
            vendors.page * vendors.size + index + 1,
            layout,
            state.selected.contains(&id),
            state.selected.contains(&id) || (state.form_open && state.editing_id == vendor.id),
            &columns,
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
    use crate::backend::accounting::vendors::{
        ExportColumn, VENDOR_DEFAULT_VISIBLE, VENDOR_EXPORT_COLUMNS,
    };
    use crate::backend::organization::companies::Company;
    use crate::backend::repositories::organization_repository::{self, OrganizationRepository};
    use crate::backend::repositories::party_repository;
    use crate::database::database::Database;
    use chrono::NaiveDate;
    use std::path::PathBuf;

    #[test]
    fn clicking_a_header_sorts_ascending_then_descending_then_resets() {
        let mut state = State {
            vendors: ["b", "C", "a"]
                .iter()
                .map(|name| Vendor::new(1, *name))
                .collect(),
            ..State::default()
        };
        let names = |state: &State| -> Vec<String> {
            vendor_page(state)
                .rows
                .iter()
                .map(|v| v.name.clone())
                .collect()
        };
        let ctx = ctx_with_company();
        update(&mut state, Message::SortBy(COL_VENDOR_NAME), &ctx);
        assert_eq!(names(&state), ["a", "b", "C"]);
        update(&mut state, Message::SortBy(COL_VENDOR_NAME), &ctx);
        assert_eq!(names(&state), ["C", "b", "a"]);
        update(&mut state, Message::SortBy(COL_VENDOR_NAME), &ctx);
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
            pagination::results_label(page.rows.len(), page.total),
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
            pagination::results_label(page.rows.len(), page.total),
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
            pagination::results_label(page.rows.len(), page.total),
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
        assert_eq!(roomy.cell(COL_VENDOR_ID), tight.cell(COL_VENDOR_ID));
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
                            state.form.tab = tab;
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
                toolbar::card_header(class, item(), item(), item(), item(), item());
        }
    }

    #[test]
    fn the_field_grid_pads_a_short_last_line() {
        let fields = (0..5)
            .map(|index| forms::readonly_field(Theme::Light, "Label", index.to_string()))
            .collect();
        let _: Element<'_, Message> = forms::field_grid(fields, 2);
        let _: Element<'_, Message> = forms::field_grid(Vec::new(), 2);
    }

    #[test]
    fn the_preview_builds_with_no_ledger_an_empty_ledger_and_a_full_one() {
        use crate::backend::accounting::accounts_payable::payables::Payable;
        use crate::backend::purchases::vendor_ledger::{
            with_running_balances, VendorAccountTotal, VendorLedgerEntry,
        };
        use rust_decimal_macros::dec;
        let date = NaiveDate::from_ymd_opt(2026, 1, 10).unwrap();
        let entry = |id, debit, credit| VendorLedgerEntry {
            transaction_id: id,
            date,
            kind: "Bill".into(),
            reference: format!("BILL-{id}"),
            description: "Supplies".into(),
            debit,
            credit,
            running_balance: dec!(0),
        };
        let full = VendorLedger {
            currency: "PKR".into(),
            entries: with_running_balances(vec![
                entry(1, dec!(0), dec!(300)),
                entry(2, dec!(50), dec!(0)),
            ]),
            accounts: vec![VendorAccountTotal {
                code: "2000".into(),
                name: "Accounts Payable".into(),
                debit: dec!(50),
                credit: dec!(300),
            }],
            open_payables: vec![Payable::new(1, 1, date, date, dec!(250))],
        };
        let empty = VendorLedger {
            currency: "PKR".into(),
            ..Default::default()
        };
        for theme in THEMES {
            for ledger in [None, Some(empty.clone()), Some(full.clone())] {
                for tab in 0..PREVIEW_TABS.len() {
                    for newest_first in [false, true] {
                        let mut state = state_with(vendors(1, false));
                        state.show_preview = true;
                        state.ledger = ledger.clone();
                        state.preview_tab = tab;
                        state.ledger_newest_first = newest_first;
                        let _: Element<'_, Message> = vendor_preview(&state, theme);
                    }
                }
            }
        }
    }

    #[test]
    fn balance_badge_follows_the_ap_sign_convention() {
        use rust_decimal_macros::dec;
        assert_eq!(balance_label(dec!(1000)).0, "Payable");
        assert_eq!(balance_label(dec!(600)).0, "Payable");
        assert_eq!(balance_label(dec!(0)).0, "Settled");
        assert_eq!(balance_label(dec!(-5)).0, "Receivable");
    }

    // -- columns by index ------------------------------------------------

    fn names(columns: &[usize]) -> Vec<&'static str> {
        columns
            .iter()
            .map(|column| VENDOR_HEADERS[*column])
            .collect()
    }

    #[test]
    fn the_column_constants_resolve_to_the_same_names_as_before() {
        let always = [
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
        assert_eq!(names(&VENDOR_ALWAYS_SELECTED), always);
        assert_eq!(names(&VENDOR_DEFAULT_VISIBLE), always);
        assert_eq!(
            names(&VENDOR_VISIBLE_COLUMNS),
            [
                "S.No",
                "",
                "Vendor ID",
                "Vendor Name",
                "Company Name",
                "Phone",
                "Email",
                "Mailing Address",
                "Debit",
                "Credit",
                "Balance"
            ]
        );
        assert_eq!(
            VENDOR_OPTIONAL_LABELS,
            [
                "Website",
                "Shipping Address",
                "Payment Terms",
                "Payment Method",
                "Attachments",
                "Description",
                "Status",
                "Create Timestamp",
                "Modified Timestamp",
                "Custom Fields"
            ]
        );
        // The default table shows the same columns, in the same order.
        let state = state_with(Vec::new());
        assert_eq!(
            names(&visible_columns(&state)),
            [
                "Vendor ID",
                "Vendor Name",
                "Company Name",
                "Phone",
                "Email",
                "Mailing Address",
                "Debit",
                "Credit",
                "Balance"
            ]
        );
    }

    #[test]
    fn columns_csv_headers_and_sort_keys_are_built_from_the_header_indexes() {
        assert_eq!(
            VENDOR_OPTIONAL_LABELS.to_vec(),
            names(&VENDOR_OPTIONAL_COLUMNS)
        );
        for (header, column) in export_headers().iter().zip(VENDOR_EXPORT_COLUMNS) {
            match column {
                ExportColumn::Table(column) => assert_eq!(*header, VENDOR_HEADERS[column]),
                ExportColumn::VendorType => assert_eq!(*header, VENDOR_TYPE_LABEL),
            }
        }
        assert_eq!(
            export_headers()[..7],
            [
                "Vendor Name",
                "Company Name",
                "Email",
                "Phone",
                "Vendor Type",
                "Payment Terms",
                "Status"
            ]
        );

        let ctx = ctx_with_company();
        let mut state = state_with(vendors(3, false));
        for column in FIRST_DATA_COLUMN..VENDOR_HEADERS.len() {
            update(&mut state, Message::SortBy(column), &ctx);
            assert_eq!(state.sort, Some((column, true)));
            update(&mut state, Message::SortBy(column), &ctx);
            update(&mut state, Message::SortBy(column), &ctx);
            assert_eq!(state.sort, None);
        }

        let all_optional: Vec<usize> = {
            update(&mut state, Message::SelectAllColumns, &ctx);
            visible_columns(&state)
        };
        assert_eq!(
            names(&all_optional),
            VENDOR_HEADERS[FIRST_DATA_COLUMN..].to_vec()
        );
    }

    #[test]
    fn toggling_an_optional_column_shows_it_and_reset_hides_it_again() {
        let ctx = ctx_with_company();
        let mut state = state_with(vendors(1, false));
        let website = VENDOR_OPTIONAL_COLUMNS
            .iter()
            .position(|column| *column == COL_WEBSITE)
            .unwrap();
        update(&mut state, Message::ColumnToggled(website), &ctx);
        assert!(visible_columns(&state).contains(&COL_WEBSITE));
        update(&mut state, Message::ResetColumns, &ctx);
        assert!(!visible_columns(&state).contains(&COL_WEBSITE));
    }

    // -- profile fields, custom fields and attachments --------------------

    /// A context whose uploads land in a fresh temporary directory.
    fn ctx_with_uploads() -> (Ctx, PathBuf) {
        let mut ctx = ctx_with_company();
        let dir = std::env::temp_dir().join(format!(
            "memonbooks-vendor-uploads-{}-{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        ctx.uploads_dir = dir.join("uploads");
        (ctx, dir)
    }

    fn sample_file(dir: &Path, name: &str, bytes: &[u8]) -> PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, bytes).unwrap();
        path
    }

    #[test]
    fn attaching_a_file_copies_it_and_lists_its_name_and_size() {
        let (ctx, dir) = ctx_with_uploads();
        let mut state = load(&ctx);
        update(&mut state, Message::OpenForm, &ctx);
        let source = sample_file(&dir, "contract.pdf", b"hello world");

        attach_file(&mut state, &ctx, &source);

        assert!(state.error.is_none(), "{:?}", state.error);
        let attachment = &state.form.attachments[0];
        assert_eq!(attachment.display_name, "contract.pdf");
        assert_eq!(attachment.size_bytes, 11);
        let stored = ctx.upload_path(&attachment.file_name);
        assert_eq!(std::fs::read(stored).unwrap(), b"hello world");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn renaming_an_attachment_changes_only_its_display_name() {
        let (ctx, dir) = ctx_with_uploads();
        let mut state = load(&ctx);
        update(&mut state, Message::OpenForm, &ctx);
        attach_file(&mut state, &ctx, &sample_file(&dir, "scan.png", b"png"));
        let stored = state.form.attachments[0].file_name.clone();

        update(&mut state, Message::AttachmentRenameStarted(0), &ctx);
        assert_eq!(state.form.renaming, Some((0, "scan.png".to_string())));
        update(
            &mut state,
            Message::AttachmentRenameChanged("  Signed licence  ".into()),
            &ctx,
        );
        update(&mut state, Message::AttachmentRenameDone, &ctx);

        assert_eq!(state.form.renaming, None);
        assert_eq!(state.form.attachments[0].display_name, "Signed licence");
        assert_eq!(state.form.attachments[0].file_name, stored);

        // A blank name keeps the old one.
        update(&mut state, Message::AttachmentRenameStarted(0), &ctx);
        update(
            &mut state,
            Message::AttachmentRenameChanged("  ".into()),
            &ctx,
        );
        update(&mut state, Message::AttachmentRenameDone, &ctx);
        assert_eq!(state.form.attachments[0].display_name, "Signed licence");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn removing_an_attachment_drops_the_link_and_keeps_the_stored_copy() {
        let (ctx, dir) = ctx_with_uploads();
        let mut state = load(&ctx);
        update(&mut state, Message::OpenForm, &ctx);
        attach_file(&mut state, &ctx, &sample_file(&dir, "a.txt", b"a"));
        attach_file(&mut state, &ctx, &sample_file(&dir, "b.txt", b"b"));
        let removed = state.form.attachments[0].file_name.clone();

        update(&mut state, Message::AttachmentRenameStarted(1), &ctx);
        update(&mut state, Message::AttachmentRemoved(0), &ctx);
        update(&mut state, Message::AttachmentRemoved(5), &ctx);

        assert_eq!(state.form.renaming, None);
        assert_eq!(state.form.attachments.len(), 1);
        assert_eq!(state.form.attachments[0].display_name, "b.txt");
        assert!(ctx.upload_path(&removed).exists());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn every_profile_field_custom_field_and_attachment_is_saved_and_reloaded() {
        let (ctx, dir) = ctx_with_uploads();
        let mut state = load(&ctx);
        update(&mut state, Message::OpenForm, &ctx);
        for message in [
            Message::NameChanged("Acme Supplies".into()),
            Message::WebsiteChanged("  acme.example  ".into()),
            Message::MailingAddressChanged("1 Mail Road".into()),
            Message::ShippingAddressChanged("Dock 4".into()),
            Message::PaymentMethodChanged("Cheque".into()),
            Message::DescriptionEdited(text_editor::Action::Edit(text_editor::Edit::Paste(
                std::sync::Arc::new("Stationery".into()),
            ))),
            Message::CustomFieldAdded,
            Message::CustomFieldNameChanged(0, "NTN".into()),
            Message::CustomFieldValueChanged(0, "123".into()),
            Message::CustomFieldAdded,
            Message::CustomFieldAdded,
            Message::CustomFieldRemoved(2),
        ] {
            update(&mut state, message, &ctx);
        }
        attach_file(&mut state, &ctx, &sample_file(&dir, "contract.pdf", b"pdf"));
        update(&mut state, Message::Create, &ctx);
        assert!(state.error.is_none(), "{:?}", state.error);

        let mut state = load(&ctx);
        let vendor = state.vendors[0].clone();
        assert_eq!(vendor.website.as_deref(), Some("acme.example"));
        assert_eq!(vendor.mailing_address.as_deref(), Some("1 Mail Road"));
        assert_eq!(vendor.shipping_address.as_deref(), Some("Dock 4"));
        assert_eq!(vendor.payment_method.as_deref(), Some("Cheque"));
        assert_eq!(vendor.description.as_deref(), Some("Stationery"));
        // The blank second custom field is dropped on save.
        assert_eq!(vendor.custom_fields, vec![CustomField::new("NTN", "123")]);
        assert_eq!(vendor.attachments.len(), 1);
        assert_eq!(vendor.attachments[0].display_name, "contract.pdf");

        // Editing loads every field back into the form, and saving again
        // keeps them.
        update(&mut state, Message::EditRow(vendor.id.unwrap()), &ctx);
        assert_eq!(state.form.website, "acme.example");
        assert_eq!(state.form.payment_method, "Cheque");
        assert_eq!(state.form.description_text(), "Stationery");
        update(&mut state, Message::AttachmentRemoved(0), &ctx);
        update(&mut state, Message::Create, &ctx);
        assert!(state.error.is_none(), "{:?}", state.error);
        let reloaded = load(&ctx).vendors[0].clone();
        assert!(reloaded.attachments.is_empty());
        assert_eq!(reloaded.custom_fields, vendor.custom_fields);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn search_also_matches_the_profile_text_fields() {
        let mut state = state_with(vendors(2, false));
        state.vendors[1].shipping_address = Some("Port Qasim".into());
        let ctx = ctx_with_company();
        update(&mut state, Message::SearchChanged("qasim".into()), &ctx);
        let page = vendor_page(&state);
        assert_eq!(page.rows.len(), 1);
        assert_eq!(page.rows[0].name, "Vendor 1");
    }

    #[test]
    fn the_export_writes_the_new_fields_after_the_original_columns() {
        let mut vendor = Vendor::new(1, "Acme");
        vendor.website = Some("acme.example".into());
        vendor.payment_method = Some("Bank".into());
        vendor.custom_fields = vec![CustomField::new("NTN", "1")];
        let row = export_row(&vendor);
        assert_eq!(row[..7], ["Acme", "", "", "", "Other", "30", "Active"]);
        assert_eq!(row[7], "acme.example");
        assert_eq!(row[10], "Bank");
        assert_eq!(row[12], "NTN: 1");
        assert_eq!(row[13], "");
    }

    /// A vendor with `count` custom fields and attachments.
    fn vendor_with_extras(count: usize) -> Vendor {
        let mut vendor = vendors(1, false).remove(0);
        vendor.website = Some("acme.example".into());
        vendor.payment_method = Some("Online".into());
        vendor.custom_fields = (0..count)
            .map(|index| CustomField::new(format!("Field {index}"), format!("Value {index}")))
            .collect();
        vendor.attachments = (0..count)
            .map(|index| VendorAttachment {
                file_name: format!("{index}-file.pdf"),
                display_name: format!("File {index}.pdf"),
                size_bytes: 1024 * index as u64,
            })
            .collect();
        vendor
    }

    #[test]
    fn the_panel_builds_in_view_and_edit_mode_open_and_closed_with_any_extras() {
        let ctx = ctx_with_company();
        for theme in THEMES {
            for width in WIDTHS {
                for extras in [0, 12] {
                    for (preview, open) in [(false, false), (false, true), (true, true)] {
                        for tab in 0..VENDOR_FORM_TABS.len() {
                            for renaming in [false, true] {
                                let mut state = state_with(vec![vendor_with_extras(extras)]);
                                update(&mut state, Message::EditRow(1), &ctx);
                                if renaming && extras > 0 {
                                    update(&mut state, Message::AttachmentRenameStarted(1), &ctx);
                                }
                                state.form_open = open;
                                state.show_preview = preview;
                                state.form.tab = tab;
                                let _: Element<'_, Message> =
                                    page(&state, theme, width, None, |message| message);
                            }
                        }
                    }
                }
            }
        }
    }
}
