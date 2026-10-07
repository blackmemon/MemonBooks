//! Schema-driven native document editor and searchable document lists.
use super::maintain::customers_view::{
    card_style, filled_button, outlined_button, page_items, primary_button,
};
use crate::backend::core::errors::{AppError, AppResult};
use crate::backend::services::{
    accounting_service::AccountingService, document_service::DocumentService,
    inventory_service::InventoryService, party_service::PartyService,
};
use crate::backend::transactions::documents::*;
use crate::frontend::styles::{
    badges, borders, button as buttons, checkbox as checkbox_style, colors::Theme, dropdown, fonts,
    headers, icons, inputs, panels, spacing, states, table as table_style, tabs, typography,
};
use crate::frontend::views::Ctx;
use iced::widget::{
    button, checkbox, column, container, pick_list, row, scrollable, stack, text, text_input,
    tooltip, Space,
};
use iced::{Alignment, Background, Element, Length};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Debug, Clone)]
pub struct State {
    pub sort_key: String,
    pub descending: bool,
    pub detail: String,
    pub kind: Kind,
    pub records: Vec<Document>,
    pub form: Option<Document>,
    pub search: String,
    pub status: String,
    pub selected: BTreeSet<i64>,
    pub choices: BTreeMap<String, Vec<String>>,
    pub message: String,
    pub confirm_delete: bool,
    pub columns: bool,
    pub hidden: BTreeSet<String>,
    pub page: usize,
    pub page_size: usize,
    pub templates: Vec<String>,
    pub template_name: String,
    pub show_attachments: bool,
    pub show_preview: bool,
    pub generate: bool,
    pub period: String,
    pub employee_ids: BTreeSet<i64>,
    pub employees: Vec<Document>,
    /// Customer/Vendor detail panel: `false` shows values, `true` inputs.
    pub master_edit: bool,
    pub master_tab: usize,
}
impl Default for State {
    fn default() -> Self {
        Self {
            sort_key: String::new(),
            descending: false,
            detail: String::new(),
            kind: Kind::PurchaseOrder,
            records: vec![],
            form: None,
            search: String::new(),
            status: "All".into(),
            selected: BTreeSet::new(),
            choices: BTreeMap::new(),
            message: String::new(),
            confirm_delete: false,
            columns: false,
            hidden: BTreeSet::new(),
            page: 0,
            page_size: 50,
            templates: vec![],
            template_name: String::new(),
            show_attachments: false,
            show_preview: false,
            generate: false,
            period: chrono::Utc::now().format("%Y-%m").to_string(),
            employee_ids: BTreeSet::new(),
            employees: vec![],
            master_edit: false,
            master_tab: 0,
        }
    }
}
#[derive(Debug, Clone)]
pub enum Message {
    Import,
    DownloadTemplate(String),
    Sort(String),
    Stock,
    Tree,
    Kind(Kind),
    Add,
    Close,
    Field(String, String),
    Line(usize, String, String),
    Status(String),
    AddRow,
    RemoveRow(usize),
    Save,
    Edit(i64),
    Search(String),
    Filter(String),
    Select(i64, bool),
    All(bool),
    Reset,
    Columns,
    Column(String, bool),
    Delete,
    ConfirmDelete,
    CancelDelete,
    Convert,
    Attach,
    Attachments,
    RemoveAttachment(usize),
    Preview,
    Export(String),
    Print,
    TemplateName(String),
    SaveTemplate,
    LoadTemplate(String),
    Page(usize),
    PageSize(usize),
    GenerateDialog,
    Period(String),
    EmployeeSelect(i64, bool),
    Generate,
    Mark(String),
    CustomField,
    RemoveCustom(String),
    MasterEdit(bool),
    MasterTab(usize),
    PreviewEdit,
}
pub fn load(ctx: &Ctx, kind: Kind) -> State {
    let mut s = State {
        kind,
        ..State::default()
    };
    if let Err(e) = reload(&mut s, ctx) {
        s.message = e.to_string();
    }
    s
}
fn reload(s: &mut State, ctx: &Ctx) -> AppResult<()> {
    let service = DocumentService::new(ctx.db.clone());
    s.records = service.list(ctx.company_id, s.kind)?;
    if s.kind == Kind::Employee {
        s.status = "Active".into();
    }
    if MASTERS.contains(&s.kind)
        || (!matches!(
            s.kind,
            Kind::Employee | Kind::Payroll | Kind::Trip | Kind::Expense
        ) && s.selected.is_empty())
    {
        s.selected.extend(s.records.iter().filter_map(|d| d.id));
    }
    s.templates = service.templates(ctx.company_id, s.kind)?;
    s.employees = service.list(ctx.company_id, Kind::Employee)?;
    let a = AccountingService::new(ctx.db.clone())
        .list_accounts(ctx.company_id)?
        .into_iter()
        .filter(|a| a.status == "Active")
        .map(|a| format!("{} · {}", a.id.unwrap_or_default(), a.account_name))
        .collect::<Vec<_>>();
    for key in [
        "Parent Account",
        "Account",
        "Payment Account",
        "Refund Account",
        "Cash Account",
        "Bank Account",
        "Contra Account",
        "Salary Expense Account",
        "Deductions Account",
    ] {
        s.choices.insert(key.into(), a.clone());
    }
    let parties = PartyService::new(ctx.db.clone());
    s.choices.insert(
        "Vendor".into(),
        parties
            .list_vendors(ctx.company_id)?
            .into_iter()
            .filter(|p| p.is_active)
            .map(|p| format!("{} · {}", p.id.unwrap_or_default(), p.name))
            .collect(),
    );
    s.choices.insert(
        "Customer".into(),
        parties
            .list_customers(ctx.company_id)?
            .into_iter()
            .filter(|p| p.is_active)
            .map(|p| format!("{} · {}", p.id.unwrap_or_default(), p.name))
            .collect(),
    );
    let inventory = InventoryService::new(ctx.db.clone());
    s.choices.insert(
        "Item".into(),
        inventory
            .list_products(ctx.company_id)?
            .into_iter()
            .filter(|p| p.is_active)
            .map(|p| format!("{} · {}", p.id.unwrap_or_default(), p.name))
            .collect(),
    );
    s.choices.insert(
        "Location".into(),
        inventory
            .list_warehouses(ctx.company_id)?
            .into_iter()
            .filter(|p| p.is_active)
            .map(|p| format!("{} · {}", p.id.unwrap_or_default(), p.name))
            .collect(),
    );
    s.choices.insert(
        "Employee".into(),
        s.employees
            .iter()
            .map(|p| {
                format!(
                    "{} · {} {}",
                    p.id.unwrap_or_default(),
                    p.get("First Name"),
                    p.get("Last Name")
                )
            })
            .collect(),
    );
    Ok(())
}
pub fn update(s: &mut State, m: Message, ctx: &Ctx) {
    if let Err(e) = handle(s, m, ctx) {
        s.message = e.to_string();
    }
}
fn handle(s: &mut State, m: Message, ctx: &Ctx) -> AppResult<()> {
    let service = DocumentService::new(ctx.db.clone());
    match m {
        Message::Sort(key) => {
            s.descending = s.sort_key == key && !s.descending;
            s.sort_key = key;
            s.page = 0;
        }
        Message::Stock => {
            let record = s
                .records
                .iter()
                .find(|d| d.id.is_some_and(|id| s.selected.contains(&id)))
                .ok_or_else(|| AppError::Validation("Select a location first".into()))?;
            s.detail = service.stock_report(ctx.company_id, record.get("Master ID"))?;
        }
        Message::Tree => {
            s.detail = service.account_tree(ctx.company_id)?;
        }
        Message::DownloadTemplate(format) => {
            let headers = s.kind.headers();
            if let Some(path) = rfd::FileDialog::new()
                .set_file_name(format!("{}-template.{format}", s.kind.prefix()))
                .save_file()
            {
                if format == "xlsx" {
                    crate::database::exports::xlsx::export_xlsx("Template", &headers, &[], &path)?;
                } else {
                    std::fs::write(
                        &path,
                        crate::database::exports::csv::export_csv(&headers, &[]),
                    )
                    .map_err(|source| AppError::Io {
                        path: path.clone(),
                        source,
                    })?;
                }
                s.message = format!("Template saved to {}", path.display());
            }
        }
        Message::Import => {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Master data", &["csv", "xlsx"])
                .pick_file()
            {
                let rows = crate::database::exports::import::read(&path)?;
                let (count, skipped) = service.import(ctx.company_id, s.kind, &rows)?;
                reload(s, ctx)?;
                s.message = format!("Imported {count}; skipped {skipped} blank records");
            }
        }
        Message::Kind(k) => *s = load(ctx, k),
        Message::PreviewEdit => {
            s.show_preview = false;
            s.master_edit = true;
        }
        Message::MasterEdit(edit) => s.master_edit = edit,
        Message::MasterTab(index) => {
            s.master_tab = index;
            s.show_preview = false;
        }
        Message::Add => {
            s.show_preview = false;
            s.master_edit = true;
            s.master_tab = 0;
            s.form = Some(Document::new(ctx.company_id, s.kind));
            if s.kind == Kind::Item {
                if let Some(d) = &mut s.form {
                    for location in s.choices.get("Location").into_iter().flatten() {
                        d.set(&format!("Stock at {location}"), "");
                    }
                }
            }
            s.message.clear();
        }
        Message::Close => {
            s.form = None;
            s.show_preview = false;
            s.master_edit = false;
            s.generate = false;
        }
        Message::Field(k, v) => {
            if k == "Employee" {
                if let Some(e) = s
                    .employees
                    .iter()
                    .find(|e| v.starts_with(&format!("{} · ", e.id.unwrap_or_default())))
                {
                    s.form = Some(service.payroll_for(e, &s.period)?);
                }
            } else if let Some(d) = &mut s.form {
                d.set(&k, v.clone());
                if k == "Category" && d.kind == Kind::Item {
                    d.set("Sub-Category", "");
                }
                if k == "Customer" || k == "Vendor" {
                    let kind = if k == "Customer" {
                        Kind::Customer
                    } else {
                        Kind::Vendor
                    };
                    let id = v.split(" · ").next().unwrap_or("");
                    if let Some(p) = service
                        .list(ctx.company_id, kind)?
                        .iter()
                        .find(|p| p.get("Master ID") == id)
                    {
                        for key in ["Mailing Address", "Shipping Address"] {
                            d.set(key, p.get(key));
                        }
                        d.set("Delivery Address", p.get("Shipping Address"));
                    }
                }
                if k == "Payment Method" && v == "Cash" {
                    d.set("Bank Name", "");
                    d.set("Account IBAN", "");
                }
                if k == "Experience Type" && v == "Fresh" {
                    for key in [
                        "Previous Company",
                        "Previous Designation",
                        "Previous Salary",
                        "Previous Leave Date",
                    ] {
                        d.set(key, "");
                    }
                }
                if k == "Transport Mode" && v == "Own" {
                    d.set("Transport Allowance", "");
                }
                if k == "Gratuity" && v != "Monthly" {
                    d.set("Gratuity Amount", "");
                }
                d.recalculate()?;
            }
        }
        Message::Line(i, k, v) => {
            if let Some(d) = &mut s.form {
                if let Some(l) = d.lines.get_mut(i) {
                    l.insert(k, v);
                }
                d.recalculate()?;
            }
        }
        Message::Status(v) => {
            if let Some(d) = &mut s.form {
                d.status = v;
            }
        }
        Message::AddRow => {
            if let Some(d) = &mut s.form {
                d.lines.push(Fields::new());
            }
        }
        Message::RemoveRow(i) => {
            if let Some(d) = &mut s.form {
                if i < d.lines.len() {
                    d.lines.remove(i);
                    d.recalculate()?;
                }
            }
        }
        Message::Save => {
            if let Some(d) = &s.form {
                let saved = service.save(d)?;
                s.form = Some(saved);
                reload(s, ctx)?;
                s.master_edit = false;
                s.message = "Saved successfully".into();
            }
        }
        Message::Edit(id) => {
            s.show_preview = false;
            s.master_edit = false;
            s.master_tab = 0;
            if let Some(d) = s.records.iter().find(|d| d.id == Some(id)) {
                if d.locked() {
                    s.message = "This record is locked. You can preview or export it.".into();
                }
                s.form = Some(d.clone());
            }
        }
        Message::Search(v) => {
            s.search = v;
            s.page = 0;
        }
        Message::Filter(v) => {
            s.show_preview = false;
            s.status = v;
            s.page = 0;
            if s.status != "All" && s.status != "Inactive" {
                s.selected.clear();
            }
        }
        Message::Select(id, yes) => {
            if yes {
                s.selected.insert(id);
            } else {
                s.selected.remove(&id);
            }
        }
        Message::All(yes) => {
            let ids = visible(s)
                .iter()
                .map(|d| d.id.unwrap_or_default())
                .collect::<Vec<_>>();
            for id in ids {
                if yes {
                    s.selected.insert(id);
                } else {
                    s.selected.remove(&id);
                }
            }
        }
        Message::Reset => {
            s.sort_key.clear();
            s.descending = false;
            s.detail.clear();
            s.search.clear();
            s.status = "All".into();
            s.page = 0;
        }
        Message::Columns => s.columns = !s.columns,
        Message::Column(k, yes) => {
            if yes {
                s.hidden.remove(&k);
            } else {
                s.hidden.insert(k);
            }
        }
        Message::Delete => {
            if s.selected.is_empty() {
                return invalid("Select records to delete");
            }
            s.confirm_delete = true;
        }
        Message::CancelDelete => s.confirm_delete = false,
        Message::ConfirmDelete => {
            let count = service.delete(
                ctx.company_id,
                &s.selected.iter().copied().collect::<Vec<_>>(),
            )?;
            s.confirm_delete = false;
            s.selected.clear();
            reload(s, ctx)?;
            s.message = format!("Deleted {count} records");
        }
        Message::Convert => {
            if let Some(d) = &s.form {
                let converted = d.convert()?;
                s.kind = converted.kind;
                reload(s, ctx)?;
                s.form = Some(converted);
                s.message = "Review and save the converted draft".into();
            }
        }
        Message::Attach => {
            if let Some(d) = &mut s.form {
                if d.locked() {
                    return invalid("Record is locked");
                }
                if let Some(files) = rfd::FileDialog::new().pick_files() {
                    for file in files {
                        if d.attachments.len() == 4 {
                            break;
                        }
                        let name = file.file_name().unwrap_or_default().to_string_lossy();
                        let target = ctx.upload_path(&format!(
                            "{}-{}-{}",
                            chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default(),
                            d.attachments.len(),
                            name
                        ));
                        std::fs::copy(&file, &target).map_err(|source| AppError::Io {
                            path: target.clone(),
                            source,
                        })?;
                        d.attachments.push(target.to_string_lossy().to_string());
                    }
                    s.message = "Attachments added; save to retain them".into();
                }
            }
        }
        Message::Attachments => s.show_attachments = !s.show_attachments,
        Message::RemoveAttachment(i) => {
            if let Some(d) = &mut s.form {
                if !d.locked() && i < d.attachments.len() {
                    d.attachments.remove(i);
                }
            }
        }
        Message::Preview => s.show_preview = !s.show_preview,
        Message::TemplateName(v) => s.template_name = v,
        Message::SaveTemplate => {
            if let Some(d) = &s.form {
                service.save_template(d, &s.template_name)?;
                reload(s, ctx)?;
                s.message = "Template saved".into();
            }
        }
        Message::LoadTemplate(name) => {
            s.master_edit = true;
            s.form = Some(service.template(ctx.company_id, s.kind, &name)?)
        }
        Message::Export(format) => export(s, ctx, &format, false)?,
        Message::Print => export(s, ctx, "pdf", true)?,
        Message::Page(p) => s.page = p,
        Message::PageSize(n) => {
            s.page_size = n;
            s.page = 0;
        }
        Message::GenerateDialog => {
            s.generate = true;
            s.employee_ids = s
                .employees
                .iter()
                .filter(|e| e.status == "Active")
                .filter_map(|e| e.id)
                .collect();
        }
        Message::Period(v) => s.period = v,
        Message::EmployeeSelect(id, yes) => {
            if yes {
                s.employee_ids.insert(id);
            } else {
                s.employee_ids.remove(&id);
            }
        }
        Message::Generate => {
            let (created, skipped) = service.generate_payroll(
                ctx.company_id,
                &s.period,
                &s.employee_ids.iter().copied().collect::<Vec<_>>(),
            )?;
            reload(s, ctx)?;
            s.generate = false;
            s.message = format!("Generated {created}; skipped {skipped} existing payroll records");
        }
        Message::Mark(status) => {
            if s.selected.is_empty() {
                return invalid("Select payroll records first");
            }
            for d in s
                .records
                .clone()
                .iter()
                .filter(|d| d.id.is_some_and(|id| s.selected.contains(&id)))
            {
                let mut d = d.clone();
                if d.id.is_some_and(|id| id < 0) {
                    d.id = None;
                }
                d.status = status.clone();
                if status == "Paid" && d.get("Payment Date").is_empty() {
                    d.set("Payment Date", chrono::Utc::now().date_naive().to_string());
                }
                service.save(&d)?;
            }
            reload(s, ctx)?;
            if MASTERS.contains(&s.kind) {
                s.selected = s
                    .records
                    .iter()
                    .filter_map(|record| (record.status == status).then_some(record.id).flatten())
                    .collect();
            }
            s.message = "Selected records updated".into();
        }
        Message::CustomField => {
            if let Some(d) = &mut s.form {
                let n = d.fields.keys().filter(|k| k.starts_with("Custom ")).count() + 1;
                d.set(&format!("Custom {n} name"), "");
                d.set(&format!("Custom {n} value"), "");
            }
        }
        Message::RemoveCustom(k) => {
            if let Some(d) = &mut s.form {
                d.fields.remove(&k);
            }
        }
    }
    Ok(())
}
fn export(s: &mut State, _ctx: &Ctx, format: &str, form: bool) -> AppResult<()> {
    let docs: Vec<&Document> = if form {
        vec![s
            .form
            .as_ref()
            .filter(|d| d.id.is_some())
            .ok_or_else(|| AppError::Validation("Save before printing".into()))?]
    } else {
        filtered(s)
            .into_iter()
            .filter(|d| d.id.is_some_and(|id| s.selected.contains(&id)))
            .collect()
    };
    if docs.is_empty() {
        return invalid("Select matching records to export");
    }
    if let Some(path) = rfd::FileDialog::new()
        .set_file_name(format!("{}.{}", s.kind.prefix(), format))
        .save_file()
    {
        let keys: Vec<String> = std::iter::once("Number")
            .chain(s.kind.headers())
            .chain(s.kind.footers())
            .chain(std::iter::once("Status"))
            .filter(|k| !s.hidden.contains(*k))
            .map(str::to_string)
            .collect();
        let headers: Vec<&str> = keys.iter().map(String::as_str).collect();
        let rows: Vec<Vec<String>> = docs
            .iter()
            .map(|d| {
                keys.iter()
                    .map(|key| {
                        if key == "Number" {
                            d.number.clone()
                        } else if key == "Status" {
                            d.status.clone()
                        } else {
                            d.fields.get(key).cloned().unwrap_or_default()
                        }
                    })
                    .collect()
            })
            .collect();
        match format {
            "pdf" => {
                crate::database::exports::pdf::export_pdf(s.kind.label(), &headers, &rows, &path)?
            }
            "xlsx" => {
                crate::database::exports::xlsx::export_xlsx("Documents", &headers, &rows, &path)?
            }
            _ => std::fs::write(
                &path,
                crate::database::exports::csv::export_csv(&headers, &rows),
            )
            .map_err(|source| AppError::Io {
                path: path.clone(),
                source,
            })?,
        }
        s.message = format!("Exported {}", path.display());
    }
    Ok(())
}
fn filtered(s: &State) -> Vec<&Document> {
    let search = s.search.to_lowercase();
    let mut records: Vec<_> = s
        .records
        .iter()
        .filter(|d| {
            (s.status == "All" || d.status == s.status)
                && (search.is_empty()
                    || d.number.to_lowercase().contains(&search)
                    || d.fields
                        .values()
                        .any(|v| v.to_lowercase().contains(&search))
                    || d.lines
                        .iter()
                        .any(|line| line.values().any(|v| v.to_lowercase().contains(&search))))
        })
        .collect();
    if !s.sort_key.is_empty() {
        records.sort_by(|a, b| {
            let av = if s.sort_key == "Number" {
                &a.number
            } else {
                a.get(&s.sort_key)
            };
            let bv = if s.sort_key == "Number" {
                &b.number
            } else {
                b.get(&s.sort_key)
            };
            let ordering = match (
                av.parse::<rust_decimal::Decimal>(),
                bv.parse::<rust_decimal::Decimal>(),
            ) {
                (Ok(a), Ok(b)) => a.cmp(&b),
                _ => av.to_lowercase().cmp(&bv.to_lowercase()),
            };
            if s.descending {
                ordering.reverse()
            } else {
                ordering
            }
        });
    }
    records
}
fn visible(s: &State) -> Vec<&Document> {
    filtered(s)
        .into_iter()
        .skip(s.page.saturating_mul(s.page_size))
        .take(s.page_size)
        .collect()
}
fn action<'a>(label: impl Into<String>, msg: Message, t: Theme) -> Element<'a, Message> {
    let label = label.into();
    let positive = label == "Save" || label == "Mark as Paid";
    let negative = label.contains("Delete") || label == "×";
    let add = label == "Add" || label == "Add Row";
    let b = button(text(label).size(typography::label()))
        .on_press(msg)
        .padding([spacing::SM, spacing::SPACE_12]);
    if positive {
        b.style(buttons::positive(t)).into()
    } else if negative {
        b.style(buttons::negative(t)).into()
    } else if add {
        b.style(buttons::primary(t)).into()
    } else {
        b.style(buttons::secondary(t)).into()
    }
}
pub fn view<'a>(s: &'a State, t: Theme, kinds: &'static [Kind]) -> Element<'a, Message> {
    if matches!(s.kind, Kind::Customer | Kind::Vendor) {
        return master_view(s, t, kinds);
    }
    let mut page = column![row![
        text(if s.kind == Kind::Trip {
            "Trips"
        } else {
            s.kind.label()
        })
        .size(typography::title()),
        pick_list(kinds, Some(s.kind), Message::Kind).style(dropdown::style(t)),
        Space::with_width(Length::Fill),
        action(
            if s.form.is_some() { "Return" } else { "Add" },
            if s.form.is_some() {
                Message::Close
            } else {
                Message::Add
            },
            t
        )
    ]
    .spacing(spacing::MD)
    .align_y(iced::Alignment::Center)]
    .spacing(spacing::MD)
    .padding(spacing::SPACE_20);
    if !s.message.is_empty() {
        page = page.push(
            container(text(&s.message).size(typography::body()))
                .padding(spacing::SPACE_10)
                .style(panels::flat_card(t, 6.)),
        );
    }
    if s.confirm_delete {
        page = page.push(
            row![
                text("Permanently delete the selected records?"),
                action("Delete", Message::ConfirmDelete, t),
                action("Cancel", Message::CancelDelete, t)
            ]
            .spacing(spacing::SPACE_12),
        );
    }
    if s.kind == Kind::Payroll && s.form.is_none() {
        let gross = s
            .records
            .iter()
            .filter_map(|d| num(&d.fields, "Gross Salary").ok())
            .sum::<rust_decimal::Decimal>();
        let unpaid = s
            .records
            .iter()
            .filter(|d| d.status != "Paid")
            .filter_map(|d| num(&d.fields, "Net Pay").ok())
            .sum::<rust_decimal::Decimal>();
        let mut cards = row![].spacing(spacing::MD);
        for (label, value) in [
            ("Employees", s.employees.len().to_string()),
            ("Payroll Records", s.records.len().to_string()),
            ("Total Gross Payroll", gross.to_string()),
            ("Net Payable (Unpaid)", unpaid.to_string()),
        ] {
            cards = cards.push(
                container(
                    column![text(label), text(value).size(typography::stat_value())]
                        .spacing(spacing::SPACE_18),
                )
                .padding(spacing::SPACE_20)
                .width(Length::Fill)
                .style(panels::flat_card(t, 10.)),
            );
        }
        page = page.push(cards).push(
            row![
                action("Employee", Message::Kind(Kind::Employee), t),
                action("Generate Monthly Payroll", Message::GenerateDialog, t),
                action("Mark as Paid", Message::Mark("Paid".into()), t),
                action("Mark as Draft", Message::Mark("Draft".into()), t)
            ]
            .spacing(spacing::SPACE_8),
        );
    }
    if s.kind == Kind::Employee {
        page = page.push(action("Payroll", Message::Kind(Kind::Payroll), t));
    }
    if s.generate {
        let mut generate = column![
            text("Generate monthly payroll"),
            text_input("YYYY-MM", &s.period).on_input(Message::Period)
        ]
        .spacing(spacing::SPACE_8);
        for e in s.employees.iter().filter(|e| e.status == "Active") {
            let id = e.id.unwrap_or_default();
            generate = generate.push(
                checkbox(
                    format!("{} {}", e.get("First Name"), e.get("Last Name")),
                    s.employee_ids.contains(&id),
                )
                .on_toggle(move |v| Message::EmployeeSelect(id, v)),
            );
        }
        page = page.push(generate.push(action("Generate", Message::Generate, t)));
    }
    if let Some(d) = &s.form {
        page = page.push(form(s, d, t));
    } else {
        let status_options = std::iter::once("All".to_string())
            .chain(s.kind.statuses().iter().map(|v| v.to_string()))
            .collect::<Vec<_>>();
        page = page.push(
            row![
                text_input("Search records…", &s.search)
                    .on_input(Message::Search)
                    .style(inputs::text(t))
                    .padding(spacing::SPACE_10),
                pick_list(status_options, Some(s.status.clone()), Message::Filter)
                    .style(dropdown::style(t)),
                action("Reset All", Message::Reset, t),
                action("Columns", Message::Columns, t),
                action("CSV", Message::Export("csv".into()), t),
                action("XLSX", Message::Export("xlsx".into()), t),
                action("PDF", Message::Export("pdf".into()), t)
            ]
            .spacing(spacing::SPACE_8),
        );
        if s.columns {
            for chunk in s
                .kind
                .headers()
                .into_iter()
                .chain(s.kind.footers())
                .collect::<Vec<_>>()
                .chunks(4)
            {
                let mut cols = row![].spacing(spacing::SPACE_12);
                for k in chunk {
                    let key = k.to_string();
                    cols = cols.push(
                        checkbox(key.clone(), !s.hidden.contains(*k))
                            .on_toggle(move |v| Message::Column(key.clone(), v))
                            .width(Length::Fill),
                    );
                }
                page = page.push(cols);
            }
        }
        if MASTERS.contains(&s.kind) {
            let mut bar = row![
                action("Archive Selected", Message::Mark("Inactive".into()), t),
                action("Restore Selected", Message::Mark("Active".into()), t),
                action("Import", Message::Import, t),
                action("CSV Template", Message::DownloadTemplate("csv".into()), t),
                action(
                    "Excel Template",
                    Message::DownloadTemplate("xlsx".into()),
                    t
                )
            ]
            .spacing(spacing::SPACE_8);
            if s.kind == Kind::Account {
                bar = bar.push(action("Tree View", Message::Tree, t));
            }
            if s.kind == Kind::Location {
                bar = bar.push(action("View Stock", Message::Stock, t));
            }
            page = page.push(bar);
        }
        if !s.detail.is_empty() {
            page = page.push(
                container(text(&s.detail))
                    .padding(spacing::SPACE_12)
                    .style(panels::flat_card(t, 8.)),
            );
        }
        page = page.push(
            row![
                checkbox(
                    "Select visible",
                    !visible(s).is_empty()
                        && visible(s)
                            .iter()
                            .all(|d| s.selected.contains(&d.id.unwrap_or_default()))
                )
                .on_toggle(Message::All),
                action("Delete Selected", Message::Delete, t)
            ]
            .spacing(spacing::SPACE_20),
        );
        let mut table = column![row![
            text("Number").width(Length::FillPortion(2)),
            text("Date / Period").width(Length::FillPortion(2)),
            text("Party / Name").width(Length::FillPortion(4)),
            text("Amount").width(Length::FillPortion(2)),
            text("Status").width(Length::FillPortion(2))
        ]
        .padding(spacing::SPACE_12)]
        .spacing(spacing::SPACE_1);
        for d in visible(s) {
            let id = d.id.unwrap_or_default();
            let name = [
                "Vendor",
                "Customer",
                "Name",
                "Subject",
                "Employee Name",
                "First Name",
            ]
            .iter()
            .map(|k| d.get(k))
            .find(|v| !v.is_empty())
            .unwrap_or("");
            let date = ["Date", "Period", "Departure Date", "Hire Date"]
                .iter()
                .map(|k| d.get(k))
                .find(|v| !v.is_empty())
                .unwrap_or("");
            let total = [
                "Total Amount",
                "Net Pay",
                "Total",
                "Budget limit",
                "Gross Salary",
            ]
            .iter()
            .map(|k| d.get(k))
            .find(|v| !v.is_empty())
            .unwrap_or("");
            table = table.push(
                container(
                    row![
                        checkbox("", s.selected.contains(&id))
                            .on_toggle(move |v| Message::Select(id, v)),
                        button(text(&d.number).size(typography::body()))
                            .on_press(Message::Edit(id))
                            .style(buttons::secondary(t))
                            .width(Length::FillPortion(2)),
                        text(if s.hidden.contains("Date") { "" } else { date })
                            .width(Length::FillPortion(2)),
                        text(if s.hidden.contains("Party / Name") {
                            ""
                        } else {
                            name
                        })
                        .width(Length::FillPortion(4)),
                        text(if s.hidden.contains("Total Amount") {
                            ""
                        } else {
                            total
                        })
                        .width(Length::FillPortion(2)),
                        text(if s.hidden.contains("Status") {
                            ""
                        } else {
                            &d.status
                        })
                        .width(Length::FillPortion(2))
                    ]
                    .spacing(spacing::SPACE_8)
                    .align_y(iced::Alignment::Center),
                )
                .padding(spacing::SPACE_10)
                .style(panels::flat_card(t, 0.)),
            );
        }
        if s.records.is_empty() {
            table = table.push(
                container(text("No records yet. Select Add to get started."))
                    .padding(spacing::SPACE_30),
            );
        }
        page = page
            .push(if MASTERS.contains(&s.kind) {
                detailed_table(s, t)
            } else {
                container(table).style(panels::flat_card(t, 8.)).into()
            })
            .push(
                row![
                    Space::with_width(Length::Fill),
                    pick_list(
                        vec![25usize, 50, 100, 10000],
                        Some(s.page_size),
                        Message::PageSize
                    ),
                    action("Prev", Message::Page(s.page.saturating_sub(1)), t),
                    text(format!(
                        "Page {} · {} records",
                        s.page + 1,
                        filtered(s).len()
                    )),
                    action(
                        "Next",
                        Message::Page(if (s.page + 1) * s.page_size < filtered(s).len() {
                            s.page + 1
                        } else {
                            s.page
                        }),
                        t
                    )
                ]
                .spacing(spacing::SPACE_12)
                .align_y(iced::Alignment::Center),
            );
    }
    scrollable(page).height(Length::Fill).into()
}
fn form<'a>(s: &'a State, d: &'a Document, t: Theme) -> Element<'a, Message> {
    let mut content = column![text(if d.number.is_empty() {
        "Number assigned on save"
    } else {
        &d.number
    })
    .size(typography::emphasis())]
    .spacing(spacing::MD);
    let headers = d.kind.headers();
    for chunk in headers.chunks(3) {
        let mut r = row![].spacing(spacing::SPACE_14);
        for key in chunk {
            if !hidden_field(d, key) {
                r = r.push(field(s, d, -1, key, t));
            }
        }
        content = content.push(r);
    }
    if !d.kind.lines().is_empty() {
        content = content.push(text(format!("{} Details", d.kind)).size(typography::emphasis()));
        let mut lines = column![].spacing(spacing::SPACE_10);
        for (i, line) in d.lines.iter().enumerate() {
            let mut r = row![].spacing(spacing::SPACE_8);
            for key in d.kind.lines() {
                let _ = line;
                r = r.push(field(s, d, i as i64, key, t));
            }
            if !d.locked() {
                r = r.push(action("×", Message::RemoveRow(i), t));
            }
            lines = lines.push(r);
        }
        content = content.push(
            scrollable(
                container(lines).width(Length::Fixed(if d.kind.lines().len() > 6 {
                    1400.
                } else {
                    1000.
                })),
            )
            .direction(scrollable::Direction::Horizontal(
                scrollable::Scrollbar::default(),
            )),
        );
        if !d.locked() {
            content = content.push(action("Add Row", Message::AddRow, t));
        }
    }
    for chunk in d.kind.footers().chunks(3) {
        let mut r = row![].spacing(spacing::SPACE_14);
        for key in chunk {
            r = r.push(field(s, d, -1, key, t));
        }
        content = content.push(r);
    }
    for k in d
        .fields
        .keys()
        .filter(|k| k.starts_with("Custom ") || k.starts_with("Stock at "))
    {
        content = content.push(
            row![
                field(s, d, -1, k, t),
                action("Remove", Message::RemoveCustom(k.clone()), t)
            ]
            .spacing(spacing::SPACE_8),
        );
    }
    if matches!(d.kind, Kind::Employee | Kind::Vendor) && !d.locked() {
        content = content.push(action("Add Custom Field", Message::CustomField, t));
    }
    let status: Element<'a, Message> = if d.locked() {
        text(format!("{} · Read only", d.status)).into()
    } else {
        pick_list(
            d.kind
                .statuses()
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>(),
            Some(d.status.clone()),
            Message::Status,
        )
        .style(dropdown::style(t))
        .into()
    };
    let mut actions = row![
        status,
        action("Attachments", Message::Attach, t),
        action(
            format!("Preview files ({})", d.attachments.len()),
            Message::Attachments,
            t
        ),
        action("Preview", Message::Preview, t),
        Space::with_width(Length::Fill),
        action("Print PDF", Message::Print, t)
    ]
    .spacing(spacing::SPACE_8);
    if !d.locked() {
        actions = actions.push(action("Save", Message::Save, t));
    }
    content = content.push(actions);
    if let Some(next) = d.kind.next() {
        content = content.push(action(format!("Convert to {next}"), Message::Convert, t));
    }
    if !matches!(
        d.kind,
        Kind::Employee | Kind::Payroll | Kind::Trip | Kind::Expense
    ) {
        content = content.push(
            row![
                text_input("Template name", &s.template_name)
                    .on_input(Message::TemplateName)
                    .style(inputs::text(t)),
                action("Save as Template", Message::SaveTemplate, t),
                pick_list(s.templates.clone(), None::<String>, Message::LoadTemplate)
                    .placeholder("Load Template")
            ]
            .spacing(spacing::SPACE_10),
        );
    }
    if s.show_attachments {
        for (i, path) in d.attachments.iter().enumerate() {
            content = content.push(row![
                text(path).width(Length::Fill),
                action("Remove", Message::RemoveAttachment(i), t)
            ]);
        }
    }
    if s.show_preview {
        content = content.push(text(
            d.fields
                .iter()
                .map(|(k, v)| format!("{k}: {v}"))
                .collect::<Vec<_>>()
                .join("\n"),
        ));
    }
    container(content)
        .padding(spacing::SPACE_18)
        .style(panels::flat_card(t, 10.))
        .into()
}
fn hidden_field(d: &Document, k: &str) -> bool {
    (d.get("Payment Method") == "Cash" && matches!(k, "Bank Name" | "Account IBAN"))
        || (d.get("Experience Type") == "Fresh" && k.starts_with("Previous "))
        || (k == "Qualification Complete" && d.get("Qualification Status") != "Completed")
        || (k == "Gratuity Amount" && d.get("Gratuity") != "Monthly")
        || (k == "Transport Allowance" && d.get("Transport Mode") == "Own")
        || (k == "Contra Account" && d.get("Receipt Type") != "Petty")
        || (matches!(k, "Vendor" | "Customer")
            && matches!(d.get("Receipt Type"), "Petty" | "Multiparty"))
}
fn field<'a>(
    s: &'a State,
    d: &'a Document,
    line: i64,
    key: &str,
    t: Theme,
) -> Element<'a, Message> {
    field_with(s, d, line, key, t, false)
}
/// `muted` draws the label small and gray (detail-panel style); every
/// other form keeps the original label.
fn field_control<'a>(
    s: &'a State,
    d: &'a Document,
    line: i64,
    key: &str,
    t: Theme,
) -> Element<'a, Message> {
    let k = key.to_string();
    let values = if line < 0 {
        &d.fields
    } else {
        &d.lines[line as usize]
    };
    let value = get(values, key);
    let readonly = d.locked() || (calculated(key) && !(d.kind == Kind::Expense && key == "Total"));
    let opts = if d.kind == Kind::Item && key == "Category" {
        ["Product", "Service", "Other"]
            .iter()
            .map(|value| value.to_string())
            .collect()
    } else if d.kind == Kind::Item && key == "Sub-Category" {
        match d.get("Category") {
            "Product" => vec!["Raw Material", "Finished Goods", "Other"],
            "Service" => vec!["Labor", "Consulting", "Maintenance", "Other"],
            _ => vec!["Other"],
        }
        .into_iter()
        .map(str::to_string)
        .collect()
    } else {
        s.choices
            .get(key)
            .cloned()
            .unwrap_or_else(|| options(key).iter().map(|s| s.to_string()).collect())
    };
    let control: Element<'a, Message> = if readonly {
        container(text(if value.is_empty() { "0" } else { value }))
            .padding(spacing::SPACE_10)
            .style(panels::flat_card(t, 6.))
            .width(Length::Fill)
            .into()
    } else if !opts.is_empty() || s.choices.contains_key(key) {
        let selected = if value.is_empty() {
            None
        } else {
            Some(value.to_string())
        };
        pick_list(opts, selected, move |v| {
            if line < 0 {
                Message::Field(k.clone(), v)
            } else {
                Message::Line(line as usize, k.clone(), v)
            }
        })
        .placeholder("Select…")
        .style(dropdown::style(t))
        .padding(spacing::SPACE_10)
        .width(Length::Fill)
        .into()
    } else {
        text_input(if date_field(key) { "YYYY-MM-DD" } else { key }, value)
            .on_input(move |v| {
                if line < 0 {
                    Message::Field(k.clone(), v)
                } else {
                    Message::Line(line as usize, k.clone(), v)
                }
            })
            .style(inputs::text(t))
            .padding(spacing::SPACE_10)
            .into()
    };
    control
}
fn field_with<'a>(
    s: &'a State,
    d: &'a Document,
    line: i64,
    key: &str,
    t: Theme,
    muted: bool,
) -> Element<'a, Message> {
    let control = field_control(s, d, line, key, t);
    let label = if muted {
        text(key.to_string())
            .size(typography::caption())
            .color(t.neutral_front())
    } else {
        text(key.to_string()).size(typography::label())
    };
    column![label, control]
        .spacing(spacing::SPACE_5)
        .width(Length::Fill)
        .into()
}
fn detailed_table(s: &State, t: Theme) -> Element<'_, Message> {
    let keys: Vec<_> = s
        .kind
        .headers()
        .into_iter()
        .filter(|k| !s.hidden.contains(*k))
        .collect();
    let mut header = row![
        text("Select").width(48),
        action("Number", Message::Sort("Number".into()), t)
    ]
    .spacing(spacing::SPACE_8);
    for key in &keys {
        header = header.push(
            button(text(*key).size(typography::label()))
                .on_press(Message::Sort(key.to_string()))
                .width(180)
                .style(buttons::secondary(t)),
        );
    }
    header = header.push(text("Status").width(100));
    let mut grid = column![container(header).padding(spacing::SPACE_10)].spacing(spacing::SPACE_1);
    for d in visible(s) {
        let id = d.id.unwrap_or_default();
        let mut r = row![
            checkbox("", s.selected.contains(&id))
                .on_toggle(move |v| Message::Select(id, v))
                .width(48),
            button(text(&d.number).size(typography::body()))
                .on_press(Message::Edit(id))
                .width(150)
                .style(buttons::secondary(t))
        ]
        .spacing(spacing::SPACE_8);
        for key in &keys {
            r = r.push(text(d.get(key)).size(typography::body()).width(180));
        }
        r = r.push(text(&d.status).width(100));
        grid = grid.push(
            container(r)
                .padding(spacing::SPACE_10)
                .style(panels::flat_card(t, 0.)),
        );
    }
    scrollable(container(grid).style(panels::flat_card(t, 8.)))
        .direction(scrollable::Direction::Horizontal(
            scrollable::Scrollbar::default(),
        ))
        .into()
}

// ---------------------------------------------------------------------
// Customers / Vendors master pages: tabs, search row, table and a right
// detail panel. Only `view` reaches this for `Kind::Customer`/`Vendor`;
// every other kind keeps the layout above.
// ---------------------------------------------------------------------

const MARGIN: f32 = spacing::SPACE_20;
const GLYPH_SEARCH: &str = icons::SEARCH;
const GLYPH_RESET: &str = icons::RESET;
const GLYPH_COLUMNS: &str = icons::COLUMNS;
const GLYPH_CLOSE: &str = icons::CLOSE;
/// Table columns the Columns picker can show/hide.
const MASTER_COLUMNS: [&str; 5] = ["Number", "Name", "Phone", "Email", "Status"];

fn master_name_key(kind: Kind) -> &'static str {
    if kind == Kind::Customer {
        "Customer Name"
    } else {
        "Vendor Name"
    }
}

fn master_status_label(status: &str) -> &str {
    if status == "Inactive" {
        "Archived"
    } else {
        status
    }
}

/// One tab: label, optional count badge, and an underline when active.
fn master_tab<'a>(
    t: Theme,
    label: &str,
    count: Option<usize>,
    active: bool,
    on_press: Message,
) -> Element<'a, Message> {
    let mut label_text = text(label.to_string())
        .size(typography::label())
        .color(tabs::text_color(t, active));
    if active {
        label_text = label_text.font(fonts::BOLD);
    }
    let mut inner = row![label_text]
        .spacing(spacing::SPACE_8)
        .align_y(Alignment::Center);
    if let Some(count) = count {
        inner = inner.push(
            container(
                text(count.to_string())
                    .size(typography::caption())
                    .color(if active {
                        t.primary()
                    } else {
                        t.neutral_front()
                    }),
            )
            .padding([spacing::SPACE_2, spacing::SPACE_8])
            .style(move |_iced_theme: &iced::Theme| container::Style {
                background: Some(Background::Color(if active {
                    t.primary_tint(states::SELECTED_ALPHA)
                } else {
                    t.neutral_bg()
                })),
                border: iced::Border {
                    radius: states::RADIUS_PILL.into(),
                    ..iced::Border::default()
                },
                ..container::Style::default()
            }),
        );
    }
    column![
        button(inner)
            .style(tabs::tab_button(t))
            .padding([spacing::SPACE_6, spacing::SPACE_4])
            .on_press(on_press),
        container(Space::with_height(Length::Fixed(2.0)))
            .width(Length::Fill)
            .style(tabs::underline(t, active)),
    ]
    .spacing(spacing::SPACE_2)
    .width(Length::Shrink)
    .into()
}

fn master_icon_button<'a>(
    t: Theme,
    glyph: &'static str,
    tip: &'static str,
    message: Message,
) -> Element<'a, Message> {
    tooltip(
        button(text(glyph).size(typography::emphasis()).center())
            .width(Length::Fixed(36.0))
            .height(Length::Fixed(36.0))
            .style(outlined_button(t, false))
            .on_press(message),
        container(text(tip).size(typography::caption()))
            .padding([spacing::SPACE_4, spacing::SPACE_8])
            .style(card_style(t)),
        tooltip::Position::Bottom,
    )
    .into()
}

fn master_small_button<'a>(
    t: Theme,
    label: &'static str,
    message: Message,
) -> Element<'a, Message> {
    button(text(label).size(typography::label()))
        .padding([spacing::SPACE_8, spacing::SPACE_12])
        .style(outlined_button(t, false))
        .on_press(message)
        .into()
}

fn master_view<'a>(s: &'a State, t: Theme, kinds: &'static [Kind]) -> Element<'a, Message> {
    let open = s.form.is_some();
    let add_label = if s.kind == Kind::Customer {
        "Add Customer"
    } else {
        "Add Vendor"
    };
    let header = row![
        text(s.kind.label())
            .size(headers::page_title_size())
            .font(fonts::BOLD)
            .color(t.neutral_font()),
        pick_list(kinds, Some(s.kind), Message::Kind)
            .style(dropdown::style(t))
            .padding(spacing::SM),
        Space::with_width(Length::Fill),
        button(
            text(if open { "Return" } else { add_label })
                .size(typography::label())
                .font(fonts::BOLD)
        )
        .padding([spacing::SPACE_10, spacing::MD])
        .style(primary_button(t))
        .on_press(if open { Message::Close } else { Message::Add }),
    ]
    .spacing(spacing::SPACE_12)
    .align_y(Alignment::Center);

    let mut page = column![header].spacing(spacing::SPACE_12).padding(MARGIN);
    if !s.message.is_empty() {
        page = page.push(
            container(text(&s.message).size(typography::body()))
                .padding(spacing::SPACE_10)
                .style(panels::flat_card(t, 6.)),
        );
    }
    if s.confirm_delete {
        page = page.push(
            row![
                text("Permanently delete the selected records?"),
                action("Delete", Message::ConfirmDelete, t),
                action("Cancel", Message::CancelDelete, t)
            ]
            .spacing(spacing::SPACE_12),
        );
    }

    // Preview replaces the table and panel inline, under the same header;
    // all other state is untouched, so Back returns to exactly this page.
    if s.show_preview {
        if let Some(d) = &s.form {
            return page
                .push(master_preview(d, t))
                .width(Length::Fill)
                .height(Length::Fill)
                .into();
        }
    }

    let count = |status: &str| s.records.iter().filter(|d| d.status == status).count();
    let tab_row = row![
        master_tab(
            t,
            "All",
            Some(s.records.len()),
            s.status == "All",
            Message::Filter("All".into())
        ),
        master_tab(
            t,
            "Active",
            Some(count("Active")),
            s.status == "Active",
            Message::Filter("Active".into())
        ),
        master_tab(
            t,
            "Archived",
            Some(count("Inactive")),
            s.status == "Inactive",
            Message::Filter("Inactive".into())
        ),
    ]
    .spacing(spacing::LG);
    page = page.push(tab_row);

    let search_input = text_input("Search records…", &s.search)
        .on_input(Message::Search)
        .style(inputs::text(t))
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
            text(GLYPH_SEARCH)
                .size(typography::label())
                .color(t.neutral_front())
        )
        .padding([spacing::SPACE_10, spacing::SPACE_12])
    ]
    .width(Length::FillPortion(9));
    page = page.push(
        row![
            search_box,
            master_icon_button(t, GLYPH_RESET, "Reset all", Message::Reset),
            master_icon_button(t, GLYPH_COLUMNS, "Columns", Message::Columns),
            master_small_button(t, "CSV", Message::Export("csv".into())),
            master_small_button(t, "XLSX", Message::Export("xlsx".into())),
            master_small_button(t, "PDF", Message::Export("pdf".into())),
            Space::with_width(Length::FillPortion(7)),
        ]
        .spacing(spacing::SPACE_12)
        .align_y(Alignment::Center),
    );

    // Bulk actions only while something is selected; Import and the
    // blank templates are always available on the right.
    let mut bulk = row![].spacing(spacing::SPACE_12).align_y(Alignment::Center);
    if !s.selected.is_empty() {
        let gray = || filled_button(t, t.secondary(), t.secondary_hover());
        let red = filled_button(
            t,
            t.negative_front(),
            t.negative_front()
                .scale_alpha(states::SOLID_BUTTON_HOVER_ALPHA),
        );
        let bulk_button = |label: &'static str| {
            button(text(label).size(typography::label()).font(fonts::BOLD))
                .padding([spacing::SM, spacing::SPACE_14])
        };
        bulk = bulk
            .push(
                text(format!("{} selected", s.selected.len()))
                    .size(typography::label())
                    .color(t.neutral_front()),
            )
            .push(
                bulk_button("Archive Selected")
                    .style(gray())
                    .on_press(Message::Mark("Inactive".into())),
            )
            .push(
                bulk_button("Restore Selected")
                    .style(gray())
                    .on_press(Message::Mark("Active".into())),
            )
            .push(
                bulk_button("Delete Selected")
                    .style(red)
                    .on_press(Message::Delete),
            );
    }
    page = page.push(
        row![
            bulk,
            Space::with_width(Length::Fill),
            master_small_button(t, "Import", Message::Import),
            master_small_button(t, "CSV Template", Message::DownloadTemplate("csv".into())),
            master_small_button(
                t,
                "Excel Template",
                Message::DownloadTemplate("xlsx".into())
            ),
        ]
        .spacing(spacing::SPACE_8)
        .align_y(Alignment::Center),
    );
    if !s.detail.is_empty() {
        page = page.push(
            container(text(&s.detail))
                .padding(spacing::SPACE_12)
                .style(panels::flat_card(t, 8.)),
        );
    }

    // Table (about 2/3) and the detail panel (about 1/3), hidden until a
    // row is clicked, Add is pressed, or Columns is opened.
    let panel_open = open || s.columns;
    let table = scrollable(master_table(s, t))
        .direction(scrollable::Direction::Vertical(master_scrollbar()))
        .height(Length::Fill);
    let mut main = row![container(table).width(if panel_open {
        Length::FillPortion(2)
    } else {
        Length::Fill
    })]
    .spacing(MARGIN)
    .height(Length::Fill);
    if panel_open {
        // The columns picker sits above the form; the form fills the rest.
        let mut side = column![].spacing(MARGIN).height(Length::Fill);
        if s.columns {
            side = side.push(master_columns(s, t));
        }
        if let Some(d) = &s.form {
            side = side.push(master_form(s, d, t));
        }
        main = main.push(
            container(side)
                .padding(spacing::MD)
                .width(Length::FillPortion(1))
                .height(Length::Fill)
                .style(move |iced_theme: &iced::Theme| container::Style {
                    border: borders::card(t, states::RADIUS_LG),
                    ..card_style(t)(iced_theme)
                }),
        );
    }
    page = page.push(main);

    // Pagination: Previous on the left, rows per page and page numbers on
    // the right.
    let total = filtered(s).len();
    let pages = total.div_ceil(s.page_size.max(1)).max(1);
    let current = s.page.min(pages - 1);
    let mut numbers = row![].spacing(spacing::SPACE_6).align_y(Alignment::Center);
    for item in page_items(current, pages) {
        numbers = numbers.push(match item {
            Some(index) => Element::from(
                button(
                    text((index + 1).to_string())
                        .size(typography::label())
                        .center(),
                )
                .width(Length::Fixed(32.0))
                .padding([spacing::SPACE_4, 0.0])
                .style(outlined_button(t, index == current))
                .on_press(Message::Page(index)),
            ),
            None => text("...")
                .size(typography::label())
                .color(t.neutral_front())
                .into(),
        });
    }
    page.push(
        row![
            button(text("Previous").size(typography::label()))
                .padding([spacing::SM, spacing::MD])
                .style(outlined_button(t, false))
                .on_press_maybe((current > 0).then(|| Message::Page(current - 1))),
            Space::with_width(Length::Fill),
            text("Rows per page:")
                .size(typography::label())
                .color(t.neutral_front()),
            pick_list(
                vec![25usize, 50, 100, 10000],
                Some(s.page_size),
                Message::PageSize
            )
            .style(dropdown::style(t))
            .padding([spacing::SM, spacing::SPACE_12]),
            numbers,
        ]
        .spacing(spacing::SPACE_12)
        .align_y(Alignment::Center),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

/// The list: header row with sortable column titles and a select-all
/// checkbox, then one clickable row per record.
fn master_table<'a>(s: &'a State, t: Theme) -> Element<'a, Message> {
    let rows = visible(s);
    let name_key = master_name_key(s.kind);
    let shown = |label: &str| !s.hidden.contains(label);
    let portion = |label: &str| match label {
        "Number" => 2,
        "Name" => 4,
        "Phone" => 2,
        "Email" => 3,
        _ => 2,
    };

    let all_selected = !rows.is_empty()
        && rows
            .iter()
            .all(|d| s.selected.contains(&d.id.unwrap_or_default()));
    let mut head = row![checkbox("", all_selected)
        .on_toggle(Message::All)
        .style(checkbox_style::style(t))
        .width(Length::Fixed(28.0))]
    .spacing(spacing::SPACE_8)
    .align_y(Alignment::Center);
    for label in MASTER_COLUMNS.iter().filter(|l| shown(l)) {
        let key = match *label {
            "Name" => Some(name_key),
            "Status" => None,
            other => Some(other),
        };
        let mut title = row![text(*label)
            .size(typography::label())
            .font(fonts::BOLD)
            .color(t.neutral_font())]
        .spacing(spacing::SPACE_4);
        let cell: Element<'a, Message> = match key {
            Some(key) => {
                if s.sort_key == key {
                    title = title.push(
                        text(if s.descending {
                            icons::SORT_DESCENDING
                        } else {
                            icons::SORT_ASCENDING
                        })
                        .size(typography::caption())
                        .color(t.primary()),
                    );
                }
                button(title)
                    .padding(0)
                    .style(buttons::plain(t))
                    .on_press(Message::Sort(key.to_string()))
                    .into()
            }
            None => title.into(),
        };
        head = head.push(container(cell).width(Length::FillPortion(portion(label))));
    }
    let mut list = column![container(head)
        .padding([spacing::SPACE_10, spacing::SPACE_12])
        .width(Length::Fill)
        .style(table_style::header_row(t, states::RADIUS_LG))]
    .width(Length::Fill);

    let editing_id = s.form.as_ref().and_then(|f| f.id);
    for d in rows {
        let id = d.id.unwrap_or_default();
        let subtitle = [d.get("Company Name"), d.get("Phone")]
            .into_iter()
            .find(|v| !v.is_empty())
            .unwrap_or("");
        let mut cells = row![checkbox("", s.selected.contains(&id))
            .on_toggle(move |v| Message::Select(id, v))
            .style(checkbox_style::style(t))
            .width(Length::Fixed(28.0))]
        .spacing(spacing::SPACE_8)
        .align_y(Alignment::Center);
        for label in MASTER_COLUMNS.iter().filter(|l| shown(l)) {
            let content: Element<'a, Message> = match *label {
                "Number" => text(&d.number).size(typography::body()).into(),
                "Name" => column![
                    text(d.get(name_key))
                        .size(typography::body())
                        .font(fonts::BOLD)
                        .color(t.neutral_font()),
                    text(subtitle)
                        .size(typography::caption())
                        .color(t.neutral_front()),
                ]
                .spacing(spacing::SPACE_2)
                .into(),
                "Phone" => text(d.get("Phone")).size(typography::body()).into(),
                "Email" => text(d.get("Email")).size(typography::body()).into(),
                _ => {
                    let tone = if d.status == "Active" {
                        badges::BadgeTone::Positive
                    } else {
                        badges::BadgeTone::Neutral
                    };
                    let (bg, fg) = badges::tone_colors(t, tone);
                    container(
                        text(master_status_label(&d.status))
                            .size(typography::caption())
                            .color(fg),
                    )
                    .padding([spacing::SPACE_2, spacing::SPACE_10])
                    .style(move |_iced_theme: &iced::Theme| container::Style {
                        background: Some(Background::Color(bg)),
                        border: iced::Border {
                            radius: states::RADIUS_PILL.into(),
                            ..iced::Border::default()
                        },
                        ..container::Style::default()
                    })
                    .into()
                }
            };
            cells = cells.push(container(content).width(Length::FillPortion(portion(label))));
        }
        list = list.push(
            button(cells)
                .padding([spacing::SPACE_10, spacing::SPACE_12])
                .width(Length::Fill)
                .style(table_style::row(
                    t,
                    editing_id.is_some() && editing_id == d.id,
                ))
                .on_press(Message::Edit(id)),
        );
    }
    if s.records.is_empty() {
        list = list.push(
            container(text("No records yet. Select Add to get started."))
                .padding(spacing::SPACE_30),
        );
    }
    container(list)
        .width(Length::Fill)
        .style(panels::card(t, states::RADIUS_LG))
        .into()
}

fn master_columns<'a>(s: &'a State, t: Theme) -> Element<'a, Message> {
    let mut list = column![text("Select Columns to Display")
        .size(typography::panel_title())
        .color(t.text_primary())]
    .spacing(spacing::SPACE_10);
    for label in MASTER_COLUMNS {
        list = list.push(
            checkbox(label, !s.hidden.contains(label))
                .on_toggle(move |v| Message::Column(label.to_string(), v))
                .style(checkbox_style::style(t))
                .size(typography::checkbox_size())
                .text_size(typography::label())
                .width(Length::Fill),
        );
    }
    list.into()
}

/// A vertical scrollbar with its own gutter (embedded mode): the content
/// is narrowed instead of the bar floating over it.
fn master_scrollbar() -> scrollable::Scrollbar {
    scrollable::Scrollbar::new().spacing(spacing::SPACE_8)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MasterTab {
    Details,
    Payment,
    Notes,
    CustomFields,
    Attachments,
}

impl MasterTab {
    fn label(self) -> &'static str {
        match self {
            MasterTab::Details => "Details",
            MasterTab::Payment => "Payment",
            MasterTab::Notes => "Notes",
            MasterTab::CustomFields => "Custom Fields",
            MasterTab::Attachments => "Attachments",
        }
    }
}

fn master_keys(kind: Kind, wanted: &[&'static str]) -> Vec<&'static str> {
    let all = kind.headers();
    wanted.iter().copied().filter(|k| all.contains(k)).collect()
}

/// Name, company, phone, email -- the "Contact Info" section.
fn master_contact_keys(kind: Kind) -> Vec<&'static str> {
    master_keys(
        kind,
        &[master_name_key(kind), "Company Name", "Phone", "Email"],
    )
}

fn master_address_keys(kind: Kind) -> Vec<&'static str> {
    master_keys(kind, &["Mailing Address", "Shipping Address"])
}

/// The fields shown as cards under one tab. Details also takes every
/// field no other section claims, so nothing from `Kind::headers()` is
/// ever left out.
fn master_tab_keys(kind: Kind, tab: MasterTab) -> Vec<&'static str> {
    match tab {
        MasterTab::Payment => master_keys(kind, &["Payment Terms", "Payment Method"]),
        MasterTab::Notes => master_keys(kind, &["Notes/Memo", "Description"]),
        MasterTab::Details => {
            let claimed: Vec<&str> = master_contact_keys(kind)
                .into_iter()
                .chain(master_address_keys(kind))
                .chain(master_tab_keys(kind, MasterTab::Payment))
                .chain(master_tab_keys(kind, MasterTab::Notes))
                .collect();
            kind.headers()
                .into_iter()
                .chain(kind.footers())
                .filter(|k| !claimed.contains(k))
                .collect()
        }
        MasterTab::CustomFields | MasterTab::Attachments => vec![],
    }
}

/// Only the tabs that make sense for the kind: Payment only where the
/// kind has payment fields, Custom Fields only for vendors.
fn master_tabs(kind: Kind) -> Vec<MasterTab> {
    let mut tabs = vec![MasterTab::Details];
    if !master_tab_keys(kind, MasterTab::Payment).is_empty() {
        tabs.push(MasterTab::Payment);
    }
    if !master_tab_keys(kind, MasterTab::Notes).is_empty() {
        tabs.push(MasterTab::Notes);
    }
    if kind == Kind::Vendor {
        tabs.push(MasterTab::CustomFields);
    }
    tabs.push(MasterTab::Attachments);
    tabs
}

/// What the detail panel's action groups contain, in display order:
/// (group, label, message). The panel renders its buttons from this
/// list, so a test can check that none is dropped.
fn master_form_actions(s: &State, d: &Document) -> Vec<(&'static str, String, Message)> {
    let mut actions = vec![
        ("files", "Attachments".to_string(), Message::Attach),
        (
            "files",
            format!("Preview files ({})", d.attachments.len()),
            Message::Attachments,
        ),
        ("files", "Preview".to_string(), Message::Preview),
        ("files", "Print PDF".to_string(), Message::Print),
        (
            "template",
            "Save as Template".to_string(),
            Message::SaveTemplate,
        ),
    ];
    if s.master_edit {
        if d.kind == Kind::Vendor && !d.locked() {
            actions.push((
                "fields",
                "Add Custom Field".to_string(),
                Message::CustomField,
            ));
        }
        if !d.locked() {
            actions.push(("footer", "Save".to_string(), Message::Save));
        }
        actions.push(("footer", "Cancel".to_string(), Message::Close));
    } else if !d.locked() {
        actions.push(("footer", "Edit".to_string(), Message::MasterEdit(true)));
    }
    actions
}

fn master_glyph(key: &str) -> &'static str {
    match key {
        "Customer Name" | "Vendor Name" => icons::FIELD_NAME,
        "Company Name" => icons::FIELD_COMPANY,
        "Phone" => icons::FIELD_PHONE,
        "Email" => icons::FIELD_EMAIL,
        "Website" => icons::FIELD_WEBSITE,
        "Mailing Address" | "Shipping Address" => icons::FIELD_ADDRESS,
        "NTN" | "STRN" => "#",
        "Payment Terms" | "Payment Method" => icons::FIELD_PAYMENT,
        "Notes/Memo" | "Description" => icons::FIELD_NOTE,
        "Status" => icons::FIELD_STATUS,
        _ => icons::FIELD_OTHER,
    }
}

/// Two items per row, equal widths; an odd last item leaves the right
/// column empty.
fn master_grid<'a>(items: Vec<Element<'a, Message>>) -> Element<'a, Message> {
    let mut grid = column![].spacing(spacing::SPACE_10);
    let mut items = items.into_iter();
    while let Some(first) = items.next() {
        let second = items
            .next()
            .unwrap_or_else(|| Space::with_width(Length::Fill).into());
        grid = grid.push(
            row![
                container(first).width(Length::FillPortion(1)),
                container(second).width(Length::FillPortion(1)),
            ]
            .spacing(spacing::SPACE_10),
        );
    }
    grid.into()
}

fn master_section_title<'a>(t: Theme, title: &'a str) -> Element<'a, Message> {
    text(title)
        .size(typography::emphasis())
        .font(fonts::BOLD)
        .color(t.neutral_font())
        .into()
}

/// The value of `key` as plain text (view mode) or its input (edit mode).
fn master_value<'a>(s: &'a State, d: &'a Document, t: Theme, key: &str) -> Element<'a, Message> {
    if s.master_edit && !d.locked() {
        field_control(s, d, -1, key, t)
    } else {
        let value = d.get(key);
        text(if value.is_empty() { "--" } else { value })
            .size(typography::body())
            .color(t.neutral_font())
            .into()
    }
}

/// A label with a small glyph in front, the value below.
fn master_pair<'a>(s: &'a State, d: &'a Document, t: Theme, key: &'a str) -> Element<'a, Message> {
    column![
        row![
            text(master_glyph(key))
                .size(typography::caption())
                .color(t.neutral_front()),
            text(key)
                .size(typography::caption())
                .color(t.neutral_front()),
        ]
        .spacing(spacing::SPACE_6),
        master_value(s, d, t, key),
    ]
    .spacing(spacing::SPACE_4)
    .into()
}

/// A small white card: icon box on the left, muted label above `value`.
fn master_card<'a>(
    t: Theme,
    glyph: &'static str,
    label: impl Into<String>,
    value: Element<'a, Message>,
) -> Element<'a, Message> {
    let label: String = label.into();
    let icon = container(text(glyph).size(typography::label()).color(t.primary()))
        .center_x(Length::Fixed(28.0))
        .center_y(Length::Fixed(28.0))
        .style(move |_iced_theme: &iced::Theme| container::Style {
            background: Some(Background::Color(t.neutral_bg_hover())),
            border: iced::Border {
                color: t.neutral_bg(),
                width: states::STYLES_BORDER_WIDTH_ALPHA,
                radius: states::RADIUS_SM.into(),
            },
            ..container::Style::default()
        });
    container(
        row![
            icon,
            column![
                text(label)
                    .size(typography::caption())
                    .color(t.neutral_front()),
                value
            ]
            .spacing(spacing::SPACE_2)
            .width(Length::Fill)
        ]
        .spacing(spacing::SPACE_10)
        .align_y(Alignment::Center),
    )
    .padding(spacing::SPACE_10)
    .width(Length::Fill)
    .style(panels::flat_card(t, 8.))
    .into()
}

fn master_field_card<'a>(
    s: &'a State,
    d: &'a Document,
    t: Theme,
    key: &'a str,
) -> Element<'a, Message> {
    master_card(t, master_glyph(key), key, master_value(s, d, t, key))
}

fn master_status<'a>(s: &'a State, d: &'a Document, t: Theme) -> Element<'a, Message> {
    let value: Element<'a, Message> = if s.master_edit && !d.locked() {
        pick_list(
            d.kind
                .statuses()
                .iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>(),
            Some(d.status.clone()),
            Message::Status,
        )
        .style(dropdown::style(t))
        .padding(spacing::SPACE_8)
        .width(Length::Fill)
        .into()
    } else if d.locked() {
        text(format!("{} · Read only", d.status))
            .size(typography::body())
            .color(t.neutral_font())
            .into()
    } else {
        text(master_status_label(&d.status).to_string())
            .size(typography::body())
            .color(t.neutral_font())
            .into()
    };
    master_card(t, master_glyph("Status"), "Status", value)
}

fn master_initials(name: &str) -> String {
    let initials: String = name
        .split_whitespace()
        .filter_map(|word| word.chars().next())
        .take(2)
        .flat_map(char::to_uppercase)
        .collect();
    if initials.is_empty() {
        "?".to_string()
    } else {
        initials
    }
}

/// Saved records only: avatar + name/company, a divider, phone and email.
fn master_summary<'a>(d: &'a Document, t: Theme) -> Element<'a, Message> {
    let name = d.get(master_name_key(d.kind));
    let muted = |value: String| {
        text(value)
            .size(typography::caption())
            .color(t.neutral_front())
    };
    let dash = |value: &'a str| {
        if value.is_empty() {
            "--".to_string()
        } else {
            value.to_string()
        }
    };
    let avatar = container(
        text(master_initials(name))
            .size(typography::emphasis())
            .font(fonts::BOLD)
            .color(t.primary()),
    )
    .center_x(Length::Fixed(44.0))
    .center_y(Length::Fixed(44.0))
    .style(move |_iced_theme: &iced::Theme| container::Style {
        background: Some(Background::Color(t.primary_tint(states::SELECTED_ALPHA))),
        border: iced::Border {
            radius: 22.0.into(),
            ..iced::Border::default()
        },
        ..container::Style::default()
    });
    let divider = container(Space::with_width(Length::Fixed(1.0)))
        .height(Length::Fixed(40.0))
        .style(move |_iced_theme: &iced::Theme| container::Style {
            background: Some(Background::Color(t.neutral_border())),
            ..container::Style::default()
        });
    container(
        row![
            avatar,
            column![
                text(name)
                    .size(typography::emphasis())
                    .font(fonts::BOLD)
                    .color(t.neutral_font()),
                muted(dash(d.get("Company Name"))),
            ]
            .spacing(spacing::SPACE_2)
            .width(Length::FillPortion(1)),
            divider,
            column![
                muted(format!(
                    "{}  {}",
                    master_glyph("Phone"),
                    dash(d.get("Phone"))
                )),
                muted(format!(
                    "{}  {}",
                    master_glyph("Email"),
                    dash(d.get("Email"))
                )),
            ]
            .spacing(spacing::SPACE_4)
            .width(Length::FillPortion(1)),
        ]
        .spacing(spacing::SPACE_12)
        .align_y(Alignment::Center),
    )
    .padding(spacing::SPACE_12)
    .width(Length::Fill)
    .style(move |_iced_theme: &iced::Theme| container::Style {
        background: Some(Background::Color(
            t.primary_tint(states::TINTED_PANEL_ALPHA),
        )),
        border: borders::card(t, states::RADIUS_MD),
        ..container::Style::default()
    })
    .into()
}

/// The scrolling content under the tabs.
fn master_tab_body<'a>(
    s: &'a State,
    d: &'a Document,
    t: Theme,
    tab: MasterTab,
) -> Element<'a, Message> {
    let mut body = column![].spacing(spacing::MD);
    match tab {
        MasterTab::Details => {
            body = body.push(master_section_title(t, "Contact Info"));
            body = body.push(master_grid(
                master_contact_keys(d.kind)
                    .into_iter()
                    .map(|k| master_pair(s, d, t, k))
                    .collect(),
            ));
            body = body.push(master_section_title(t, "Addresses"));
            body = body.push(master_grid(
                master_address_keys(d.kind)
                    .into_iter()
                    .map(|k| master_pair(s, d, t, k))
                    .collect(),
            ));
            let mut cards: Vec<Element<'a, Message>> = master_tab_keys(d.kind, tab)
                .into_iter()
                .map(|k| master_field_card(s, d, t, k))
                .collect();
            cards.push(master_status(s, d, t));
            body = body.push(master_grid(cards));
        }
        MasterTab::Payment | MasterTab::Notes => {
            body = body.push(master_grid(
                master_tab_keys(d.kind, tab)
                    .into_iter()
                    .map(|k| master_field_card(s, d, t, k))
                    .collect(),
            ));
        }
        MasterTab::CustomFields => {
            let keys: Vec<&String> = d
                .fields
                .keys()
                .filter(|k| k.starts_with("Custom ") || k.starts_with("Stock at "))
                .collect();
            if keys.is_empty() {
                body = body.push(
                    text("No custom fields.")
                        .size(typography::caption())
                        .color(t.neutral_front()),
                );
            }
            for k in keys {
                let value = master_value(s, d, t, k);
                let value: Element<'a, Message> = if s.master_edit && !d.locked() {
                    row![value, action("Remove", Message::RemoveCustom(k.clone()), t)]
                        .spacing(spacing::SPACE_8)
                        .align_y(Alignment::Center)
                        .into()
                } else {
                    value
                };
                body = body.push(master_card(t, master_glyph(k), k, value));
            }
        }
        MasterTab::Attachments => {
            body = body.push(
                text(format!("{} file(s) attached", d.attachments.len()))
                    .size(typography::body())
                    .color(t.neutral_font()),
            );
            for (i, path) in d.attachments.iter().enumerate() {
                let name = std::path::Path::new(path)
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.clone());
                let mut line = row![text(name).size(typography::body()).width(Length::Fill)]
                    .spacing(spacing::SPACE_8)
                    .align_y(Alignment::Center);
                if s.master_edit && !d.locked() {
                    line = line.push(action("Remove", Message::RemoveAttachment(i), t));
                }
                body = body.push(
                    container(line)
                        .padding(spacing::SPACE_10)
                        .width(Length::Fill)
                        .style(panels::flat_card(t, 8.)),
                );
            }
        }
    }
    body.into()
}

fn master_form<'a>(s: &'a State, d: &'a Document, t: Theme) -> Element<'a, Message> {
    let title = if d.kind == Kind::Customer {
        "Customer Details"
    } else {
        "Vendor Details"
    };
    let actions = master_form_actions(s, d);
    let button_for = |label: &str| -> Element<'a, Message> {
        let (_, text_label, message) = actions
            .iter()
            .find(|(_, l, _)| l == label || (label == "Preview files" && l.starts_with(label)))
            .cloned()
            .expect("action exists");
        let primary = text_label == "Save" || text_label == "Edit";
        let b = button(text(text_label).size(typography::label()).font(fonts::BOLD))
            .padding([spacing::SM, spacing::MD])
            .on_press(message);
        if primary {
            b.style(primary_button(t)).into()
        } else {
            b.style(outlined_button(t, false)).into()
        }
    };
    let has = |label: &str| actions.iter().any(|(_, l, _)| l == label);

    // Fixed at the top: header, number, summary card and the tabs.
    let mut top = column![
        row![
            text(title)
                .size(typography::panel_title())
                .font(fonts::BOLD)
                .color(t.neutral_font()),
            Space::with_width(Length::Fill),
            button(text(GLYPH_CLOSE).size(typography::label()))
                .padding([spacing::SPACE_4, spacing::SPACE_8])
                .style(buttons::plain(t))
                .on_press(Message::Close),
        ]
        .align_y(Alignment::Center),
        text(if d.number.is_empty() {
            "Number assigned on save"
        } else {
            &d.number
        })
        .size(typography::caption())
        .color(t.neutral_front()),
    ]
    .spacing(spacing::SPACE_10);
    if d.id.is_some() {
        top = top.push(master_summary(d, t));
    }
    let tab_list = master_tabs(d.kind);
    let active = s.master_tab.min(tab_list.len() - 1);
    let mut tab_row = row![].spacing(spacing::MD);
    for (index, tab) in tab_list.iter().enumerate() {
        tab_row = tab_row.push(master_tab(
            t,
            tab.label(),
            None,
            index == active,
            Message::MasterTab(index),
        ));
    }
    top = top.push(tab_row);

    // Only this part scrolls; the scrollbar has its own gutter.
    let mut body = column![master_tab_body(s, d, t, tab_list[active])].spacing(spacing::MD);
    if has("Add Custom Field") && tab_list[active] == MasterTab::CustomFields {
        body = body.push(button_for("Add Custom Field"));
    }
    if s.show_attachments && tab_list[active] == MasterTab::Attachments {
        for (i, path) in d.attachments.iter().enumerate() {
            body = body.push(row![
                text(path).width(Length::Fill),
                action("Remove", Message::RemoveAttachment(i), t)
            ]);
        }
    }
    let scroll = scrollable(body.width(Length::Fill))
        .direction(scrollable::Direction::Vertical(master_scrollbar()))
        .height(Length::Fill);

    // Pinned at the bottom, grouped: files, template, then Save/Cancel/Edit.
    let mut footer = row![].spacing(spacing::SM);
    for label in ["Save", "Edit", "Cancel"] {
        if has(label) {
            footer = footer.push(button_for(label));
        }
    }
    let bottom = column![
        row![
            button_for("Attachments"),
            button_for("Preview files"),
            button_for("Preview"),
            button_for("Print PDF"),
        ]
        .spacing(spacing::SPACE_8)
        .wrap(),
        row![
            text_input("Template name", &s.template_name)
                .on_input(Message::TemplateName)
                .style(inputs::text(t))
                .padding(spacing::SPACE_10)
                .width(Length::Fill),
            button_for("Save as Template"),
            pick_list(s.templates.clone(), None::<String>, Message::LoadTemplate)
                .placeholder("Load Template")
                .style(dropdown::style(t))
                .padding(spacing::SPACE_10),
        ]
        .spacing(spacing::SPACE_8)
        .align_y(Alignment::Center),
        footer,
    ]
    .spacing(spacing::SPACE_10);

    column![top, scroll, bottom]
        .spacing(spacing::MD)
        .height(Length::Fill)
        .into()
}

/// Value text for the preview: `-` when empty.
fn preview_value(value: &str) -> &str {
    if value.is_empty() {
        "-"
    } else {
        value
    }
}

/// The preview's left-hand sections: (title, glyph, field names).
/// Built from the same key lists the detail panel uses, so every
/// `Kind::headers()` field lands in exactly one of them.
fn master_preview_sections(kind: Kind) -> Vec<(&'static str, &'static str, Vec<&'static str>)> {
    let details: Vec<&'static str> = master_tab_keys(kind, MasterTab::Details)
        .into_iter()
        .chain(master_tab_keys(kind, MasterTab::Payment))
        .chain(master_tab_keys(kind, MasterTab::Notes))
        .collect();
    vec![
        ("Contact Info", icons::FIELD_NAME, master_contact_keys(kind)),
        ("Addresses", icons::FIELD_ADDRESS, master_address_keys(kind)),
        ("Details", icons::FIELD_OTHER, details),
    ]
}

/// The two fields in the header band's white summary card.
fn master_preview_summary_keys(kind: Kind) -> [&'static str; 2] {
    if kind == Kind::Customer {
        ["NTN", "STRN"]
    } else {
        ["Payment Terms", "Payment Method"]
    }
}

/// One (name, value) per custom field -- `Custom n name` / `Custom n value`.
fn master_custom_fields(d: &Document) -> Vec<(String, String)> {
    d.fields
        .keys()
        .filter_map(|k| {
            let n = k.strip_prefix("Custom ")?.strip_suffix(" name")?;
            let name = d.get(k);
            let value = d.get(&format!("Custom {n} value"));
            Some((
                if name.is_empty() {
                    format!("Custom {n}")
                } else {
                    name.to_string()
                },
                value.to_string(),
            ))
        })
        .collect()
}

fn master_file_size(path: &str) -> Option<String> {
    let bytes = std::fs::metadata(path).ok()?.len();
    Some(if bytes >= 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else if bytes >= 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{bytes} B")
    })
}

/// Section header: small primary-tinted icon square, bold primary title
/// and a count badge.
fn preview_section_header<'a>(
    t: Theme,
    glyph: &'static str,
    title: &'a str,
    count: usize,
) -> Element<'a, Message> {
    let icon = container(text(glyph).size(typography::caption()).color(t.primary()))
        .center_x(Length::Fixed(22.0))
        .center_y(Length::Fixed(22.0))
        .style(move |_iced_theme: &iced::Theme| container::Style {
            background: Some(Background::Color(t.primary_tint(states::SELECTED_ALPHA))),
            border: iced::Border {
                radius: states::RADIUS_SM.into(),
                ..iced::Border::default()
            },
            ..container::Style::default()
        });
    let badge = container(
        text(count.to_string())
            .size(typography::caption())
            .color(t.neutral_front()),
    )
    .padding([spacing::SPACE_2, spacing::SPACE_8])
    .style(move |_iced_theme: &iced::Theme| container::Style {
        background: Some(Background::Color(t.neutral_bg())),
        border: iced::Border {
            radius: states::RADIUS_PILL.into(),
            ..iced::Border::default()
        },
        ..container::Style::default()
    });
    row![
        icon,
        text(title)
            .size(typography::emphasis())
            .font(fonts::BOLD)
            .color(t.primary()),
        badge
    ]
    .spacing(spacing::SPACE_8)
    .align_y(Alignment::Center)
    .into()
}

/// A small outlined pill such as "Attachments 2".
fn preview_pill<'a>(t: Theme, label: &'a str, count: usize) -> Element<'a, Message> {
    container(
        text(format!("{label} {count}"))
            .size(typography::caption())
            .color(t.neutral_front()),
    )
    .padding([spacing::SPACE_4, spacing::SPACE_10])
    .style(move |_iced_theme: &iced::Theme| container::Style {
        background: Some(Background::Color(t.surface())),
        border: borders::card(t, states::RADIUS_SM),
        ..container::Style::default()
    })
    .into()
}

fn preview_text<'a>(value: &'a str, t: Theme) -> Element<'a, Message> {
    text(preview_value(value))
        .size(typography::body())
        .color(t.neutral_font())
        .into()
}

/// The read-only record preview, shown inline in the page area.
fn master_preview<'a>(d: &'a Document, t: Theme) -> Element<'a, Message> {
    let name = d.get(master_name_key(d.kind));
    let muted = |value: String| {
        text(value)
            .size(typography::caption())
            .color(t.neutral_front())
    };
    let customs = master_custom_fields(d);

    // Header band: avatar, name + status pill, identity lines, count
    // pills, and the white summary card on the right.
    let avatar = container(
        text(master_initials(name))
            .size(typography::subtitle())
            .font(fonts::BOLD)
            .color(t.primary()),
    )
    .center_x(Length::Fixed(64.0))
    .center_y(Length::Fixed(64.0))
    .style(move |_iced_theme: &iced::Theme| container::Style {
        background: Some(Background::Color(t.primary_tint(states::SELECTED_ALPHA))),
        border: iced::Border {
            radius: 32.0.into(),
            ..iced::Border::default()
        },
        ..container::Style::default()
    });
    let tone = if d.status == "Active" {
        badges::BadgeTone::Positive
    } else {
        badges::BadgeTone::Neutral
    };
    let (pill_bg, pill_fg) = badges::tone_colors(t, tone);
    let status_pill = container(
        text(master_status_label(&d.status).to_string())
            .size(typography::caption())
            .color(pill_fg),
    )
    .padding([spacing::SPACE_2, spacing::SPACE_10])
    .style(move |_iced_theme: &iced::Theme| container::Style {
        background: Some(Background::Color(pill_bg)),
        border: iced::Border {
            radius: states::RADIUS_PILL.into(),
            ..iced::Border::default()
        },
        ..container::Style::default()
    });
    let number = if d.number.is_empty() { "-" } else { &d.number };
    let identity = column![
        row![
            text(preview_value(name))
                .size(typography::subtitle())
                .font(fonts::BOLD)
                .color(t.neutral_font()),
            status_pill
        ]
        .spacing(spacing::SPACE_10)
        .align_y(Alignment::Center),
        muted(format!(
            "{number}  ·  {}",
            preview_value(d.get("Company Name"))
        )),
        row![
            muted(format!(
                "{}  {}",
                icons::FIELD_PHONE,
                preview_value(d.get("Phone"))
            )),
            muted(format!(
                "{}  {}",
                icons::FIELD_EMAIL,
                preview_value(d.get("Email"))
            )),
        ]
        .spacing(spacing::LG),
        muted(format!(
            "{}  {}",
            icons::FIELD_ADDRESS,
            preview_value(d.get("Mailing Address"))
        )),
        row![
            preview_pill(t, "Attachments", d.attachments.len()),
            preview_pill(t, "Custom Fields", customs.len()),
        ]
        .spacing(spacing::SPACE_8),
    ]
    .spacing(spacing::SPACE_6)
    .width(Length::Fill);
    let mut summary = row![].spacing(spacing::LG);
    for key in master_preview_summary_keys(d.kind) {
        summary = summary.push(
            column![
                text(key)
                    .size(typography::caption())
                    .color(t.neutral_front()),
                preview_text(d.get(key), t),
            ]
            .spacing(spacing::SPACE_2),
        );
    }
    let band = container(
        row![
            avatar,
            identity,
            container(summary).padding(spacing::MD).style(card_style(t)),
        ]
        .spacing(spacing::MD)
        .align_y(Alignment::Center),
    )
    .padding(spacing::MD)
    .width(Length::Fill)
    .style(move |_iced_theme: &iced::Theme| container::Style {
        background: Some(Background::Color(
            t.primary_tint(states::TINTED_PANEL_ALPHA),
        )),
        border: borders::card(t, states::RADIUS_LG),
        ..container::Style::default()
    });

    let overview = text("Overview")
        .size(typography::panel_title())
        .font(fonts::BOLD)
        .color(t.neutral_font());

    // Fixed toolbar: Back on the left, Edit and Print PDF on the right.
    let toolbar = row![
        button(
            row![
                text(icons::ARROW_LEFT).size(typography::label()),
                text("Back").size(typography::label()).font(fonts::BOLD),
            ]
            .spacing(spacing::SPACE_6)
            .align_y(Alignment::Center)
        )
        .padding([spacing::SM, spacing::MD])
        .style(outlined_button(t, false))
        .on_press(Message::Preview),
        Space::with_width(Length::Fill),
        button(text("Edit").size(typography::label()).font(fonts::BOLD))
            .padding([spacing::SM, spacing::MD])
            .style(primary_button(t))
            .on_press(Message::PreviewEdit),
        button(
            text("Print PDF")
                .size(typography::label())
                .font(fonts::BOLD)
        )
        .padding([spacing::SM, spacing::MD])
        .style(outlined_button(t, false))
        .on_press(Message::Print),
    ]
    .spacing(spacing::SPACE_8)
    .align_y(Alignment::Center);

    // Left: record sections as card grids.
    let mut left = column![].spacing(spacing::MD);
    for (title, glyph, keys) in master_preview_sections(d.kind) {
        left = left.push(preview_section_header(t, glyph, title, keys.len()));
        left = left.push(master_grid(
            keys.into_iter()
                .map(|key| master_card(t, master_glyph(key), key, preview_text(d.get(key), t)))
                .collect(),
        ));
    }

    // Right: custom fields and attachments.
    let mut right = column![preview_section_header(
        t,
        icons::FIELD_NOTE,
        "Custom Fields",
        customs.len()
    )]
    .spacing(spacing::SPACE_10);
    if customs.is_empty() {
        right = right.push(muted("No custom fields".to_string()));
    }
    for (label, value) in customs {
        right = right.push(master_card(
            t,
            icons::FIELD_NOTE,
            label,
            text(preview_value(&value).to_string())
                .size(typography::body())
                .color(t.neutral_font())
                .into(),
        ));
    }
    right = right.push(preview_section_header(
        t,
        icons::FIELD_OTHER,
        "Attachments",
        d.attachments.len(),
    ));
    if d.attachments.is_empty() {
        right = right.push(muted("No attachments".to_string()));
    }
    for path in &d.attachments {
        let file = std::path::Path::new(path)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.clone());
        let size = master_file_size(path).unwrap_or_default();
        right = right.push(master_card(
            t,
            icons::FIELD_OTHER,
            file,
            text(preview_value(&size).to_string())
                .size(typography::body())
                .color(t.neutral_font())
                .into(),
        ));
    }

    let content = column![
        band,
        overview,
        row![
            container(left).width(Length::FillPortion(2)),
            container(right).width(Length::FillPortion(1)),
        ]
        .spacing(spacing::LG),
    ]
    .spacing(spacing::MD)
    .width(Length::Fill);

    // Only the content scrolls, with its own gutter for the scrollbar.
    column![
        toolbar,
        scrollable(content)
            .direction(scrollable::Direction::Vertical(master_scrollbar()))
            .height(Length::Fill),
    ]
    .spacing(spacing::MD)
    .height(Length::Fill)
    .into()
}

#[cfg(test)]
mod master_tests {
    use super::*;

    /// Names of every field the detail panel shows for `d`.
    fn master_form_fields(d: &Document) -> Vec<&'static str> {
        let kind = d.kind;
        master_contact_keys(kind)
            .into_iter()
            .chain(master_address_keys(kind))
            .chain(master_tab_keys(kind, MasterTab::Details))
            .chain(master_tab_keys(kind, MasterTab::Payment))
            .chain(master_tab_keys(kind, MasterTab::Notes))
            .filter(|k| !hidden_field(d, k))
            .collect()
    }

    fn form_for(kind: Kind) -> Document {
        Document::new(1, kind)
    }

    fn state_with(kind: Kind, form: Option<Document>) -> State {
        State {
            kind,
            form,
            ..State::default()
        }
    }

    #[test]
    fn customer_and_vendor_views_build_in_every_panel_state_and_both_themes() {
        for kind in [Kind::Customer, Kind::Vendor] {
            for theme in [Theme::Light, Theme::Dark] {
                // Panel closed.
                let closed = state_with(kind, None);
                let _: Element<'_, Message> = view(&closed, theme, MASTERS);

                // Columns panel alone.
                let mut columns = state_with(kind, None);
                columns.columns = true;
                let _: Element<'_, Message> = view(&columns, theme, MASTERS);

                let mut saved = form_for(kind);
                saved.id = Some(1);
                saved.number = "X-1".into();
                saved.set("Custom 1 name", "a");
                saved.set("Custom 1 value", "b");
                saved.attachments.push("/tmp/file.pdf".into());

                // New record (edit mode) and saved record in view and
                // edit mode, on every tab, with the extras toggled on.
                for (form, edit) in [
                    (form_for(kind), true),
                    (saved.clone(), false),
                    (saved.clone(), true),
                ] {
                    for tab in 0..master_tabs(kind).len() + 1 {
                        let mut open = state_with(kind, Some(form.clone()));
                        open.master_edit = edit;
                        open.master_tab = tab;
                        let _: Element<'_, Message> = view(&open, theme, MASTERS);

                        open.columns = true;
                        open.show_attachments = true;
                        open.show_preview = true;
                        open.selected.insert(1);
                        let _: Element<'_, Message> = view(&open, theme, MASTERS);
                    }
                }
            }
        }
    }

    fn preview_state(kind: Kind, form: Document) -> State {
        let mut s = state_with(kind, Some(form));
        s.show_preview = true;
        s
    }

    #[test]
    fn preview_builds_for_saved_new_and_rich_records_in_both_themes() {
        for kind in [Kind::Customer, Kind::Vendor] {
            let mut saved = form_for(kind);
            saved.id = Some(1);
            saved.number = "X-1".into();
            saved.set(master_name_key(kind), "harbor supply");
            saved.set("Phone", "5550100");

            let mut rich = saved.clone();
            rich.status = "Inactive".into();
            rich.set("Custom 1 name", "Region");
            rich.set("Custom 1 value", "North");
            rich.set("Custom 2 name", "");
            rich.attachments.push("/tmp/definitely-missing.pdf".into());

            for theme in [Theme::Light, Theme::Dark] {
                for record in [form_for(kind), saved.clone(), rich.clone()] {
                    let s = preview_state(kind, record);
                    let _: Element<'_, Message> = view(&s, theme, MASTERS);
                }
            }
        }
    }

    fn test_ctx() -> Ctx {
        Ctx {
            db: crate::database::database::Database::open_in_memory().unwrap(),
            company_id: 1,
            current_user_id: None,
            export_dir: std::path::PathBuf::from("."),
            uploads_dir: std::path::PathBuf::from("."),
        }
    }

    #[test]
    fn toggling_preview_on_and_off_restores_the_previous_state() {
        let ctx = test_ctx();
        let mut form = form_for(Kind::Vendor);
        form.set("Vendor Name", "Unsaved Name");
        form.set("Phone", "123");
        let mut s = state_with(Kind::Vendor, Some(form));
        s.search = "harbor".into();
        s.status = "Active".into();
        s.page = 2;
        s.master_tab = 1;
        s.master_edit = true;
        s.columns = true;
        s.selected.insert(7);

        update(&mut s, Message::Preview, &ctx);
        assert!(s.show_preview);
        let _: Element<'_, Message> = view(&s, Theme::Light, MASTERS);

        update(&mut s, Message::Preview, &ctx);
        assert!(!s.show_preview);
        assert_eq!(s.search, "harbor");
        assert_eq!(s.status, "Active");
        assert_eq!(s.page, 2);
        assert_eq!(s.master_tab, 1);
        assert!(s.master_edit && s.columns);
        assert!(s.selected.contains(&7));
        let form = s.form.as_ref().unwrap();
        assert_eq!(form.get("Vendor Name"), "Unsaved Name");
        assert_eq!(form.get("Phone"), "123");
    }

    #[test]
    fn edit_from_the_preview_leaves_it_and_opens_edit_mode() {
        let ctx = test_ctx();
        let mut s = state_with(Kind::Customer, Some(form_for(Kind::Customer)));
        s.show_preview = true;
        update(&mut s, Message::PreviewEdit, &ctx);
        assert!(!s.show_preview && s.master_edit);
    }

    #[test]
    fn return_and_tab_changes_leave_the_preview() {
        let ctx = test_ctx();
        for message in [
            Message::Close,
            Message::MasterTab(0),
            Message::Filter("All".into()),
        ] {
            let mut s = state_with(Kind::Customer, Some(form_for(Kind::Customer)));
            s.show_preview = true;
            update(&mut s, message, &ctx);
            assert!(!s.show_preview);
        }
    }

    #[test]
    fn every_header_field_appears_in_the_preview() {
        for kind in [Kind::Customer, Kind::Vendor] {
            let shown: Vec<&str> = master_preview_sections(kind)
                .into_iter()
                .flat_map(|(_, _, keys)| keys)
                .collect();
            for key in kind.headers() {
                assert!(shown.contains(&key), "{kind}: preview misses {key}");
            }
            for key in master_preview_summary_keys(kind) {
                assert!(kind.headers().contains(&key), "{kind}: {key} not a header");
            }
        }
    }

    #[test]
    fn custom_fields_pair_up_name_and_value() {
        let mut d = form_for(Kind::Vendor);
        d.set("Custom 1 name", "Region");
        d.set("Custom 1 value", "North");
        d.set("Custom 2 name", "");
        assert_eq!(
            master_custom_fields(&d),
            vec![
                ("Region".to_string(), "North".to_string()),
                ("Custom 2".to_string(), String::new())
            ]
        );
    }

    #[test]
    fn tabs_fit_the_kind_and_every_header_lands_in_exactly_one_section() {
        assert_eq!(
            master_tabs(Kind::Vendor),
            vec![
                MasterTab::Details,
                MasterTab::Payment,
                MasterTab::Notes,
                MasterTab::CustomFields,
                MasterTab::Attachments
            ]
        );
        assert_eq!(
            master_tabs(Kind::Customer),
            vec![MasterTab::Details, MasterTab::Notes, MasterTab::Attachments]
        );
        for kind in [Kind::Customer, Kind::Vendor] {
            let mut shown = master_form_fields(&form_for(kind));
            let mut want = kind.headers();
            shown.sort_unstable();
            want.sort_unstable();
            assert_eq!(shown, want, "{kind}");
        }
    }

    #[test]
    fn the_panel_lists_every_field_the_kind_defines() {
        for kind in [Kind::Customer, Kind::Vendor] {
            let d = form_for(kind);
            let shown = master_form_fields(&d);
            for key in kind.headers() {
                assert!(shown.contains(&key), "{kind}: missing field {key}");
            }
        }
        let vendor = master_form_fields(&form_for(Kind::Vendor));
        for key in [
            "Vendor Name",
            "Company Name",
            "Phone",
            "Email",
            "Website",
            "Mailing Address",
            "Shipping Address",
            "Payment Terms",
            "Payment Method",
            "Notes/Memo",
        ] {
            assert!(vendor.contains(&key), "vendor missing {key}");
        }
        let customer = master_form_fields(&form_for(Kind::Customer));
        for key in [
            "Customer Name",
            "Company Name",
            "Phone",
            "Email",
            "NTN",
            "STRN",
        ] {
            assert!(customer.contains(&key), "customer missing {key}");
        }
    }

    #[test]
    fn the_panel_keeps_every_action_across_view_and_edit_mode() {
        for kind in [Kind::Customer, Kind::Vendor] {
            let d = form_for(kind);
            let mut labels: Vec<String> = Vec::new();
            for edit in [false, true] {
                let mut s = state_with(kind, Some(d.clone()));
                s.master_edit = edit;
                labels.extend(
                    master_form_actions(&s, &d)
                        .into_iter()
                        .map(|(_, label, _)| label),
                );
            }
            for want in [
                "Attachments",
                "Preview files (0)",
                "Preview",
                "Print PDF",
                "Save as Template",
                "Save",
                "Cancel",
                "Edit",
            ] {
                assert!(labels.iter().any(|l| l == want), "{kind}: missing {want}");
            }
            assert_eq!(
                labels.iter().any(|l| l == "Add Custom Field"),
                kind == Kind::Vendor
            );
        }
    }

    #[test]
    fn initials_use_the_first_letters_of_the_first_two_words() {
        assert_eq!(master_initials("harbor supply co"), "HS");
        assert_eq!(master_initials("Acme"), "A");
        assert_eq!(master_initials(""), "?");
    }
}
