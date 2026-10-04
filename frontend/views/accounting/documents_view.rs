//! Schema-driven native document editor and searchable document lists.
use crate::backend::core::errors::{AppError, AppResult};
use crate::backend::services::{
    accounting_service::AccountingService, document_service::DocumentService,
    inventory_service::InventoryService, party_service::PartyService,
};
use crate::backend::transactions::documents::*;
use crate::frontend::styles::{
    button as buttons, colors::Theme, dropdown, inputs, panels, spacing, typography,
};
use crate::frontend::views::Ctx;
use iced::widget::{
    button, checkbox, column, container, pick_list, row, scrollable, text, text_input, Space,
};
use iced::{Element, Length};
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
        Message::Add => {
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
                s.message = "Saved successfully".into();
            }
        }
        Message::Edit(id) => {
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
    column![text(key.to_string()).size(typography::label()), control]
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
