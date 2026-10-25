use crate::backend::accounting::vendors::{
    compare_by_column, dash, day, default_column_visibility, export_headers, export_row, money,
    optional_input, sort_of, vendor_column_value, COL_ATTACHMENTS, COL_BALANCE, COL_COMPANY_NAME,
    COL_CREATE_TIMESTAMP, COL_CREDIT, COL_CUSTOM_FIELDS, COL_DEBIT, COL_DESCRIPTION, COL_EMAIL,
    COL_MAILING_ADDRESS, COL_MODIFIED_TIMESTAMP, COL_PAYMENT_METHOD, COL_PAYMENT_TERMS, COL_PHONE,
    COL_SELECT, COL_SERIAL, COL_SHIPPING_ADDRESS, COL_STATUS, COL_VENDOR_ID, COL_VENDOR_NAME,
    COL_WEBSITE, DEFAULT_PAGE_SIZE, FIRST_DATA_COLUMN, PAGE_SIZES, VENDOR_ALWAYS_SELECTED,
    VENDOR_HEADERS, VENDOR_OPTIONAL_COLUMNS, VENDOR_OPTIONAL_LABELS, VENDOR_STATUS_OPTIONS,
    VENDOR_TYPE_LABEL, VENDOR_TYPE_OPTIONS, VENDOR_VISIBLE_COLUMNS,
};
use crate::backend::core::reference_code;
use crate::backend::core::validation::Validate;
use crate::backend::parties::vendors::{
    CustomField, Vendor, VendorAttachment, VENDOR_PAYMENT_METHODS, VENDOR_TYPES,
};
use crate::backend::purchases::vendor_ledger::VendorLedger;
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

mod preview;
use preview::{refresh_ledger, vendor_preview, PREVIEW_TABS};

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
    ledger: Option<VendorLedger>,
    ledger_error: Option<String>,
    preview_tab: usize,
    ledger_newest_first: bool,
    /// Profile-column sections of the preview that are folded shut.
    collapsed_sections: HashSet<usize>,
    page: usize,
    page_size: usize,
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
    PreviewSectionToggled(usize),
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
        Message::PreviewSectionToggled(section) => {
            if !state.collapsed_sections.remove(&section) {
                state.collapsed_sections.insert(section);
            }
        }
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
fn vendor_live_preview(state: &State, theme: Theme) -> Element<'static, Message> {
    let form = &state.form;
    let amount_card =
        |label: &'static str, value: String, background: iced::Color, foreground: iced::Color| {
            container(
                column![
                    text(label).size(typography::caption()).color(foreground),
                    text(value).size(typography::body()).color(foreground),
                ]
                .spacing(spacing::SPACE_2)
                .width(Length::Fill),
            )
            .padding(spacing::XS)
            .width(Length::FillPortion(1))
            .style(move |_iced_theme: &iced::Theme| container::Style {
                background: Some(iced::Background::Color(background)),
                border: iced::Border {
                    radius: crate::frontend::components::states::RADIUS_MD.into(),
                    ..iced::Border::default()
                },
                ..container::Style::default()
            })
            .into()
        };
    let card = |column: usize, value: String| {
        forms::field_box(
            theme,
            VENDOR_HEADERS[column],
            text(dash(&value))
                .size(typography::body())
                .color(theme.neutral_font())
                .into(),
        )
    };
    let debit = state
        .ledger
        .as_ref()
        .map_or(Decimal::ZERO, VendorLedger::total_debit);
    let credit = state
        .ledger
        .as_ref()
        .map_or(Decimal::ZERO, VendorLedger::total_credit);
    forms::field_grid(
        vec![
            amount_card(
                VENDOR_HEADERS[COL_DEBIT],
                money(debit),
                theme.positive_bg(),
                theme.positive_front(),
            ),
            amount_card(
                VENDOR_HEADERS[COL_CREDIT],
                money(credit),
                theme.negative_bg(),
                theme.negative_front(),
            ),
            card(COL_MAILING_ADDRESS, form.mailing_address.clone()),
            card(COL_PAYMENT_METHOD, form.payment_method.clone()),
            card(COL_DESCRIPTION, form.description_text()),
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
            .push(lift(vendor_preview(state, theme, width)))
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
