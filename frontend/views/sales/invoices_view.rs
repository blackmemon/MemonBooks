//! List invoices and create, edit, post, and pay a simple single-line
//! invoice. A Draft has no ledger impact yet, so it's freely editable
//! and deletable (bulk-select on the Draft tab); once Posted, its GL
//! impact is never rewritten in place -- corrections go through a
//! credit note instead (see the immutability proposal this file
//! implements).

use std::collections::HashSet;

use chrono::{Days, Utc};
use rust_decimal::Decimal;

use iced::widget::{button, column, container, pick_list, scrollable, text, text_input};
use iced::{Element, Length};

use crate::backend::accounting::chart_of_accounts::Account;
use crate::backend::sales::invoices::{Invoice, InvoiceLine, InvoiceStatus};
use crate::backend::services::sales_service::SalesService;
use crate::database::exports::csv;
use crate::frontend::components::common::data_table::{self, Cell, ColumnDef, Row, SortDirection};
use crate::frontend::components::common::{bulk_action_bar, page_header, section_heading, toolbar};
use crate::frontend::styles::badges::BadgeTone;
use crate::frontend::styles::colors::Theme;
use crate::frontend::styles::typography;
use crate::frontend::styles::dropdown;
use crate::frontend::styles::inputs;
use crate::frontend::styles::button as button_style;
use crate::frontend::views::Ctx;

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

#[derive(Debug, Clone, Default)]
pub struct State {
    invoices: Vec<Invoice>,
    customers: Vec<Choice>,
    accounts: Vec<Choice>,
    customer: Option<Choice>,
    /// Only needed by the "Create & post" fast path -- editing a draft,
    /// or posting one via its own row action, chooses the AR account
    /// separately (`post_ar_account`) at the point it's actually needed.
    ar_account: Option<Choice>,
    revenue_account: Option<Choice>,
    description_input: String,
    amount_input: String,
    search: String,
    /// Which status tab is active: 0 = Draft, 1 = Posted, 2 = Voided.
    /// Voided invoices stay out of the other two tabs (audit trail, not
    /// clutter) rather than being filtered out of the screen entirely.
    active_tab: usize,
    /// `Some(id)` while the form is editing an existing draft instead of
    /// creating a new one -- `Message::SaveDraft` does double duty as
    /// "save changes" in that state. Only ever reached via a Draft row's
    /// kebab menu; a Posted row's kebab opens the credit-note form
    /// instead, since a posted invoice is never edited in place.
    editing_id: Option<i64>,
    /// Bulk-select, Draft rows only -- the checkbox column and bulk
    /// action bar only render on the Draft tab, since only a Draft has
    /// no ledger impact yet and is safe to delete outright.
    selected: HashSet<i64>,
    /// The draft invoice a row's "Post" button was last clicked for --
    /// `None` until then, and again after a successful submission.
    post_invoice_id: Option<i64>,
    post_ar_account: Option<Choice>,
    /// The invoice a row's "Record Payment" button was last clicked for
    /// -- `None` until then, and again after a successful submission.
    payment_invoice_id: Option<i64>,
    payment_cash_account: Option<Choice>,
    payment_ar_account: Option<Choice>,
    payment_amount_input: String,
    /// The posted invoice a row's kebab menu was last used to open a
    /// credit note against -- `None` until then, and again after a
    /// successful submission.
    credit_note_invoice_id: Option<i64>,
    credit_note_revenue_account: Option<Choice>,
    credit_note_ar_account: Option<Choice>,
    credit_note_description_input: String,
    credit_note_amount_input: String,
    message: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    CustomerSelected(Choice),
    ArAccountSelected(Choice),
    RevenueAccountSelected(Choice),
    DescriptionChanged(String),
    AmountChanged(String),
    /// Creates a new Draft (no ledger impact), or -- while `editing_id`
    /// is set -- saves changes to the existing draft in place.
    SaveDraft,
    /// The fast path for a brand new invoice: create then immediately
    /// post, in one step. Never available while editing an existing
    /// draft -- that's `SaveDraft` (to keep it Draft) plus the row's own
    /// "Post" action (to post it once ready).
    CreateAndPost,
    CancelEdit,
    RowSelected(i64, bool),
    SelectAllChanged(Vec<i64>, bool),
    ClearSelection,
    BulkDelete,
    BulkExportCsv,
    SearchChanged(String),
    TabSelected(usize),
    FiltersClicked,
    SortClicked,
    RowAction(usize),
    /// Fired by a Draft row's "Post" button, carrying that invoice's id.
    PostForRow(i64),
    PostArAccountSelected(Choice),
    SubmitPost,
    /// Fired by a Posted invoice row's "Record Payment" button, carrying
    /// that invoice's id -- looks up its own remaining balance and
    /// prefills the amount field with it. `SubmitPayment` posts against
    /// exactly this invoice's receivable, capped at its own balance --
    /// other open invoices for the same customer are never touched.
    RecordPaymentForRow(i64),
    PaymentCashAccountSelected(Choice),
    PaymentArAccountSelected(Choice),
    PaymentAmountChanged(String),
    SubmitPayment,
    /// Fired by a row's kebab menu, carrying that invoice's id -- a
    /// Draft row opens the edit form, prefilled from its own line; a
    /// Posted row opens the credit-note form below, prefilled with its
    /// own remaining balance; a Voided row has no action.
    KebabForRow(i64),
    CreditNoteRevenueAccountSelected(Choice),
    CreditNoteArAccountSelected(Choice),
    CreditNoteDescriptionChanged(String),
    CreditNoteAmountChanged(String),
    SubmitCreditNote,
}

fn status_tone(status: &str) -> BadgeTone {
    match status {
        "posted" => BadgeTone::Positive,
        "voided" => BadgeTone::Negative,
        _ => BadgeTone::Neutral,
    }
}

pub fn load(ctx: &Ctx) -> State {
    let service = SalesService::new(ctx.db.clone());

    let customers = service
        .list_customers(ctx.company_id)
        .map(|customers| customers.iter().map(|c| Choice { id: c.id.unwrap_or_default(), label: c.name.clone() }).collect())
        .unwrap_or_default();
    let accounts: Vec<Choice> = service
        .list_gl_accounts(ctx.company_id)
        .map(|accounts: Vec<Account>| accounts.iter().map(|a| Choice { id: a.id.unwrap_or_default(), label: format!("{} - {}", a.code, a.name) }).collect())
        .unwrap_or_default();

    let invoices = service.list_invoices(ctx.company_id).unwrap_or_default();

    State {
        invoices,
        customers,
        accounts,
        ..Default::default()
    }
}

pub fn update(state: &mut State, message: Message, ctx: &Ctx) {
    match message {
        Message::CustomerSelected(choice) => state.customer = Some(choice),
        Message::ArAccountSelected(choice) => state.ar_account = Some(choice),
        Message::RevenueAccountSelected(choice) => state.revenue_account = Some(choice),
        Message::DescriptionChanged(value) => state.description_input = value,
        Message::AmountChanged(value) => state.amount_input = value,
        Message::SaveDraft => {
            state.message = None;
            state.error = None;

            let Some(customer) = &state.customer else {
                state.error = Some("choose a customer".into());
                return;
            };
            let Some(revenue_account) = &state.revenue_account else {
                state.error = Some("choose a revenue account".into());
                return;
            };
            let amount: Decimal = match state.amount_input.parse() {
                Ok(amount) => amount,
                Err(_) => {
                    state.error = Some("amount is not a valid number".into());
                    return;
                }
            };

            if let Some(id) = state.editing_id {
                let Some(existing) = state.invoices.iter().find(|i| i.id == Some(id)).cloned() else {
                    state.error = Some("that invoice no longer exists".into());
                    state.editing_id = None;
                    return;
                };
                let invoice = Invoice {
                    customer_id: customer.id,
                    lines: vec![InvoiceLine::new(revenue_account.id, state.description_input.clone(), Decimal::ONE, amount)],
                    ..existing
                };
                let service = SalesService::new(ctx.db.clone());
                match service.update_invoice(&invoice) {
                    Ok(()) => {
                        if let Some(slot) = state.invoices.iter_mut().find(|i| i.id == Some(id)) {
                            *slot = invoice.clone();
                        }
                        state.editing_id = None;
                        state.customer = None;
                        state.revenue_account = None;
                        state.description_input.clear();
                        state.amount_input.clear();
                        state.message = Some("Saved changes.".into());
                    }
                    Err(err) => state.error = Some(err.to_string()),
                }
                return;
            }

            let today = Utc::now().date_naive();
            let due_date = today.checked_add_days(Days::new(30)).unwrap_or(today);
            let mut invoice = Invoice::new(ctx.company_id, customer.id, today, due_date);
            invoice.lines.push(InvoiceLine::new(revenue_account.id, state.description_input.clone(), Decimal::ONE, amount));

            let service = SalesService::new(ctx.db.clone());
            match service.create_invoice(&invoice) {
                Ok(created) => {
                    state.invoices.push(created);
                    state.customer = None;
                    state.revenue_account = None;
                    state.description_input.clear();
                    state.amount_input.clear();
                    state.message = Some("Saved as draft.".into());
                }
                Err(err) => state.error = Some(err.to_string()),
            }
        }
        Message::CreateAndPost => {
            state.message = None;
            state.error = None;

            let (Some(customer), Some(ar_account), Some(revenue_account)) = (&state.customer, &state.ar_account, &state.revenue_account) else {
                state.error = Some("choose a customer, AR account, and revenue account".into());
                return;
            };
            let amount: Decimal = match state.amount_input.parse() {
                Ok(amount) => amount,
                Err(_) => {
                    state.error = Some("amount is not a valid number".into());
                    return;
                }
            };

            let today = Utc::now().date_naive();
            let due_date = today.checked_add_days(Days::new(30)).unwrap_or(today);
            let mut invoice = Invoice::new(ctx.company_id, customer.id, today, due_date);
            invoice.lines.push(InvoiceLine::new(revenue_account.id, state.description_input.clone(), Decimal::ONE, amount));

            let service = SalesService::new(ctx.db.clone());
            let created = match service.create_invoice(&invoice) {
                Ok(invoice) => invoice,
                Err(err) => {
                    state.error = Some(err.to_string());
                    return;
                }
            };
            match service.post_invoice(created.id.unwrap_or_default(), ar_account.id) {
                Ok((posted, _transaction, _receivable)) => {
                    state.message = Some(format!("Posted invoice #{}.", posted.id.unwrap_or_default()));
                    state.invoices.push(posted);
                    state.description_input.clear();
                    state.amount_input.clear();
                }
                Err(err) => state.error = Some(err.to_string()),
            }
        }
        Message::CancelEdit => {
            state.editing_id = None;
            state.customer = None;
            state.revenue_account = None;
            state.description_input.clear();
            state.amount_input.clear();
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
            let service = SalesService::new(ctx.db.clone());
            let mut deleted = 0;
            let mut failures = Vec::new();
            for id in state.selected.clone() {
                match service.delete_invoice(id) {
                    Ok(()) => {
                        deleted += 1;
                        state.invoices.retain(|i| i.id != Some(id));
                        state.selected.remove(&id);
                    }
                    Err(err) => failures.push(format!("invoice #{id}: {err}")),
                }
            }
            if !failures.is_empty() {
                state.error = Some(format!("Deleted {deleted}; {} could not be deleted -- {}", failures.len(), failures.join("; ")));
            } else if deleted > 0 {
                state.message = Some(format!("Deleted {deleted} draft invoice(s)."));
            }
        }
        Message::BulkExportCsv => {
            state.message = None;
            state.error = None;
            let selected: Vec<&Invoice> = state.invoices.iter().filter(|i| state.selected.contains(&i.id.unwrap_or_default())).collect();
            let headers = ["Invoice", "Date", "Status", "Total"];
            let rows: Vec<Vec<String>> = selected
                .iter()
                .map(|i| vec![i.id.map(|id| format!("INV-{id}")).unwrap_or_default(), i.invoice_date.to_string(), i.status.as_str().to_string(), i.total().to_string()])
                .collect();
            let path = ctx.export_path("invoices_export.csv");
            match std::fs::write(&path, csv::export_csv(&headers, &rows)) {
                Ok(()) => state.message = Some(format!("Exported {} invoice(s) to {}", selected.len(), path.display())),
                Err(err) => state.error = Some(err.to_string()),
            }
        }
        Message::SearchChanged(value) => state.search = value,
        Message::TabSelected(index) => state.active_tab = index,
        // "Filters" / "Sort by" are rendered by the shared toolbar
        // component now (Section 6) -- wiring real filter/sort behavior
        // is follow-up work, not part of this pass. RowAction is the
        // harmless stub the shared DataTable requires for its checkbox
        // callbacks on tabs where the checkbox column isn't shown.
        Message::FiltersClicked | Message::SortClicked | Message::RowAction(_) => {}
        Message::PostForRow(invoice_id) => {
            state.message = None;
            state.error = None;
            state.post_invoice_id = Some(invoice_id);
        }
        Message::PostArAccountSelected(choice) => state.post_ar_account = Some(choice),
        Message::SubmitPost => {
            state.message = None;
            state.error = None;

            let (Some(invoice_id), Some(ar_account)) = (state.post_invoice_id, &state.post_ar_account) else {
                state.error = Some("choose a draft (via a row's Post button) and an AR account".into());
                return;
            };
            let service = SalesService::new(ctx.db.clone());
            match service.post_invoice(invoice_id, ar_account.id) {
                Ok((posted, _transaction, _receivable)) => {
                    if let Some(slot) = state.invoices.iter_mut().find(|i| i.id == Some(invoice_id)) {
                        *slot = posted;
                    }
                    state.message = Some(format!("Posted INV-{invoice_id}."));
                    state.post_invoice_id = None;
                    state.post_ar_account = None;
                }
                Err(err) => state.error = Some(err.to_string()),
            }
        }
        Message::KebabForRow(invoice_id) => {
            state.message = None;
            state.error = None;
            let Some(invoice) = state.invoices.iter().find(|i| i.id == Some(invoice_id)).cloned() else { return };
            match invoice.status {
                InvoiceStatus::Draft => {
                    // `state.invoices` only holds headers (no lines), so
                    // the full record is needed to prefill the form.
                    let service = SalesService::new(ctx.db.clone());
                    match service.get_invoice(invoice_id) {
                        Ok(full) => {
                            state.editing_id = Some(invoice_id);
                            state.customer = state.customers.iter().find(|c| c.id == full.customer_id).cloned();
                            let first_line = full.lines.first();
                            state.revenue_account = first_line.and_then(|line| state.accounts.iter().find(|a| a.id == line.revenue_account_id).cloned());
                            state.description_input = first_line.map(|line| line.description.clone()).unwrap_or_default();
                            state.amount_input = first_line.map(|line| line.amount().to_string()).unwrap_or_default();
                        }
                        Err(err) => state.error = Some(err.to_string()),
                    }
                }
                InvoiceStatus::Posted => {
                    let service = SalesService::new(ctx.db.clone());
                    state.credit_note_invoice_id = Some(invoice_id);
                    state.credit_note_amount_input = service.invoice_remaining_balance(invoice_id).map(|balance| balance.to_string()).unwrap_or_default();
                }
                InvoiceStatus::Voided => {}
            }
        }
        Message::CreditNoteRevenueAccountSelected(choice) => state.credit_note_revenue_account = Some(choice),
        Message::CreditNoteArAccountSelected(choice) => state.credit_note_ar_account = Some(choice),
        Message::CreditNoteDescriptionChanged(value) => state.credit_note_description_input = value,
        Message::CreditNoteAmountChanged(value) => state.credit_note_amount_input = value,
        Message::SubmitCreditNote => {
            state.message = None;
            state.error = None;

            let (Some(invoice_id), Some(revenue_account), Some(ar_account)) = (state.credit_note_invoice_id, &state.credit_note_revenue_account, &state.credit_note_ar_account) else {
                state.error = Some("choose an invoice (via a posted row's kebab menu), revenue account, and AR account".into());
                return;
            };
            let amount: Decimal = match state.credit_note_amount_input.parse() {
                Ok(amount) => amount,
                Err(_) => {
                    state.error = Some("amount is not a valid number".into());
                    return;
                }
            };

            let today = Utc::now().date_naive();
            let service = SalesService::new(ctx.db.clone());
            match service.create_and_post_credit_note_for_invoice(invoice_id, today, revenue_account.id, state.credit_note_description_input.clone(), amount, ar_account.id) {
                Ok((credit_note, _transaction)) => {
                    state.message = Some(format!("Credit note #{} posted against INV-{invoice_id}.", credit_note.id.unwrap_or_default()));
                    state.credit_note_description_input.clear();
                    state.credit_note_amount_input.clear();
                    state.credit_note_invoice_id = None;
                }
                Err(err) => state.error = Some(err.to_string()),
            }
        }
        Message::RecordPaymentForRow(invoice_id) => {
            state.message = None;
            state.error = None;
            let service = SalesService::new(ctx.db.clone());
            match service.invoice_remaining_balance(invoice_id) {
                Ok(balance) => {
                    state.payment_invoice_id = Some(invoice_id);
                    state.payment_amount_input = balance.to_string();
                }
                Err(err) => {
                    state.payment_invoice_id = None;
                    state.error = Some(err.to_string());
                }
            }
        }
        Message::PaymentCashAccountSelected(choice) => state.payment_cash_account = Some(choice),
        Message::PaymentArAccountSelected(choice) => state.payment_ar_account = Some(choice),
        Message::PaymentAmountChanged(value) => state.payment_amount_input = value,
        Message::SubmitPayment => {
            state.message = None;
            state.error = None;

            let (Some(invoice_id), Some(cash_account), Some(ar_account)) = (state.payment_invoice_id, &state.payment_cash_account, &state.payment_ar_account) else {
                state.error = Some("choose an invoice (via a row's Record Payment button), cash account, and AR account".into());
                return;
            };
            let amount: Decimal = match state.payment_amount_input.parse() {
                Ok(amount) => amount,
                Err(_) => {
                    state.error = Some("amount is not a valid number".into());
                    return;
                }
            };

            let today = Utc::now().date_naive();
            let service = SalesService::new(ctx.db.clone());
            match service.record_payment_for_invoice(invoice_id, today, amount, cash_account.id, ar_account.id) {
                Ok(recorded) => {
                    state.message = Some(format!("Recorded payment #{} against INV-{invoice_id}.", recorded.id.unwrap_or_default()));
                    state.payment_amount_input.clear();
                    state.payment_invoice_id = None;
                }
                Err(err) => state.error = Some(err.to_string()),
            }
        }
    }
}

pub fn view(state: &State, theme: Theme) -> Element<'_, Message> {
    let header = page_header::view(theme, "Invoices", "Everything you've billed customers, in one place.", "Create & post", Message::CreateAndPost);

    let toolbar = toolbar::view(
        theme,
        &["Draft", "Posted", "Voided"],
        state.active_tab,
        Message::TabSelected,
        &state.search,
        "Search invoices",
        Message::SearchChanged,
        "Filters",
        Message::FiltersClicked,
        "Sort by",
        Message::SortClicked,
    );

    let heading = section_heading::view(theme, "Invoice list", "Every invoice on this company's books.");

    let tab_status = match state.active_tab {
        0 => InvoiceStatus::Draft,
        1 => InvoiceStatus::Posted,
        _ => InvoiceStatus::Voided,
    };
    let query = state.search.trim().to_lowercase();
    let filtered: Vec<&Invoice> = state
        .invoices
        .iter()
        .filter(|invoice| invoice.status == tab_status)
        .filter(|invoice| query.is_empty() || invoice.id.map(|id| format!("INV-{id}")).unwrap_or_default().to_lowercase().contains(&query))
        .collect();

    let columns = vec![
        ColumnDef::new("Invoice", Length::FillPortion(1)).sorted(SortDirection::Descending),
        ColumnDef::new("Date", Length::FillPortion(1)),
        ColumnDef::new("Status", Length::FillPortion(1)),
        ColumnDef::new("Total", Length::FillPortion(1)),
    ];

    let rows = filtered
        .iter()
        .map(|invoice| {
            let row = Row::new(vec![
                Cell::text(invoice.id.map(|id| format!("INV-{id}")).unwrap_or_default()),
                Cell::date(invoice.invoice_date.to_string()),
                Cell::status(invoice.status.as_str(), status_tone(invoice.status.as_str())),
                Cell::text(invoice.total().to_string()),
            ]);
            match invoice.status {
                InvoiceStatus::Draft => row.with_primary_action("Post"),
                InvoiceStatus::Posted => row.with_primary_action("Record Payment"),
                InvoiceStatus::Voided => row,
            }
        })
        .collect();

    let invoice_ids: Vec<i64> = filtered.iter().map(|invoice| invoice.id.unwrap_or_default()).collect();
    let select_ids = invoice_ids.clone();
    let selected_flags: Vec<bool> = invoice_ids.iter().map(|id| state.selected.contains(id)).collect();
    let kebab_ids = invoice_ids.clone();
    let primary_ids = invoice_ids.clone();
    let active_tab = state.active_tab;

    let table = data_table::view(
        theme,
        columns,
        rows,
        if active_tab == 0 { Some(selected_flags) } else { None },
        move |index, checked| Message::RowSelected(invoice_ids[index], checked),
        move |checked| Message::SelectAllChanged(select_ids.clone(), checked),
        move |index| Message::KebabForRow(kebab_ids[index]),
        move |index| {
            let id = primary_ids[index];
            if active_tab == 0 { Message::PostForRow(id) } else { Message::RecordPaymentForRow(id) }
        },
    );

    let form_heading = if state.editing_id.is_some() { "Edit draft invoice" } else { "Create an invoice" };
    let mut form = column![
        text(form_heading).size(typography::LABEL),
        pick_list(state.customers.as_slice(), state.customer.clone(), Message::CustomerSelected).style(dropdown::style(theme)).placeholder("Customer"),
    ]
    .spacing(8);

    if state.editing_id.is_none() {
        form = form.push(pick_list(state.accounts.as_slice(), state.ar_account.clone(), Message::ArAccountSelected).style(dropdown::style(theme)).placeholder("AR account (for Create & post)"));
    }
    form = form.push(pick_list(state.accounts.as_slice(), state.revenue_account.clone(), Message::RevenueAccountSelected).style(dropdown::style(theme)).placeholder("Revenue account"));
    form = form.push(text_input("Description", &state.description_input).style(inputs::text(theme)).on_input(Message::DescriptionChanged).width(Length::Fill).padding(8));
    form = form.push(text_input("Amount", &state.amount_input).style(inputs::text(theme)).on_input(Message::AmountChanged).width(Length::Fill).padding(8));

    if state.editing_id.is_some() {
        form = form.push(
            iced::widget::row![
                button(text("Save changes")).style(button_style::primary(theme)).on_press(Message::SaveDraft).padding(8),
                button(text("Cancel")).style(button_style::secondary(theme)).on_press(Message::CancelEdit).padding(8),
            ]
            .spacing(8),
        );
    } else {
        form = form.push(
            iced::widget::row![
                button(text("Save as draft")).style(button_style::secondary(theme)).on_press(Message::SaveDraft).padding(8),
                button(text("Create & post")).style(button_style::primary(theme)).on_press(Message::CreateAndPost).padding(8),
            ]
            .spacing(8),
        );
    }

    let post_target_label = state
        .post_invoice_id
        .and_then(|id| state.invoices.iter().find(|invoice| invoice.id == Some(id)).map(|invoice| (id, invoice.customer_id)))
        .map(|(id, customer_id)| {
            let customer_name = state.customers.iter().find(|c| c.id == customer_id).map(|c| c.label.clone()).unwrap_or_else(|| "unknown customer".to_string());
            format!("INV-{id} ({customer_name})")
        })
        .unwrap_or_else(|| "Click a draft row's Post button to choose an invoice".to_string());
    let post_form = column![
        text("Post this draft").size(typography::LABEL),
        text(format!("Invoice: {post_target_label}")).size(typography::BODY).color(theme.neutral_front()),
        pick_list(state.accounts.as_slice(), state.post_ar_account.clone(), Message::PostArAccountSelected).style(dropdown::style(theme)).placeholder("AR account"),
        button(text("Post")).style(button_style::primary(theme)).on_press(Message::SubmitPost).padding(8),
    ]
    .spacing(8);

    let payment_target_label = state
        .payment_invoice_id
        .and_then(|id| state.invoices.iter().find(|invoice| invoice.id == Some(id)).map(|invoice| (id, invoice.customer_id)))
        .map(|(id, customer_id)| {
            let customer_name = state.customers.iter().find(|c| c.id == customer_id).map(|c| c.label.clone()).unwrap_or_else(|| "unknown customer".to_string());
            format!("INV-{id} ({customer_name})")
        })
        .unwrap_or_else(|| "Click a row's Record Payment button to choose an invoice".to_string());
    let payment_form = column![
        text("Record a payment").size(typography::LABEL),
        text(format!("Invoice: {payment_target_label}")).size(typography::BODY).color(theme.neutral_front()),
        pick_list(state.accounts.as_slice(), state.payment_cash_account.clone(), Message::PaymentCashAccountSelected).style(dropdown::style(theme)).placeholder("Cash account"),
        pick_list(state.accounts.as_slice(), state.payment_ar_account.clone(), Message::PaymentArAccountSelected).style(dropdown::style(theme)).placeholder("AR account"),
        text_input("Amount", &state.payment_amount_input).style(inputs::text(theme)).on_input(Message::PaymentAmountChanged).width(Length::Fill).padding(8),
        button(text("Record payment")).style(button_style::primary(theme)).on_press(Message::SubmitPayment).padding(8),
    ]
    .spacing(8);

    let credit_note_target_label = state
        .credit_note_invoice_id
        .and_then(|id| state.invoices.iter().find(|invoice| invoice.id == Some(id)).map(|invoice| (id, invoice.customer_id)))
        .map(|(id, customer_id)| {
            let customer_name = state.customers.iter().find(|c| c.id == customer_id).map(|c| c.label.clone()).unwrap_or_else(|| "unknown customer".to_string());
            format!("INV-{id} ({customer_name})")
        })
        .unwrap_or_else(|| "Click a posted row's kebab menu to choose an invoice".to_string());
    let credit_note_form = column![
        text("Issue a credit note").size(typography::LABEL),
        text(format!("Invoice: {credit_note_target_label}")).size(typography::BODY).color(theme.neutral_front()),
        pick_list(state.accounts.as_slice(), state.credit_note_revenue_account.clone(), Message::CreditNoteRevenueAccountSelected).style(dropdown::style(theme)).placeholder("Revenue account"),
        pick_list(state.accounts.as_slice(), state.credit_note_ar_account.clone(), Message::CreditNoteArAccountSelected).style(dropdown::style(theme)).placeholder("AR account"),
        text_input("Description", &state.credit_note_description_input).style(inputs::text(theme)).on_input(Message::CreditNoteDescriptionChanged).width(Length::Fill).padding(8),
        text_input("Amount", &state.credit_note_amount_input).style(inputs::text(theme)).on_input(Message::CreditNoteAmountChanged).width(Length::Fill).padding(8),
        button(text("Issue credit note")).style(button_style::primary(theme)).on_press(Message::SubmitCreditNote).padding(8),
    ]
    .spacing(8);

    let mut content = column![header, toolbar, heading].spacing(20).padding(24).width(Length::Fill);

    if active_tab == 0 && !state.selected.is_empty() {
        content = content.push(bulk_action_bar::view(theme, state.selected.len(), Message::BulkDelete, Message::BulkExportCsv, Message::ClearSelection));
    }

    content = content.push(table).push(form).push(post_form).push(payment_form).push(credit_note_form);

    if let Some(message) = &state.message {
        content = content.push(text(message.clone()).color(theme.positive_front()));
    }
    if let Some(error) = &state.error {
        content = content.push(text(error.clone()).color(theme.negative_front()));
    }

    container(scrollable(content).height(Length::Fill)).width(Length::Fill).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::accounting::accounting_rules::AccountType;
    use crate::backend::accounting::fiscal_periods::FiscalPeriod;
    use crate::backend::organization::companies::Company;
    use crate::backend::parties::customers::Customer;
    use crate::backend::repositories::accounting_repository::{self, AccountingRepository};
    use crate::backend::repositories::organization_repository::{self, OrganizationRepository};
    use crate::backend::repositories::party_repository::{self, PartyRepository};
    use crate::backend::repositories::sales_repository;
    use crate::database::database::Database;
    use chrono::NaiveDate;
    use std::path::PathBuf;

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    fn ctx_with_customer_and_accounts() -> (Ctx, Choice, Choice, Choice) {
        let db = Database::open_in_memory().unwrap();
        db.run_migrations(organization_repository::MIGRATIONS).unwrap();
        db.run_migrations(party_repository::MIGRATIONS).unwrap();
        db.run_migrations(accounting_repository::MIGRATIONS).unwrap();
        db.run_migrations(crate::backend::audit::audit_logs::MIGRATIONS).unwrap();
        db.run_migrations(sales_repository::MIGRATIONS).unwrap();

        let org_repo = OrganizationRepository::new(db.clone());
        let company_id = org_repo.create_company(&Company::new("Acme Trading Co", "USD")).unwrap().id.unwrap();

        let party_repo = PartyRepository::new(db.clone());
        let customer = party_repo.create_customer(&Customer::new(company_id, "Acme Retail")).unwrap();

        let accounting_repo = AccountingRepository::new(db.clone());
        accounting_repo.create_fiscal_period(&FiscalPeriod::new(company_id, "Open Period", date(2026, 1, 1), date(2030, 12, 31))).unwrap();
        let ar = accounting_repo.create_account(&Account::new(company_id, "1100", "Accounts Receivable", AccountType::Asset)).unwrap();
        let revenue = accounting_repo.create_account(&Account::new(company_id, "4000", "Sales Revenue", AccountType::Revenue)).unwrap();

        let ctx = Ctx { db, company_id, current_user_id: None, export_dir: PathBuf::from(".") };
        let customer_choice = Choice { id: customer.id.unwrap(), label: customer.name.clone() };
        let ar_choice = Choice { id: ar.id.unwrap(), label: "1100 - Accounts Receivable".into() };
        let revenue_choice = Choice { id: revenue.id.unwrap(), label: "4000 - Sales Revenue".into() };
        (ctx, customer_choice, ar_choice, revenue_choice)
    }

    #[test]
    fn field_changed_messages_update_the_corresponding_field() {
        let (ctx, customer, ar, revenue) = ctx_with_customer_and_accounts();
        let mut state = State::default();

        update(&mut state, Message::CustomerSelected(customer.clone()), &ctx);
        update(&mut state, Message::ArAccountSelected(ar.clone()), &ctx);
        update(&mut state, Message::RevenueAccountSelected(revenue.clone()), &ctx);
        update(&mut state, Message::DescriptionChanged("Consulting".into()), &ctx);
        update(&mut state, Message::AmountChanged("1500".into()), &ctx);

        assert_eq!(state.customer, Some(customer));
        assert_eq!(state.ar_account, Some(ar));
        assert_eq!(state.revenue_account, Some(revenue));
        assert_eq!(state.description_input, "Consulting");
        assert_eq!(state.amount_input, "1500");
    }

    #[test]
    fn creating_without_choosing_customer_and_both_accounts_sets_an_error() {
        let (ctx, customer, _ar, _revenue) = ctx_with_customer_and_accounts();
        let mut state = State { customer: Some(customer), ..Default::default() };

        update(&mut state, Message::CreateAndPost, &ctx);

        assert_eq!(state.error.as_deref(), Some("choose a customer, AR account, and revenue account"));
    }

    #[test]
    fn creating_with_a_non_numeric_amount_sets_an_error() {
        let (ctx, customer, ar, revenue) = ctx_with_customer_and_accounts();
        let mut state = State {
            customer: Some(customer),
            ar_account: Some(ar),
            revenue_account: Some(revenue),
            amount_input: "not a number".into(),
            ..Default::default()
        };

        update(&mut state, Message::CreateAndPost, &ctx);

        assert_eq!(state.error.as_deref(), Some("amount is not a valid number"));
    }

    #[test]
    fn creating_and_posting_a_valid_invoice_adds_it_to_the_list_and_clears_the_form() {
        let (ctx, customer, ar, revenue) = ctx_with_customer_and_accounts();
        let mut state = State {
            customer: Some(customer),
            ar_account: Some(ar),
            revenue_account: Some(revenue),
            description_input: "Consulting".into(),
            amount_input: "1500".into(),
            ..Default::default()
        };

        update(&mut state, Message::CreateAndPost, &ctx);

        assert!(state.error.is_none());
        assert!(state.message.as_deref().unwrap_or_default().starts_with("Posted invoice"));
        assert_eq!(state.invoices.len(), 1);
        assert_eq!(state.invoices[0].total(), rust_decimal_macros::dec!(1500));
        assert!(state.description_input.is_empty());
        assert!(state.amount_input.is_empty());
    }

    #[test]
    fn search_changed_updates_the_search_field() {
        let (ctx, _customer, _ar, _revenue) = ctx_with_customer_and_accounts();
        let mut state = State::default();

        update(&mut state, Message::SearchChanged("inv-1".into()), &ctx);

        assert_eq!(state.search, "inv-1");
    }

    /// Creates and posts an invoice for `amount` and returns its id.
    fn create_and_post_invoice(state: &mut State, ctx: &Ctx, customer: &Choice, ar: &Choice, revenue: &Choice, amount: &str) -> i64 {
        state.customer = Some(customer.clone());
        state.ar_account = Some(ar.clone());
        state.revenue_account = Some(revenue.clone());
        state.description_input = "Consulting".into();
        state.amount_input = amount.into();
        update(state, Message::CreateAndPost, ctx);
        state.invoices.last().unwrap().id.unwrap()
    }

    /// Saves a new draft (no posting) for `amount` and returns its id.
    fn save_draft_invoice(state: &mut State, ctx: &Ctx, customer: &Choice, revenue: &Choice, amount: &str) -> i64 {
        state.customer = Some(customer.clone());
        state.revenue_account = Some(revenue.clone());
        state.description_input = "Consulting".into();
        state.amount_input = amount.into();
        update(state, Message::SaveDraft, ctx);
        state.invoices.last().unwrap().id.unwrap()
    }

    #[test]
    fn recording_a_row_s_payment_prefills_the_amount_with_that_invoice_s_own_balance() {
        let (ctx, customer, ar, revenue) = ctx_with_customer_and_accounts();
        let mut state = State { customers: vec![customer.clone()], ..Default::default() };
        let invoice_id = create_and_post_invoice(&mut state, &ctx, &customer, &ar, &revenue, "1000");

        update(&mut state, Message::RecordPaymentForRow(invoice_id), &ctx);

        assert_eq!(state.payment_invoice_id, Some(invoice_id));
        assert_eq!(state.payment_amount_input, "1000");
        assert!(state.error.is_none());
    }

    #[test]
    fn submitting_a_payment_without_choosing_an_invoice_and_both_accounts_sets_an_error() {
        let (ctx, _customer, _ar, _revenue) = ctx_with_customer_and_accounts();
        let mut state = State::default();

        update(&mut state, Message::SubmitPayment, &ctx);

        assert_eq!(state.error.as_deref(), Some("choose an invoice (via a row's Record Payment button), cash account, and AR account"));
    }

    #[test]
    fn submitting_a_payment_with_a_non_numeric_amount_sets_an_error() {
        let (ctx, customer, ar, revenue) = ctx_with_customer_and_accounts();
        let cash = ar.clone();
        let mut state = State { customers: vec![customer.clone()], ..Default::default() };
        let invoice_id = create_and_post_invoice(&mut state, &ctx, &customer, &ar, &revenue, "1000");
        state.payment_invoice_id = Some(invoice_id);
        state.payment_cash_account = Some(cash);
        state.payment_ar_account = Some(ar);
        state.payment_amount_input = "not a number".into();

        update(&mut state, Message::SubmitPayment, &ctx);

        assert_eq!(state.error.as_deref(), Some("amount is not a valid number"));
    }

    #[test]
    fn a_payment_exactly_equal_to_the_invoice_s_balance_fully_settles_it() {
        let (ctx, customer, ar, revenue) = ctx_with_customer_and_accounts();
        let cash = ar.clone();
        let mut state = State { customers: vec![customer.clone()], ..Default::default() };
        let invoice_id = create_and_post_invoice(&mut state, &ctx, &customer, &ar, &revenue, "1000");

        update(&mut state, Message::RecordPaymentForRow(invoice_id), &ctx);
        update(&mut state, Message::PaymentCashAccountSelected(cash), &ctx);
        update(&mut state, Message::PaymentArAccountSelected(ar), &ctx);
        update(&mut state, Message::SubmitPayment, &ctx);

        assert!(state.error.is_none());
        assert!(state.message.as_deref().unwrap_or_default().contains(&format!("INV-{invoice_id}")));
        assert!(state.payment_invoice_id.is_none());

        let service = SalesService::new(ctx.db.clone());
        assert!(service.invoice_remaining_balance(invoice_id).is_err(), "a fully-settled invoice has no open receivable left");
    }

    #[test]
    fn a_partial_payment_reduces_only_that_invoice_s_balance() {
        let (ctx, customer, ar, revenue) = ctx_with_customer_and_accounts();
        let cash = ar.clone();
        let mut state = State { customers: vec![customer.clone()], ..Default::default() };
        let invoice_id = create_and_post_invoice(&mut state, &ctx, &customer, &ar, &revenue, "1000");

        update(&mut state, Message::RecordPaymentForRow(invoice_id), &ctx);
        state.payment_amount_input = "400".into();
        update(&mut state, Message::PaymentCashAccountSelected(cash), &ctx);
        update(&mut state, Message::PaymentArAccountSelected(ar), &ctx);
        update(&mut state, Message::SubmitPayment, &ctx);

        assert!(state.error.is_none());
        let service = SalesService::new(ctx.db.clone());
        assert_eq!(service.invoice_remaining_balance(invoice_id).unwrap(), rust_decimal_macros::dec!(600));
    }

    #[test]
    fn an_overpayment_attempt_beyond_the_invoice_s_own_balance_is_rejected() {
        let (ctx, customer, ar, revenue) = ctx_with_customer_and_accounts();
        let cash = ar.clone();
        let mut state = State { customers: vec![customer.clone()], ..Default::default() };
        let invoice_id = create_and_post_invoice(&mut state, &ctx, &customer, &ar, &revenue, "1000");

        update(&mut state, Message::RecordPaymentForRow(invoice_id), &ctx);
        state.payment_amount_input = "1500".into();
        update(&mut state, Message::PaymentCashAccountSelected(cash), &ctx);
        update(&mut state, Message::PaymentArAccountSelected(ar), &ctx);
        update(&mut state, Message::SubmitPayment, &ctx);

        assert!(state.error.as_deref().unwrap_or_default().contains("exceeds"));
        assert!(state.message.is_none());

        let service = SalesService::new(ctx.db.clone());
        assert_eq!(service.invoice_remaining_balance(invoice_id).unwrap(), rust_decimal_macros::dec!(1000), "a rejected overpayment must not touch the invoice's balance at all");
    }

    /// Regression test for the exact bug this feature replaces: paying
    /// one invoice via its row must never touch a *different* open
    /// invoice for the same customer, even partially.
    #[test]
    fn paying_one_invoice_via_its_row_leaves_the_customer_s_other_open_invoice_completely_untouched() {
        let (ctx, customer, ar, revenue) = ctx_with_customer_and_accounts();
        let cash = ar.clone();
        let mut state = State { customers: vec![customer.clone()], ..Default::default() };
        let first_invoice_id = create_and_post_invoice(&mut state, &ctx, &customer, &ar, &revenue, "1000");
        let second_invoice_id = create_and_post_invoice(&mut state, &ctx, &customer, &ar, &revenue, "500");

        update(&mut state, Message::RecordPaymentForRow(first_invoice_id), &ctx);
        update(&mut state, Message::PaymentCashAccountSelected(cash), &ctx);
        update(&mut state, Message::PaymentArAccountSelected(ar), &ctx);
        update(&mut state, Message::SubmitPayment, &ctx);

        assert!(state.error.is_none());

        let service = SalesService::new(ctx.db.clone());
        assert!(service.invoice_remaining_balance(first_invoice_id).is_err(), "the paid invoice should be fully settled");
        assert_eq!(
            service.invoice_remaining_balance(second_invoice_id).unwrap(),
            rust_decimal_macros::dec!(500),
            "the customer's other open invoice must be completely untouched"
        );
    }

    #[test]
    fn tab_selected_updates_the_active_tab() {
        let (ctx, _customer, _ar, _revenue) = ctx_with_customer_and_accounts();
        let mut state = State::default();

        update(&mut state, Message::TabSelected(2), &ctx);

        assert_eq!(state.active_tab, 2);
    }

    #[test]
    fn kebab_on_a_voided_invoice_is_a_no_op() {
        let (ctx, customer, _ar, revenue) = ctx_with_customer_and_accounts();
        let service = SalesService::new(ctx.db.clone());
        let mut invoice = Invoice::new(ctx.company_id, customer.id, date(2026, 1, 5), date(2026, 2, 4));
        invoice.lines.push(InvoiceLine::new(revenue.id, "Consulting", rust_decimal_macros::dec!(1), rust_decimal_macros::dec!(500)));
        let draft = service.create_invoice(&invoice).unwrap();
        let voided = service.void_invoice(draft.id.unwrap()).unwrap();
        let mut state = State { invoices: vec![voided.clone()], ..Default::default() };

        update(&mut state, Message::KebabForRow(voided.id.unwrap()), &ctx);

        assert!(state.error.is_none());
        assert!(state.editing_id.is_none());
        assert!(state.credit_note_invoice_id.is_none());
        assert_eq!(state.invoices[0].status, InvoiceStatus::Voided);
    }

    #[test]
    fn kebab_on_a_posted_invoice_opens_the_credit_note_form_prefilled_with_its_own_balance() {
        let (ctx, customer, ar, revenue) = ctx_with_customer_and_accounts();
        let mut state = State { customers: vec![customer.clone()], ..Default::default() };
        let invoice_id = create_and_post_invoice(&mut state, &ctx, &customer, &ar, &revenue, "750");

        update(&mut state, Message::KebabForRow(invoice_id), &ctx);

        assert_eq!(state.credit_note_invoice_id, Some(invoice_id));
        assert_eq!(state.credit_note_amount_input, "750");
    }

    #[test]
    fn submitting_a_credit_note_without_choosing_an_invoice_and_both_accounts_sets_an_error() {
        let (ctx, _customer, _ar, _revenue) = ctx_with_customer_and_accounts();
        let mut state = State::default();

        update(&mut state, Message::SubmitCreditNote, &ctx);

        assert_eq!(state.error.as_deref(), Some("choose an invoice (via a posted row's kebab menu), revenue account, and AR account"));
    }

    #[test]
    fn a_credit_note_exactly_equal_to_the_invoice_s_balance_fully_settles_it() {
        let (ctx, customer, ar, revenue) = ctx_with_customer_and_accounts();
        let mut state = State { customers: vec![customer.clone()], ..Default::default() };
        let invoice_id = create_and_post_invoice(&mut state, &ctx, &customer, &ar, &revenue, "1000");

        update(&mut state, Message::KebabForRow(invoice_id), &ctx);
        update(&mut state, Message::CreditNoteRevenueAccountSelected(revenue), &ctx);
        update(&mut state, Message::CreditNoteArAccountSelected(ar), &ctx);
        state.credit_note_description_input = "Return".into();
        update(&mut state, Message::SubmitCreditNote, &ctx);

        assert!(state.error.is_none());
        assert!(state.message.as_deref().unwrap_or_default().starts_with("Credit note"));
        assert!(state.credit_note_invoice_id.is_none());

        let service = SalesService::new(ctx.db.clone());
        assert!(service.invoice_remaining_balance(invoice_id).is_err(), "a fully-credited invoice has no open receivable left");
    }

    #[test]
    fn a_credit_note_amount_exceeding_the_invoice_s_own_balance_is_rejected() {
        let (ctx, customer, ar, revenue) = ctx_with_customer_and_accounts();
        let mut state = State { customers: vec![customer.clone()], ..Default::default() };
        let invoice_id = create_and_post_invoice(&mut state, &ctx, &customer, &ar, &revenue, "1000");

        update(&mut state, Message::KebabForRow(invoice_id), &ctx);
        state.credit_note_amount_input = "1500".into();
        update(&mut state, Message::CreditNoteRevenueAccountSelected(revenue), &ctx);
        update(&mut state, Message::CreditNoteArAccountSelected(ar), &ctx);
        state.credit_note_description_input = "Return".into();
        update(&mut state, Message::SubmitCreditNote, &ctx);

        assert!(state.error.as_deref().unwrap_or_default().contains("exceeds"));
        assert!(state.message.is_none());

        let service = SalesService::new(ctx.db.clone());
        assert_eq!(service.invoice_remaining_balance(invoice_id).unwrap(), rust_decimal_macros::dec!(1000), "a rejected credit note must not touch the invoice's balance at all");
    }

    /// Same regression shape as the payment-scoping test above, now for
    /// credit notes: issuing one against a single invoice must never
    /// touch a *different* open invoice for the same customer.
    #[test]
    fn issuing_a_credit_note_against_one_invoice_leaves_the_customer_s_other_open_invoice_completely_untouched() {
        let (ctx, customer, ar, revenue) = ctx_with_customer_and_accounts();
        let mut state = State { customers: vec![customer.clone()], ..Default::default() };
        let first_invoice_id = create_and_post_invoice(&mut state, &ctx, &customer, &ar, &revenue, "1000");
        let second_invoice_id = create_and_post_invoice(&mut state, &ctx, &customer, &ar, &revenue, "500");

        update(&mut state, Message::KebabForRow(first_invoice_id), &ctx);
        update(&mut state, Message::CreditNoteRevenueAccountSelected(revenue), &ctx);
        update(&mut state, Message::CreditNoteArAccountSelected(ar), &ctx);
        state.credit_note_description_input = "Return".into();
        update(&mut state, Message::SubmitCreditNote, &ctx);

        assert!(state.error.is_none());

        let service = SalesService::new(ctx.db.clone());
        assert!(service.invoice_remaining_balance(first_invoice_id).is_err(), "the credited invoice should be fully settled");
        assert_eq!(
            service.invoice_remaining_balance(second_invoice_id).unwrap(),
            rust_decimal_macros::dec!(500),
            "the customer's other open invoice must be completely untouched"
        );
    }

    #[test]
    fn saving_a_new_draft_without_choosing_a_customer_sets_an_error() {
        let (ctx, _customer, _ar, revenue) = ctx_with_customer_and_accounts();
        let mut state = State { revenue_account: Some(revenue), amount_input: "500".into(), ..Default::default() };

        update(&mut state, Message::SaveDraft, &ctx);

        assert_eq!(state.error.as_deref(), Some("choose a customer"));
        assert!(state.invoices.is_empty());
    }

    #[test]
    fn saving_a_new_draft_adds_it_to_the_list_as_draft_and_clears_the_form() {
        let (ctx, customer, _ar, revenue) = ctx_with_customer_and_accounts();
        let mut state = State {
            customer: Some(customer),
            revenue_account: Some(revenue),
            description_input: "Consulting".into(),
            amount_input: "500".into(),
            ..Default::default()
        };

        update(&mut state, Message::SaveDraft, &ctx);

        assert!(state.error.is_none());
        assert_eq!(state.invoices.len(), 1);
        assert_eq!(state.invoices[0].status, InvoiceStatus::Draft);
        assert_eq!(state.invoices[0].total(), rust_decimal_macros::dec!(500));
        assert!(state.customer.is_none());
        assert!(state.description_input.is_empty());
    }

    #[test]
    fn kebab_on_a_draft_invoice_opens_the_edit_form_prefilled() {
        let (ctx, customer, _ar, revenue) = ctx_with_customer_and_accounts();
        let mut state = State { customers: vec![customer.clone()], accounts: vec![revenue.clone()], ..Default::default() };
        let invoice_id = save_draft_invoice(&mut state, &ctx, &customer, &revenue, "500");

        update(&mut state, Message::KebabForRow(invoice_id), &ctx);

        assert_eq!(state.editing_id, Some(invoice_id));
        assert_eq!(state.customer, Some(customer));
        assert_eq!(state.revenue_account, Some(revenue));
        assert_eq!(state.description_input, "Consulting");
        assert_eq!(state.amount_input, "500");
    }

    #[test]
    fn cancel_edit_clears_editing_state_without_touching_the_stored_draft() {
        let (ctx, customer, _ar, revenue) = ctx_with_customer_and_accounts();
        let mut state = State { customers: vec![customer.clone()], accounts: vec![revenue.clone()], ..Default::default() };
        let invoice_id = save_draft_invoice(&mut state, &ctx, &customer, &revenue, "500");

        update(&mut state, Message::KebabForRow(invoice_id), &ctx);
        state.description_input = "Should Not Save".into();
        update(&mut state, Message::CancelEdit, &ctx);

        assert!(state.editing_id.is_none());
        assert!(state.description_input.is_empty());

        let service = SalesService::new(ctx.db.clone());
        let reloaded = service.get_invoice(invoice_id).unwrap();
        assert_eq!(reloaded.lines[0].description, "Consulting");
    }

    #[test]
    fn saving_changes_to_a_draft_updates_it_in_place_and_clears_editing_state() {
        let (ctx, customer, _ar, revenue) = ctx_with_customer_and_accounts();
        let mut state = State { customers: vec![customer.clone()], accounts: vec![revenue.clone()], ..Default::default() };
        let invoice_id = save_draft_invoice(&mut state, &ctx, &customer, &revenue, "500");

        update(&mut state, Message::KebabForRow(invoice_id), &ctx);
        state.description_input = "Revised consulting".into();
        state.amount_input = "800".into();
        update(&mut state, Message::SaveDraft, &ctx);

        assert!(state.error.is_none());
        assert!(state.editing_id.is_none());
        assert_eq!(state.invoices.len(), 1);
        assert_eq!(state.invoices[0].total(), rust_decimal_macros::dec!(800));

        let service = SalesService::new(ctx.db.clone());
        let reloaded = service.get_invoice(invoice_id).unwrap();
        assert_eq!(reloaded.lines[0].description, "Revised consulting");
    }

    #[test]
    fn row_selected_toggles_membership_in_the_selection() {
        let (ctx, _customer, _ar, _revenue) = ctx_with_customer_and_accounts();
        let mut state = State::default();

        update(&mut state, Message::RowSelected(1, true), &ctx);
        update(&mut state, Message::RowSelected(2, true), &ctx);
        assert_eq!(state.selected, HashSet::from([1, 2]));

        update(&mut state, Message::RowSelected(1, false), &ctx);
        assert_eq!(state.selected, HashSet::from([2]));
    }

    #[test]
    fn select_all_changed_selects_or_clears_every_given_id() {
        let (ctx, _customer, _ar, _revenue) = ctx_with_customer_and_accounts();
        let mut state = State::default();

        update(&mut state, Message::SelectAllChanged(vec![1, 2, 3], true), &ctx);
        assert_eq!(state.selected, HashSet::from([1, 2, 3]));

        update(&mut state, Message::SelectAllChanged(vec![1, 2, 3], false), &ctx);
        assert!(state.selected.is_empty());
    }

    #[test]
    fn clear_selection_empties_it_regardless_of_what_was_selected() {
        let (ctx, _customer, _ar, _revenue) = ctx_with_customer_and_accounts();
        let mut state = State { selected: HashSet::from([1, 2, 3]), ..Default::default() };

        update(&mut state, Message::ClearSelection, &ctx);

        assert!(state.selected.is_empty());
    }

    #[test]
    fn bulk_delete_removes_every_selected_draft_invoice() {
        let (ctx, customer, _ar, revenue) = ctx_with_customer_and_accounts();
        let mut state = State { customers: vec![customer.clone()], ..Default::default() };
        save_draft_invoice(&mut state, &ctx, &customer, &revenue, "500");
        save_draft_invoice(&mut state, &ctx, &customer, &revenue, "300");
        let ids: Vec<i64> = state.invoices.iter().map(|i| i.id.unwrap()).collect();
        state.selected = ids.iter().copied().collect();

        update(&mut state, Message::BulkDelete, &ctx);

        assert!(state.error.is_none());
        assert!(state.invoices.is_empty());
        assert!(state.selected.is_empty());
        assert!(state.message.as_deref().unwrap_or_default().contains("Deleted 2"));
    }

    #[test]
    fn bulk_export_csv_writes_only_the_selected_invoices() {
        let (ctx, customer, _ar, revenue) = ctx_with_customer_and_accounts();
        let ctx = Ctx { export_dir: std::env::temp_dir().join(format!("memonbooks_invoices_view_test_{:?}", std::thread::current().id())), ..ctx };
        let mut state = State { customers: vec![customer.clone()], ..Default::default() };
        let export_id = save_draft_invoice(&mut state, &ctx, &customer, &revenue, "500");
        save_draft_invoice(&mut state, &ctx, &customer, &revenue, "300");
        state.selected = HashSet::from([export_id]);

        update(&mut state, Message::BulkExportCsv, &ctx);

        assert!(state.error.is_none());
        let contents = std::fs::read_to_string(ctx.export_path("invoices_export.csv")).unwrap();
        assert!(contents.contains(&format!("INV-{export_id}")));

        let _ = std::fs::remove_dir_all(&ctx.export_dir);
    }

    #[test]
    fn post_for_row_sets_the_target_invoice() {
        let (ctx, customer, _ar, revenue) = ctx_with_customer_and_accounts();
        let mut state = State { customers: vec![customer.clone()], ..Default::default() };
        let invoice_id = save_draft_invoice(&mut state, &ctx, &customer, &revenue, "500");

        update(&mut state, Message::PostForRow(invoice_id), &ctx);

        assert_eq!(state.post_invoice_id, Some(invoice_id));
    }

    #[test]
    fn submitting_post_without_choosing_a_draft_and_an_ar_account_sets_an_error() {
        let (ctx, _customer, _ar, _revenue) = ctx_with_customer_and_accounts();
        let mut state = State::default();

        update(&mut state, Message::SubmitPost, &ctx);

        assert_eq!(state.error.as_deref(), Some("choose a draft (via a row's Post button) and an AR account"));
    }

    #[test]
    fn posting_a_draft_via_its_post_button_moves_it_to_posted_and_creates_a_receivable() {
        let (ctx, customer, ar, revenue) = ctx_with_customer_and_accounts();
        let mut state = State { customers: vec![customer.clone()], ..Default::default() };
        let invoice_id = save_draft_invoice(&mut state, &ctx, &customer, &revenue, "500");

        update(&mut state, Message::PostForRow(invoice_id), &ctx);
        update(&mut state, Message::PostArAccountSelected(ar), &ctx);
        update(&mut state, Message::SubmitPost, &ctx);

        assert!(state.error.is_none());
        assert_eq!(state.invoices[0].status, InvoiceStatus::Posted);
        let service = SalesService::new(ctx.db.clone());
        assert_eq!(service.invoice_remaining_balance(invoice_id).unwrap(), rust_decimal_macros::dec!(500));
    }

    /// The exact scenario requested when this Draft/Edit/Post pattern
    /// was added: create as Draft, edit it, edit it again, then Post --
    /// the final Posted record must reflect the *last* edit, not the
    /// original draft values or the first edit.
    #[test]
    fn creating_a_draft_editing_it_twice_then_posting_reflects_the_last_edit_not_the_original() {
        let (ctx, customer, ar, revenue) = ctx_with_customer_and_accounts();
        let mut state = State { customers: vec![customer.clone()], accounts: vec![ar.clone(), revenue.clone()], ..Default::default() };

        let invoice_id = save_draft_invoice(&mut state, &ctx, &customer, &revenue, "1000");
        assert_eq!(state.invoices[0].status, InvoiceStatus::Draft);

        update(&mut state, Message::KebabForRow(invoice_id), &ctx);
        assert_eq!(state.editing_id, Some(invoice_id));
        state.description_input = "Revised consulting".into();
        state.amount_input = "1200".into();
        update(&mut state, Message::SaveDraft, &ctx);
        assert!(state.error.is_none());
        assert!(state.editing_id.is_none());

        update(&mut state, Message::KebabForRow(invoice_id), &ctx);
        assert_eq!(state.description_input, "Revised consulting");
        assert_eq!(state.amount_input, "1200");
        state.description_input = "Final consulting".into();
        state.amount_input = "1500".into();
        update(&mut state, Message::SaveDraft, &ctx);
        assert!(state.error.is_none());

        update(&mut state, Message::PostForRow(invoice_id), &ctx);
        update(&mut state, Message::PostArAccountSelected(ar), &ctx);
        update(&mut state, Message::SubmitPost, &ctx);

        assert!(state.error.is_none());
        let posted = state.invoices.iter().find(|i| i.id == Some(invoice_id)).unwrap();
        assert_eq!(posted.status, InvoiceStatus::Posted);
        assert_eq!(posted.total(), rust_decimal_macros::dec!(1500));

        let service = SalesService::new(ctx.db.clone());
        let reloaded = service.get_invoice(invoice_id).unwrap();
        assert_eq!(reloaded.lines[0].description, "Final consulting");
        assert_eq!(reloaded.total(), rust_decimal_macros::dec!(1500));
        assert_ne!(reloaded.lines[0].description, "Original consulting");
    }
}
