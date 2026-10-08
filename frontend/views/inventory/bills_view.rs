//! List bills and create, edit, post, and pay a simple single-line bill.
//! Mirrors `sales::invoices_view` exactly (Bill/DebitNote instead of
//! Invoice/CreditNote, vendor instead of customer): a Draft has no
//! ledger impact yet, so it's freely editable and deletable
//! (bulk-select on the Draft tab); once Posted, its GL impact is never
//! rewritten in place -- corrections go through a debit note instead.

use std::collections::HashSet;

use chrono::{Days, Utc};
use rust_decimal::Decimal;

use iced::widget::{button, column, container, pick_list, scrollable, text, text_input};
use iced::{Element, Length};

use crate::backend::accounting::chart_of_accounts::Account;
use crate::backend::purchases::bills::{Bill, BillLine, BillStatus};
use crate::backend::services::purchase_service::PurchaseService;
use crate::database::exports::csv;
use crate::frontend::components::data_table::{self, Cell, ColumnDef, Row, SortDirection};
use crate::frontend::components::{bulk_action_bar, page_header, section_heading, toolbar};
use crate::frontend::components::badges::BadgeTone;
use crate::frontend::components::button as button_style;
use crate::frontend::components::colors::Theme;
use crate::frontend::components::dropdown;
use crate::frontend::components::inputs;
use crate::frontend::components::spacing;
use crate::frontend::components::typography;
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
    bills: Vec<Bill>,
    vendors: Vec<Choice>,
    accounts: Vec<Choice>,
    vendor: Option<Choice>,
    /// Only needed by the "Create & post" fast path -- editing a draft,
    /// or posting one via its own row action, chooses the AP account
    /// separately (`post_ap_account`) at the point it's actually needed.
    ap_account: Option<Choice>,
    expense_account: Option<Choice>,
    description_input: String,
    amount_input: String,
    search: String,
    /// Which status tab is active: 0 = Draft, 1 = Posted, 2 = Voided.
    /// Voided bills stay out of the other two tabs (audit trail, not
    /// clutter) rather than being filtered out of the screen entirely.
    active_tab: usize,
    /// `Some(id)` while the form is editing an existing draft instead of
    /// creating a new one -- `Message::SaveDraft` does double duty as
    /// "save changes" in that state. Only ever reached via a Draft row's
    /// kebab menu; a Posted row's kebab opens the debit-note form
    /// instead, since a posted bill is never edited in place.
    editing_id: Option<i64>,
    /// Bulk-select, Draft rows only -- the checkbox column and bulk
    /// action bar only render on the Draft tab, since only a Draft has
    /// no ledger impact yet and is safe to delete outright.
    selected: HashSet<i64>,
    /// The draft bill a row's "Post" button was last clicked for --
    /// `None` until then, and again after a successful submission.
    post_bill_id: Option<i64>,
    post_ap_account: Option<Choice>,
    /// The bill a row's "Pay" button was last clicked for -- `None`
    /// until then, and again after a successful submission.
    payment_bill_id: Option<i64>,
    payment_ap_account: Option<Choice>,
    payment_cash_account: Option<Choice>,
    payment_amount_input: String,
    /// The posted bill a row's kebab menu was last used to open a debit
    /// note against -- `None` until then, and again after a successful
    /// submission.
    debit_note_bill_id: Option<i64>,
    debit_note_account: Option<Choice>,
    debit_note_ap_account: Option<Choice>,
    debit_note_description_input: String,
    debit_note_amount_input: String,
    message: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    VendorSelected(Choice),
    ApAccountSelected(Choice),
    ExpenseAccountSelected(Choice),
    DescriptionChanged(String),
    AmountChanged(String),
    /// Creates a new Draft (no ledger impact), or -- while `editing_id`
    /// is set -- saves changes to the existing draft in place.
    SaveDraft,
    /// The fast path for a brand new bill: create then immediately post,
    /// in one step. Never available while editing an existing draft --
    /// that's `SaveDraft` (to keep it Draft) plus the row's own "Post"
    /// action (to post it once ready).
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
    /// Fired by a Draft row's "Post" button, carrying that bill's id.
    PostForRow(i64),
    PostApAccountSelected(Choice),
    SubmitPost,
    /// Fired by a Posted bill row's "Pay" button, carrying that bill's
    /// id -- looks up its own remaining balance and prefills the amount
    /// field with it. `SubmitPayment` posts against exactly this bill's
    /// payable, capped at its own balance -- other open bills for the
    /// same vendor are never touched.
    PayForRow(i64),
    PaymentApAccountSelected(Choice),
    PaymentCashAccountSelected(Choice),
    PaymentAmountChanged(String),
    SubmitPayment,
    /// Fired by a row's kebab menu, carrying that bill's id -- a Draft
    /// row opens the edit form, prefilled from its own line; a Posted
    /// row opens the debit-note form below, prefilled with its own
    /// remaining balance; a Voided row has no action.
    KebabForRow(i64),
    DebitNoteAccountSelected(Choice),
    DebitNoteApAccountSelected(Choice),
    DebitNoteDescriptionChanged(String),
    DebitNoteAmountChanged(String),
    SubmitDebitNote,
}

fn status_tone(status: &str) -> BadgeTone {
    match status {
        "posted" => BadgeTone::Positive,
        "voided" => BadgeTone::Negative,
        _ => BadgeTone::Neutral,
    }
}

pub fn load(ctx: &Ctx) -> State {
    let service = PurchaseService::new(ctx.db.clone());

    let vendors = service
        .list_vendors(ctx.company_id)
        .map(|vendors| {
            vendors
                .iter()
                .map(|v| Choice {
                    id: v.id.unwrap_or_default(),
                    label: v.name.clone(),
                })
                .collect()
        })
        .unwrap_or_default();
    let accounts: Vec<Choice> = service
        .list_gl_accounts(ctx.company_id)
        .map(|accounts: Vec<Account>| {
            accounts
                .iter()
                .map(|a| Choice {
                    id: a.id.unwrap_or_default(),
                    label: format!("{} - {}", a.account_code, a.account_name),
                })
                .collect()
        })
        .unwrap_or_default();
    let bills = service.list_bills(ctx.company_id).unwrap_or_default();

    State {
        bills,
        vendors,
        accounts,
        ..Default::default()
    }
}

pub fn update(state: &mut State, message: Message, ctx: &Ctx) {
    match message {
        Message::VendorSelected(choice) => state.vendor = Some(choice),
        Message::ApAccountSelected(choice) => state.ap_account = Some(choice),
        Message::ExpenseAccountSelected(choice) => state.expense_account = Some(choice),
        Message::DescriptionChanged(value) => state.description_input = value,
        Message::AmountChanged(value) => state.amount_input = value,
        Message::SaveDraft => {
            state.message = None;
            state.error = None;

            let Some(vendor) = &state.vendor else {
                state.error = Some("choose a vendor".into());
                return;
            };
            let Some(expense_account) = &state.expense_account else {
                state.error = Some("choose an expense account".into());
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
                let Some(existing) = state.bills.iter().find(|b| b.id == Some(id)).cloned() else {
                    state.error = Some("that bill no longer exists".into());
                    state.editing_id = None;
                    return;
                };
                let bill = Bill {
                    vendor_id: vendor.id,
                    lines: vec![BillLine::new(
                        expense_account.id,
                        state.description_input.clone(),
                        Decimal::ONE,
                        amount,
                    )],
                    ..existing
                };
                let service = PurchaseService::new(ctx.db.clone());
                match service.update_bill(&bill) {
                    Ok(()) => {
                        if let Some(slot) = state.bills.iter_mut().find(|b| b.id == Some(id)) {
                            *slot = bill.clone();
                        }
                        state.editing_id = None;
                        state.vendor = None;
                        state.expense_account = None;
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
            let mut bill = Bill::new(ctx.company_id, vendor.id, today, due_date);
            bill.lines.push(BillLine::new(
                expense_account.id,
                state.description_input.clone(),
                Decimal::ONE,
                amount,
            ));

            let service = PurchaseService::new(ctx.db.clone());
            match service.create_bill(&bill) {
                Ok(created) => {
                    state.bills.push(created);
                    state.vendor = None;
                    state.expense_account = None;
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

            let (Some(vendor), Some(ap_account), Some(expense_account)) =
                (&state.vendor, &state.ap_account, &state.expense_account)
            else {
                state.error = Some("choose a vendor, AP account, and expense account".into());
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
            let mut bill = Bill::new(ctx.company_id, vendor.id, today, due_date);
            bill.lines.push(BillLine::new(
                expense_account.id,
                state.description_input.clone(),
                Decimal::ONE,
                amount,
            ));

            let service = PurchaseService::new(ctx.db.clone());
            let created = match service.create_bill(&bill) {
                Ok(bill) => bill,
                Err(err) => {
                    state.error = Some(err.to_string());
                    return;
                }
            };
            match service.post_bill(created.id.unwrap_or_default(), ap_account.id) {
                Ok((posted, _transaction, _payable)) => {
                    state.message =
                        Some(format!("Posted bill #{}.", posted.id.unwrap_or_default()));
                    state.bills.push(posted);
                    state.description_input.clear();
                    state.amount_input.clear();
                }
                Err(err) => state.error = Some(err.to_string()),
            }
        }
        Message::CancelEdit => {
            state.editing_id = None;
            state.vendor = None;
            state.expense_account = None;
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
            let service = PurchaseService::new(ctx.db.clone());
            let mut deleted = 0;
            let mut failures = Vec::new();
            for id in state.selected.clone() {
                match service.delete_bill(id) {
                    Ok(()) => {
                        deleted += 1;
                        state.bills.retain(|b| b.id != Some(id));
                        state.selected.remove(&id);
                    }
                    Err(err) => failures.push(format!("bill #{id}: {err}")),
                }
            }
            if !failures.is_empty() {
                state.error = Some(format!(
                    "Deleted {deleted}; {} could not be deleted -- {}",
                    failures.len(),
                    failures.join("; ")
                ));
            } else if deleted > 0 {
                state.message = Some(format!("Deleted {deleted} draft bill(s)."));
            }
        }
        Message::BulkExportCsv => {
            state.message = None;
            state.error = None;
            let selected: Vec<&Bill> = state
                .bills
                .iter()
                .filter(|b| state.selected.contains(&b.id.unwrap_or_default()))
                .collect();
            let headers = ["Bill", "Date", "Status", "Total"];
            let rows: Vec<Vec<String>> = selected
                .iter()
                .map(|b| {
                    vec![
                        b.id.map(|id| format!("BILL-{id}")).unwrap_or_default(),
                        b.bill_date.to_string(),
                        b.status.as_str().to_string(),
                        b.total().to_string(),
                    ]
                })
                .collect();
            let path = ctx.export_path("bills_export.csv");
            match std::fs::write(&path, csv::export_csv(&headers, &rows)) {
                Ok(()) => {
                    state.message = Some(format!(
                        "Exported {} bill(s) to {}",
                        selected.len(),
                        path.display()
                    ))
                }
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
        Message::PostForRow(bill_id) => {
            state.message = None;
            state.error = None;
            state.post_bill_id = Some(bill_id);
        }
        Message::PostApAccountSelected(choice) => state.post_ap_account = Some(choice),
        Message::SubmitPost => {
            state.message = None;
            state.error = None;

            let (Some(bill_id), Some(ap_account)) = (state.post_bill_id, &state.post_ap_account)
            else {
                state.error =
                    Some("choose a draft (via a row's Post button) and an AP account".into());
                return;
            };
            let service = PurchaseService::new(ctx.db.clone());
            match service.post_bill(bill_id, ap_account.id) {
                Ok((posted, _transaction, _payable)) => {
                    if let Some(slot) = state.bills.iter_mut().find(|b| b.id == Some(bill_id)) {
                        *slot = posted;
                    }
                    state.message = Some(format!("Posted BILL-{bill_id}."));
                    state.post_bill_id = None;
                    state.post_ap_account = None;
                }
                Err(err) => state.error = Some(err.to_string()),
            }
        }
        Message::KebabForRow(bill_id) => {
            state.message = None;
            state.error = None;
            let Some(bill) = state.bills.iter().find(|b| b.id == Some(bill_id)).cloned() else {
                return;
            };
            match bill.status {
                BillStatus::Draft => {
                    // `state.bills` only holds headers (no lines), so the
                    // full record is needed to prefill the form.
                    let service = PurchaseService::new(ctx.db.clone());
                    match service.get_bill(bill_id) {
                        Ok(full) => {
                            state.editing_id = Some(bill_id);
                            state.vendor = state
                                .vendors
                                .iter()
                                .find(|v| v.id == full.vendor_id)
                                .cloned();
                            let first_line = full.lines.first();
                            state.expense_account = first_line.and_then(|line| {
                                state
                                    .accounts
                                    .iter()
                                    .find(|a| a.id == line.account_id)
                                    .cloned()
                            });
                            state.description_input = first_line
                                .map(|line| line.description.clone())
                                .unwrap_or_default();
                            state.amount_input = first_line
                                .map(|line| line.amount().to_string())
                                .unwrap_or_default();
                        }
                        Err(err) => state.error = Some(err.to_string()),
                    }
                }
                BillStatus::Posted => {
                    let service = PurchaseService::new(ctx.db.clone());
                    state.debit_note_bill_id = Some(bill_id);
                    state.debit_note_amount_input = service
                        .bill_remaining_balance(bill_id)
                        .map(|balance| balance.to_string())
                        .unwrap_or_default();
                }
                BillStatus::Voided => {}
            }
        }
        Message::DebitNoteAccountSelected(choice) => state.debit_note_account = Some(choice),
        Message::DebitNoteApAccountSelected(choice) => state.debit_note_ap_account = Some(choice),
        Message::DebitNoteDescriptionChanged(value) => state.debit_note_description_input = value,
        Message::DebitNoteAmountChanged(value) => state.debit_note_amount_input = value,
        Message::SubmitDebitNote => {
            state.message = None;
            state.error = None;

            let (Some(bill_id), Some(account), Some(ap_account)) = (
                state.debit_note_bill_id,
                &state.debit_note_account,
                &state.debit_note_ap_account,
            ) else {
                state.error = Some(
                    "choose a bill (via a posted row's kebab menu), account, and AP account".into(),
                );
                return;
            };
            let amount: Decimal = match state.debit_note_amount_input.parse() {
                Ok(amount) => amount,
                Err(_) => {
                    state.error = Some("amount is not a valid number".into());
                    return;
                }
            };

            let today = Utc::now().date_naive();
            let service = PurchaseService::new(ctx.db.clone());
            match service.create_and_post_debit_note_for_bill(
                bill_id,
                today,
                account.id,
                state.debit_note_description_input.clone(),
                amount,
                ap_account.id,
            ) {
                Ok((debit_note, _transaction)) => {
                    state.message = Some(format!(
                        "Debit note #{} posted against BILL-{bill_id}.",
                        debit_note.id.unwrap_or_default()
                    ));
                    state.debit_note_description_input.clear();
                    state.debit_note_amount_input.clear();
                    state.debit_note_bill_id = None;
                }
                Err(err) => state.error = Some(err.to_string()),
            }
        }
        Message::PayForRow(bill_id) => {
            state.message = None;
            state.error = None;
            let service = PurchaseService::new(ctx.db.clone());
            match service.bill_remaining_balance(bill_id) {
                Ok(balance) => {
                    state.payment_bill_id = Some(bill_id);
                    state.payment_amount_input = balance.to_string();
                }
                Err(err) => {
                    state.payment_bill_id = None;
                    state.error = Some(err.to_string());
                }
            }
        }
        Message::PaymentApAccountSelected(choice) => state.payment_ap_account = Some(choice),
        Message::PaymentCashAccountSelected(choice) => state.payment_cash_account = Some(choice),
        Message::PaymentAmountChanged(value) => state.payment_amount_input = value,
        Message::SubmitPayment => {
            state.message = None;
            state.error = None;

            let (Some(bill_id), Some(ap_account), Some(cash_account)) = (
                state.payment_bill_id,
                &state.payment_ap_account,
                &state.payment_cash_account,
            ) else {
                state.error = Some(
                    "choose a bill (via a row's Pay button), AP account, and cash account".into(),
                );
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
            let service = PurchaseService::new(ctx.db.clone());
            match service.record_payment_for_bill(
                bill_id,
                today,
                amount,
                ap_account.id,
                cash_account.id,
            ) {
                Ok(recorded) => {
                    state.message = Some(format!(
                        "Recorded payment #{} against BILL-{bill_id}.",
                        recorded.id.unwrap_or_default()
                    ));
                    state.payment_amount_input.clear();
                    state.payment_bill_id = None;
                }
                Err(err) => state.error = Some(err.to_string()),
            }
        }
    }
}

pub fn view(state: &State, theme: Theme) -> Element<'_, Message> {
    let header = page_header::view(
        theme,
        "Bills",
        "Everything you owe vendors, in one place.",
        "Create & post",
        Message::CreateAndPost,
    );

    let toolbar = toolbar::view(
        theme,
        &["Draft", "Posted", "Voided"],
        state.active_tab,
        Message::TabSelected,
        &state.search,
        "Search bills",
        Message::SearchChanged,
        "Filters",
        Message::FiltersClicked,
        "Sort by",
        Message::SortClicked,
    );

    let heading = section_heading::view(theme, "Bill list", "Every bill on this company's books.");

    let tab_status = match state.active_tab {
        0 => BillStatus::Draft,
        1 => BillStatus::Posted,
        _ => BillStatus::Voided,
    };
    let query = state.search.trim().to_lowercase();
    let filtered: Vec<&Bill> = state
        .bills
        .iter()
        .filter(|bill| bill.status == tab_status)
        .filter(|bill| {
            query.is_empty()
                || bill
                    .id
                    .map(|id| format!("BILL-{id}"))
                    .unwrap_or_default()
                    .to_lowercase()
                    .contains(&query)
        })
        .collect();

    let columns = vec![
        ColumnDef::new("Bill", Length::FillPortion(1)).sorted(SortDirection::Descending),
        ColumnDef::new("Date", Length::FillPortion(1)),
        ColumnDef::new("Status", Length::FillPortion(1)),
        ColumnDef::new("Total", Length::FillPortion(1)),
    ];

    let rows = filtered
        .iter()
        .map(|bill| {
            let row = Row::new(vec![
                Cell::text(bill.id.map(|id| format!("BILL-{id}")).unwrap_or_default()),
                Cell::date(bill.bill_date.to_string()),
                Cell::status(bill.status.as_str(), status_tone(bill.status.as_str())),
                Cell::text(bill.total().to_string()),
            ]);
            match bill.status {
                BillStatus::Draft => row.with_primary_action("Post"),
                BillStatus::Posted => row.with_primary_action("Pay"),
                BillStatus::Voided => row,
            }
        })
        .collect();

    let bill_ids: Vec<i64> = filtered
        .iter()
        .map(|bill| bill.id.unwrap_or_default())
        .collect();
    let select_ids = bill_ids.clone();
    let selected_flags: Vec<bool> = bill_ids
        .iter()
        .map(|id| state.selected.contains(id))
        .collect();
    let kebab_ids = bill_ids.clone();
    let primary_ids = bill_ids.clone();
    let active_tab = state.active_tab;

    let table = data_table::view(
        theme,
        columns,
        rows,
        if active_tab == 0 {
            Some(selected_flags)
        } else {
            None
        },
        move |index, checked| Message::RowSelected(bill_ids[index], checked),
        move |checked| Message::SelectAllChanged(select_ids.clone(), checked),
        move |index| Message::KebabForRow(kebab_ids[index]),
        move |index| {
            let id = primary_ids[index];
            if active_tab == 0 {
                Message::PostForRow(id)
            } else {
                Message::PayForRow(id)
            }
        },
    );

    let form_heading = if state.editing_id.is_some() {
        "Edit draft bill"
    } else {
        "Create a bill"
    };
    let mut form = column![
        text(form_heading).size(typography::label()),
        pick_list(
            state.vendors.as_slice(),
            state.vendor.clone(),
            Message::VendorSelected
        )
        .style(dropdown::style(theme))
        .placeholder("Vendor"),
    ]
    .spacing(spacing::SM);

    if state.editing_id.is_none() {
        form = form.push(
            pick_list(
                state.accounts.as_slice(),
                state.ap_account.clone(),
                Message::ApAccountSelected,
            )
            .style(dropdown::style(theme))
            .placeholder("AP account (for Create & post)"),
        );
    }
    form = form.push(
        pick_list(
            state.accounts.as_slice(),
            state.expense_account.clone(),
            Message::ExpenseAccountSelected,
        )
        .style(dropdown::style(theme))
        .placeholder("Expense account"),
    );
    form = form.push(
        text_input("Description", &state.description_input)
            .style(inputs::text(theme))
            .on_input(Message::DescriptionChanged)
            .width(Length::Fill)
            .padding(spacing::SM),
    );
    form = form.push(
        text_input("Amount", &state.amount_input)
            .style(inputs::text(theme))
            .on_input(Message::AmountChanged)
            .width(Length::Fill)
            .padding(spacing::SM),
    );

    if state.editing_id.is_some() {
        form = form.push(
            iced::widget::row![
                button(text("Save changes"))
                    .style(button_style::primary(theme))
                    .on_press(Message::SaveDraft)
                    .padding(spacing::SM),
                button(text("Cancel"))
                    .style(button_style::secondary(theme))
                    .on_press(Message::CancelEdit)
                    .padding(spacing::SM),
            ]
            .spacing(spacing::SM),
        );
    } else {
        form = form.push(
            iced::widget::row![
                button(text("Save as draft"))
                    .style(button_style::secondary(theme))
                    .on_press(Message::SaveDraft)
                    .padding(spacing::SM),
                button(text("Create & post"))
                    .style(button_style::primary(theme))
                    .on_press(Message::CreateAndPost)
                    .padding(spacing::SM),
            ]
            .spacing(spacing::SM),
        );
    }

    let post_target_label = state
        .post_bill_id
        .and_then(|id| {
            state
                .bills
                .iter()
                .find(|bill| bill.id == Some(id))
                .map(|bill| (id, bill.vendor_id))
        })
        .map(|(id, vendor_id)| {
            let vendor_name = state
                .vendors
                .iter()
                .find(|v| v.id == vendor_id)
                .map(|v| v.label.clone())
                .unwrap_or_else(|| "unknown vendor".to_string());
            format!("BILL-{id} ({vendor_name})")
        })
        .unwrap_or_else(|| "Click a draft row's Post button to choose a bill".to_string());
    let post_form = column![
        text("Post this draft").size(typography::label()),
        text(format!("Bill: {post_target_label}"))
            .size(typography::body())
            .color(theme.neutral_front()),
        pick_list(
            state.accounts.as_slice(),
            state.post_ap_account.clone(),
            Message::PostApAccountSelected
        )
        .style(dropdown::style(theme))
        .placeholder("AP account"),
        button(text("Post"))
            .style(button_style::primary(theme))
            .on_press(Message::SubmitPost)
            .padding(spacing::SM),
    ]
    .spacing(spacing::SM);

    let payment_target_label = state
        .payment_bill_id
        .and_then(|id| {
            state
                .bills
                .iter()
                .find(|bill| bill.id == Some(id))
                .map(|bill| (id, bill.vendor_id))
        })
        .map(|(id, vendor_id)| {
            let vendor_name = state
                .vendors
                .iter()
                .find(|v| v.id == vendor_id)
                .map(|v| v.label.clone())
                .unwrap_or_else(|| "unknown vendor".to_string());
            format!("BILL-{id} ({vendor_name})")
        })
        .unwrap_or_else(|| "Click a row's Pay button to choose a bill".to_string());
    let payment_form = column![
        text("Pay a bill").size(typography::label()),
        text(format!("Bill: {payment_target_label}"))
            .size(typography::body())
            .color(theme.neutral_front()),
        pick_list(
            state.accounts.as_slice(),
            state.payment_ap_account.clone(),
            Message::PaymentApAccountSelected
        )
        .style(dropdown::style(theme))
        .placeholder("AP account"),
        pick_list(
            state.accounts.as_slice(),
            state.payment_cash_account.clone(),
            Message::PaymentCashAccountSelected
        )
        .style(dropdown::style(theme))
        .placeholder("Cash account"),
        text_input("Amount", &state.payment_amount_input)
            .style(inputs::text(theme))
            .on_input(Message::PaymentAmountChanged)
            .width(Length::Fill)
            .padding(spacing::SM),
        button(text("Record payment"))
            .style(button_style::primary(theme))
            .on_press(Message::SubmitPayment)
            .padding(spacing::SM),
    ]
    .spacing(spacing::SM);

    let debit_note_target_label = state
        .debit_note_bill_id
        .and_then(|id| {
            state
                .bills
                .iter()
                .find(|bill| bill.id == Some(id))
                .map(|bill| (id, bill.vendor_id))
        })
        .map(|(id, vendor_id)| {
            let vendor_name = state
                .vendors
                .iter()
                .find(|v| v.id == vendor_id)
                .map(|v| v.label.clone())
                .unwrap_or_else(|| "unknown vendor".to_string());
            format!("BILL-{id} ({vendor_name})")
        })
        .unwrap_or_else(|| "Click a posted row's kebab menu to choose a bill".to_string());
    let debit_note_form = column![
        text("Issue a debit note").size(typography::label()),
        text(format!("Bill: {debit_note_target_label}"))
            .size(typography::body())
            .color(theme.neutral_front()),
        pick_list(
            state.accounts.as_slice(),
            state.debit_note_account.clone(),
            Message::DebitNoteAccountSelected
        )
        .style(dropdown::style(theme))
        .placeholder("Account"),
        pick_list(
            state.accounts.as_slice(),
            state.debit_note_ap_account.clone(),
            Message::DebitNoteApAccountSelected
        )
        .style(dropdown::style(theme))
        .placeholder("AP account"),
        text_input("Description", &state.debit_note_description_input)
            .style(inputs::text(theme))
            .on_input(Message::DebitNoteDescriptionChanged)
            .width(Length::Fill)
            .padding(spacing::SM),
        text_input("Amount", &state.debit_note_amount_input)
            .style(inputs::text(theme))
            .on_input(Message::DebitNoteAmountChanged)
            .width(Length::Fill)
            .padding(spacing::SM),
        button(text("Issue debit note"))
            .style(button_style::primary(theme))
            .on_press(Message::SubmitDebitNote)
            .padding(spacing::SM),
    ]
    .spacing(spacing::SM);

    let mut content = column![header, toolbar, heading]
        .spacing(spacing::SPACE_20)
        .padding(spacing::LG)
        .width(Length::Fill);

    if active_tab == 0 && !state.selected.is_empty() {
        content = content.push(bulk_action_bar::view(
            theme,
            state.selected.len(),
            Message::BulkDelete,
            Message::BulkExportCsv,
            Message::ClearSelection,
        ));
    }

    content = content
        .push(table)
        .push(form)
        .push(post_form)
        .push(payment_form)
        .push(debit_note_form);

    if let Some(message) = &state.message {
        content = content.push(text(message.clone()).color(theme.positive_front()));
    }
    if let Some(error) = &state.error {
        content = content.push(text(error.clone()).color(theme.negative_front()));
    }

    container(scrollable(content).height(Length::Fill))
        .width(Length::Fill)
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::accounting::accounting_rules::AccountType;
    use crate::backend::accounting::fiscal_periods::FiscalPeriod;
    use crate::backend::organization::companies::Company;
    use crate::backend::parties::vendors::Vendor;
    use crate::backend::repositories::accounting_repository::{self, AccountingRepository};
    use crate::backend::repositories::organization_repository::{self, OrganizationRepository};
    use crate::backend::repositories::party_repository::{self, PartyRepository};
    use crate::backend::repositories::purchase_repository;
    use crate::database::database::Database;
    use chrono::NaiveDate;
    use std::path::PathBuf;

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    fn ctx_with_vendor_and_accounts() -> (Ctx, Choice, Choice, Choice) {
        let db = Database::open_in_memory().unwrap();
        db.run_migrations(organization_repository::MIGRATIONS)
            .unwrap();
        db.run_migrations(party_repository::MIGRATIONS).unwrap();
        db.run_migrations(accounting_repository::MIGRATIONS)
            .unwrap();
        db.run_migrations(crate::backend::audit::audit_logs::MIGRATIONS)
            .unwrap();
        db.run_migrations(purchase_repository::MIGRATIONS).unwrap();

        let org_repo = OrganizationRepository::new(db.clone());
        let company_id = org_repo
            .create_company(&Company::new("Acme Trading Co", "USD"))
            .unwrap()
            .id
            .unwrap();

        let party_repo = PartyRepository::new(db.clone());
        let vendor = party_repo
            .create_vendor(&Vendor::new(company_id, "Acme Supplies"))
            .unwrap();

        let accounting_repo = AccountingRepository::new(db.clone());
        accounting_repo
            .create_fiscal_period(&FiscalPeriod::new(
                company_id,
                "Open Period",
                date(2026, 1, 1),
                date(2030, 12, 31),
            ))
            .unwrap();
        let ap = accounting_repo
            .create_account(&Account::new(
                company_id,
                "2000",
                "Accounts Payable",
                AccountType::Liability,
            ))
            .unwrap();
        let expense = accounting_repo
            .create_account(&Account::new(
                company_id,
                "6000",
                "Office Supplies",
                AccountType::Expense,
            ))
            .unwrap();

        let ctx = Ctx {
            db,
            company_id,
            current_user_id: None,
            export_dir: PathBuf::from("."),
            uploads_dir: PathBuf::from("."),
        };
        let vendor_choice = Choice {
            id: vendor.id.unwrap(),
            label: vendor.name.clone(),
        };
        let ap_choice = Choice {
            id: ap.id.unwrap(),
            label: "2000 - Accounts Payable".into(),
        };
        let expense_choice = Choice {
            id: expense.id.unwrap(),
            label: "6000 - Office Supplies".into(),
        };
        (ctx, vendor_choice, ap_choice, expense_choice)
    }

    #[test]
    fn field_changed_messages_update_the_corresponding_field() {
        let (ctx, vendor, ap, expense) = ctx_with_vendor_and_accounts();
        let mut state = State::default();

        update(&mut state, Message::VendorSelected(vendor.clone()), &ctx);
        update(&mut state, Message::ApAccountSelected(ap.clone()), &ctx);
        update(
            &mut state,
            Message::ExpenseAccountSelected(expense.clone()),
            &ctx,
        );
        update(
            &mut state,
            Message::DescriptionChanged("Office supplies".into()),
            &ctx,
        );
        update(&mut state, Message::AmountChanged("450".into()), &ctx);

        assert_eq!(state.vendor, Some(vendor));
        assert_eq!(state.ap_account, Some(ap));
        assert_eq!(state.expense_account, Some(expense));
        assert_eq!(state.description_input, "Office supplies");
        assert_eq!(state.amount_input, "450");
    }

    #[test]
    fn creating_without_choosing_vendor_and_both_accounts_sets_an_error() {
        let (ctx, vendor, _ap, _expense) = ctx_with_vendor_and_accounts();
        let mut state = State {
            vendor: Some(vendor),
            ..Default::default()
        };

        update(&mut state, Message::CreateAndPost, &ctx);

        assert_eq!(
            state.error.as_deref(),
            Some("choose a vendor, AP account, and expense account")
        );
    }

    #[test]
    fn creating_with_a_non_numeric_amount_sets_an_error() {
        let (ctx, vendor, ap, expense) = ctx_with_vendor_and_accounts();
        let mut state = State {
            vendor: Some(vendor),
            ap_account: Some(ap),
            expense_account: Some(expense),
            amount_input: "not a number".into(),
            ..Default::default()
        };

        update(&mut state, Message::CreateAndPost, &ctx);

        assert_eq!(state.error.as_deref(), Some("amount is not a valid number"));
    }

    #[test]
    fn creating_and_posting_a_valid_bill_adds_it_to_the_list_and_clears_the_form() {
        let (ctx, vendor, ap, expense) = ctx_with_vendor_and_accounts();
        let mut state = State {
            vendor: Some(vendor),
            ap_account: Some(ap),
            expense_account: Some(expense),
            description_input: "Office supplies".into(),
            amount_input: "450".into(),
            ..Default::default()
        };

        update(&mut state, Message::CreateAndPost, &ctx);

        assert!(state.error.is_none());
        assert!(state
            .message
            .as_deref()
            .unwrap_or_default()
            .starts_with("Posted bill"));
        assert_eq!(state.bills.len(), 1);
        assert_eq!(state.bills[0].total(), rust_decimal_macros::dec!(450));
        assert!(state.description_input.is_empty());
        assert!(state.amount_input.is_empty());
    }

    #[test]
    fn search_changed_updates_the_search_field() {
        let (ctx, _vendor, _ap, _expense) = ctx_with_vendor_and_accounts();
        let mut state = State::default();

        update(&mut state, Message::SearchChanged("bill-1".into()), &ctx);

        assert_eq!(state.search, "bill-1");
    }

    /// Creates and posts a bill for `amount` and returns its id.
    fn create_and_post_bill(
        state: &mut State,
        ctx: &Ctx,
        vendor: &Choice,
        ap: &Choice,
        expense: &Choice,
        amount: &str,
    ) -> i64 {
        state.vendor = Some(vendor.clone());
        state.ap_account = Some(ap.clone());
        state.expense_account = Some(expense.clone());
        state.description_input = "Office supplies".into();
        state.amount_input = amount.into();
        update(state, Message::CreateAndPost, ctx);
        state.bills.last().unwrap().id.unwrap()
    }

    /// Saves a new draft (no posting) for `amount` and returns its id.
    fn save_draft_bill(
        state: &mut State,
        ctx: &Ctx,
        vendor: &Choice,
        expense: &Choice,
        amount: &str,
    ) -> i64 {
        state.vendor = Some(vendor.clone());
        state.expense_account = Some(expense.clone());
        state.description_input = "Office supplies".into();
        state.amount_input = amount.into();
        update(state, Message::SaveDraft, ctx);
        state.bills.last().unwrap().id.unwrap()
    }

    #[test]
    fn paying_from_a_row_prefills_the_amount_with_that_bill_s_own_balance() {
        let (ctx, vendor, ap, expense) = ctx_with_vendor_and_accounts();
        let mut state = State {
            vendors: vec![vendor.clone()],
            ..Default::default()
        };
        let bill_id = create_and_post_bill(&mut state, &ctx, &vendor, &ap, &expense, "450");

        update(&mut state, Message::PayForRow(bill_id), &ctx);

        assert_eq!(state.payment_bill_id, Some(bill_id));
        assert_eq!(state.payment_amount_input, "450");
        assert!(state.error.is_none());
    }

    #[test]
    fn submitting_a_payment_without_choosing_a_bill_and_both_accounts_sets_an_error() {
        let (ctx, _vendor, _ap, _expense) = ctx_with_vendor_and_accounts();
        let mut state = State::default();

        update(&mut state, Message::SubmitPayment, &ctx);

        assert_eq!(
            state.error.as_deref(),
            Some("choose a bill (via a row's Pay button), AP account, and cash account")
        );
    }

    #[test]
    fn submitting_a_payment_with_a_non_numeric_amount_sets_an_error() {
        let (ctx, vendor, ap, expense) = ctx_with_vendor_and_accounts();
        let cash = ap.clone();
        let mut state = State {
            vendors: vec![vendor.clone()],
            ..Default::default()
        };
        let bill_id = create_and_post_bill(&mut state, &ctx, &vendor, &ap, &expense, "450");
        state.payment_bill_id = Some(bill_id);
        state.payment_ap_account = Some(ap);
        state.payment_cash_account = Some(cash);
        state.payment_amount_input = "not a number".into();

        update(&mut state, Message::SubmitPayment, &ctx);

        assert_eq!(state.error.as_deref(), Some("amount is not a valid number"));
    }

    #[test]
    fn a_payment_exactly_equal_to_the_bill_s_balance_fully_settles_it() {
        let (ctx, vendor, ap, expense) = ctx_with_vendor_and_accounts();
        let cash = ap.clone();
        let mut state = State {
            vendors: vec![vendor.clone()],
            ..Default::default()
        };
        let bill_id = create_and_post_bill(&mut state, &ctx, &vendor, &ap, &expense, "450");

        update(&mut state, Message::PayForRow(bill_id), &ctx);
        update(&mut state, Message::PaymentApAccountSelected(ap), &ctx);
        update(&mut state, Message::PaymentCashAccountSelected(cash), &ctx);
        update(&mut state, Message::SubmitPayment, &ctx);

        assert!(state.error.is_none());
        assert!(state
            .message
            .as_deref()
            .unwrap_or_default()
            .contains(&format!("BILL-{bill_id}")));
        assert!(state.payment_bill_id.is_none());

        let service = PurchaseService::new(ctx.db.clone());
        assert!(
            service.bill_remaining_balance(bill_id).is_err(),
            "a fully-settled bill has no open payable left"
        );
    }

    #[test]
    fn a_partial_payment_reduces_only_that_bill_s_balance() {
        let (ctx, vendor, ap, expense) = ctx_with_vendor_and_accounts();
        let cash = ap.clone();
        let mut state = State {
            vendors: vec![vendor.clone()],
            ..Default::default()
        };
        let bill_id = create_and_post_bill(&mut state, &ctx, &vendor, &ap, &expense, "450");

        update(&mut state, Message::PayForRow(bill_id), &ctx);
        state.payment_amount_input = "200".into();
        update(&mut state, Message::PaymentApAccountSelected(ap), &ctx);
        update(&mut state, Message::PaymentCashAccountSelected(cash), &ctx);
        update(&mut state, Message::SubmitPayment, &ctx);

        assert!(state.error.is_none());
        let service = PurchaseService::new(ctx.db.clone());
        assert_eq!(
            service.bill_remaining_balance(bill_id).unwrap(),
            rust_decimal_macros::dec!(250)
        );
    }

    #[test]
    fn an_overpayment_attempt_beyond_the_bill_s_own_balance_is_rejected() {
        let (ctx, vendor, ap, expense) = ctx_with_vendor_and_accounts();
        let cash = ap.clone();
        let mut state = State {
            vendors: vec![vendor.clone()],
            ..Default::default()
        };
        let bill_id = create_and_post_bill(&mut state, &ctx, &vendor, &ap, &expense, "450");

        update(&mut state, Message::PayForRow(bill_id), &ctx);
        state.payment_amount_input = "600".into();
        update(&mut state, Message::PaymentApAccountSelected(ap), &ctx);
        update(&mut state, Message::PaymentCashAccountSelected(cash), &ctx);
        update(&mut state, Message::SubmitPayment, &ctx);

        assert!(state
            .error
            .as_deref()
            .unwrap_or_default()
            .contains("exceeds"));
        assert!(state.message.is_none());

        let service = PurchaseService::new(ctx.db.clone());
        assert_eq!(
            service.bill_remaining_balance(bill_id).unwrap(),
            rust_decimal_macros::dec!(450),
            "a rejected overpayment must not touch the bill's balance at all"
        );
    }

    /// Regression test for the exact bug this feature replaces: paying
    /// one bill via its row must never touch a *different* open bill for
    /// the same vendor, even partially.
    #[test]
    fn paying_one_bill_via_its_row_leaves_the_vendor_s_other_open_bill_completely_untouched() {
        let (ctx, vendor, ap, expense) = ctx_with_vendor_and_accounts();
        let cash = ap.clone();
        let mut state = State {
            vendors: vec![vendor.clone()],
            ..Default::default()
        };
        let first_bill_id = create_and_post_bill(&mut state, &ctx, &vendor, &ap, &expense, "450");
        let second_bill_id = create_and_post_bill(&mut state, &ctx, &vendor, &ap, &expense, "300");

        update(&mut state, Message::PayForRow(first_bill_id), &ctx);
        update(&mut state, Message::PaymentApAccountSelected(ap), &ctx);
        update(&mut state, Message::PaymentCashAccountSelected(cash), &ctx);
        update(&mut state, Message::SubmitPayment, &ctx);

        assert!(state.error.is_none());

        let service = PurchaseService::new(ctx.db.clone());
        assert!(
            service.bill_remaining_balance(first_bill_id).is_err(),
            "the paid bill should be fully settled"
        );
        assert_eq!(
            service.bill_remaining_balance(second_bill_id).unwrap(),
            rust_decimal_macros::dec!(300),
            "the vendor's other open bill must be completely untouched"
        );
    }

    #[test]
    fn tab_selected_updates_the_active_tab() {
        let (ctx, _vendor, _ap, _expense) = ctx_with_vendor_and_accounts();
        let mut state = State::default();

        update(&mut state, Message::TabSelected(2), &ctx);

        assert_eq!(state.active_tab, 2);
    }

    #[test]
    fn kebab_on_a_voided_bill_is_a_no_op() {
        let (ctx, vendor, _ap, expense) = ctx_with_vendor_and_accounts();
        let service = PurchaseService::new(ctx.db.clone());
        let mut bill = Bill::new(
            ctx.company_id,
            vendor.id,
            date(2026, 1, 5),
            date(2026, 2, 4),
        );
        bill.lines.push(BillLine::new(
            expense.id,
            "Office supplies",
            rust_decimal_macros::dec!(1),
            rust_decimal_macros::dec!(500),
        ));
        let draft = service.create_bill(&bill).unwrap();
        let voided = service.void_bill(draft.id.unwrap()).unwrap();
        let mut state = State {
            bills: vec![voided.clone()],
            ..Default::default()
        };

        update(&mut state, Message::KebabForRow(voided.id.unwrap()), &ctx);

        assert!(state.error.is_none());
        assert!(state.editing_id.is_none());
        assert!(state.debit_note_bill_id.is_none());
        assert_eq!(state.bills[0].status, BillStatus::Voided);
    }

    #[test]
    fn kebab_on_a_posted_bill_opens_the_debit_note_form_prefilled_with_its_own_balance() {
        let (ctx, vendor, ap, expense) = ctx_with_vendor_and_accounts();
        let mut state = State {
            vendors: vec![vendor.clone()],
            ..Default::default()
        };
        let bill_id = create_and_post_bill(&mut state, &ctx, &vendor, &ap, &expense, "750");

        update(&mut state, Message::KebabForRow(bill_id), &ctx);

        assert_eq!(state.debit_note_bill_id, Some(bill_id));
        assert_eq!(state.debit_note_amount_input, "750");
    }

    #[test]
    fn submitting_a_debit_note_without_choosing_a_bill_and_both_accounts_sets_an_error() {
        let (ctx, _vendor, _ap, _expense) = ctx_with_vendor_and_accounts();
        let mut state = State::default();

        update(&mut state, Message::SubmitDebitNote, &ctx);

        assert_eq!(
            state.error.as_deref(),
            Some("choose a bill (via a posted row's kebab menu), account, and AP account")
        );
    }

    #[test]
    fn a_debit_note_exactly_equal_to_the_bill_s_balance_fully_settles_it() {
        let (ctx, vendor, ap, expense) = ctx_with_vendor_and_accounts();
        let mut state = State {
            vendors: vec![vendor.clone()],
            ..Default::default()
        };
        let bill_id = create_and_post_bill(&mut state, &ctx, &vendor, &ap, &expense, "450");

        update(&mut state, Message::KebabForRow(bill_id), &ctx);
        update(&mut state, Message::DebitNoteAccountSelected(expense), &ctx);
        update(&mut state, Message::DebitNoteApAccountSelected(ap), &ctx);
        state.debit_note_description_input = "Damaged goods return".into();
        update(&mut state, Message::SubmitDebitNote, &ctx);

        assert!(state.error.is_none());
        assert!(state
            .message
            .as_deref()
            .unwrap_or_default()
            .starts_with("Debit note"));
        assert!(state.debit_note_bill_id.is_none());

        let service = PurchaseService::new(ctx.db.clone());
        assert!(
            service.bill_remaining_balance(bill_id).is_err(),
            "a fully-debited bill has no open payable left"
        );
    }

    #[test]
    fn a_debit_note_amount_exceeding_the_bill_s_own_balance_is_rejected() {
        let (ctx, vendor, ap, expense) = ctx_with_vendor_and_accounts();
        let mut state = State {
            vendors: vec![vendor.clone()],
            ..Default::default()
        };
        let bill_id = create_and_post_bill(&mut state, &ctx, &vendor, &ap, &expense, "450");

        update(&mut state, Message::KebabForRow(bill_id), &ctx);
        state.debit_note_amount_input = "600".into();
        update(&mut state, Message::DebitNoteAccountSelected(expense), &ctx);
        update(&mut state, Message::DebitNoteApAccountSelected(ap), &ctx);
        state.debit_note_description_input = "Damaged goods return".into();
        update(&mut state, Message::SubmitDebitNote, &ctx);

        assert!(state
            .error
            .as_deref()
            .unwrap_or_default()
            .contains("exceeds"));
        assert!(state.message.is_none());

        let service = PurchaseService::new(ctx.db.clone());
        assert_eq!(
            service.bill_remaining_balance(bill_id).unwrap(),
            rust_decimal_macros::dec!(450),
            "a rejected debit note must not touch the bill's balance at all"
        );
    }

    /// Same regression shape as the payment-scoping test above, now for
    /// debit notes: issuing one against a single bill must never touch a
    /// *different* open bill for the same vendor.
    #[test]
    fn issuing_a_debit_note_against_one_bill_leaves_the_vendor_s_other_open_bill_completely_untouched(
    ) {
        let (ctx, vendor, ap, expense) = ctx_with_vendor_and_accounts();
        let mut state = State {
            vendors: vec![vendor.clone()],
            ..Default::default()
        };
        let first_bill_id = create_and_post_bill(&mut state, &ctx, &vendor, &ap, &expense, "450");
        let second_bill_id = create_and_post_bill(&mut state, &ctx, &vendor, &ap, &expense, "300");

        update(&mut state, Message::KebabForRow(first_bill_id), &ctx);
        update(&mut state, Message::DebitNoteAccountSelected(expense), &ctx);
        update(&mut state, Message::DebitNoteApAccountSelected(ap), &ctx);
        state.debit_note_description_input = "Damaged goods return".into();
        update(&mut state, Message::SubmitDebitNote, &ctx);

        assert!(state.error.is_none());

        let service = PurchaseService::new(ctx.db.clone());
        assert!(
            service.bill_remaining_balance(first_bill_id).is_err(),
            "the debited bill should be fully settled"
        );
        assert_eq!(
            service.bill_remaining_balance(second_bill_id).unwrap(),
            rust_decimal_macros::dec!(300),
            "the vendor's other open bill must be completely untouched"
        );
    }

    #[test]
    fn saving_a_new_draft_without_choosing_a_vendor_sets_an_error() {
        let (ctx, _vendor, _ap, expense) = ctx_with_vendor_and_accounts();
        let mut state = State {
            expense_account: Some(expense),
            amount_input: "500".into(),
            ..Default::default()
        };

        update(&mut state, Message::SaveDraft, &ctx);

        assert_eq!(state.error.as_deref(), Some("choose a vendor"));
        assert!(state.bills.is_empty());
    }

    #[test]
    fn saving_a_new_draft_adds_it_to_the_list_as_draft_and_clears_the_form() {
        let (ctx, vendor, _ap, expense) = ctx_with_vendor_and_accounts();
        let mut state = State {
            vendor: Some(vendor),
            expense_account: Some(expense),
            description_input: "Office supplies".into(),
            amount_input: "500".into(),
            ..Default::default()
        };

        update(&mut state, Message::SaveDraft, &ctx);

        assert!(state.error.is_none());
        assert_eq!(state.bills.len(), 1);
        assert_eq!(state.bills[0].status, BillStatus::Draft);
        assert_eq!(state.bills[0].total(), rust_decimal_macros::dec!(500));
        assert!(state.vendor.is_none());
        assert!(state.description_input.is_empty());
    }

    #[test]
    fn kebab_on_a_draft_bill_opens_the_edit_form_prefilled() {
        let (ctx, vendor, _ap, expense) = ctx_with_vendor_and_accounts();
        let mut state = State {
            vendors: vec![vendor.clone()],
            accounts: vec![expense.clone()],
            ..Default::default()
        };
        let bill_id = save_draft_bill(&mut state, &ctx, &vendor, &expense, "500");

        update(&mut state, Message::KebabForRow(bill_id), &ctx);

        assert_eq!(state.editing_id, Some(bill_id));
        assert_eq!(state.vendor, Some(vendor));
        assert_eq!(state.expense_account, Some(expense));
        assert_eq!(state.description_input, "Office supplies");
        assert_eq!(state.amount_input, "500");
    }

    #[test]
    fn cancel_edit_clears_editing_state_without_touching_the_stored_draft() {
        let (ctx, vendor, _ap, expense) = ctx_with_vendor_and_accounts();
        let mut state = State {
            vendors: vec![vendor.clone()],
            accounts: vec![expense.clone()],
            ..Default::default()
        };
        let bill_id = save_draft_bill(&mut state, &ctx, &vendor, &expense, "500");

        update(&mut state, Message::KebabForRow(bill_id), &ctx);
        state.description_input = "Should Not Save".into();
        update(&mut state, Message::CancelEdit, &ctx);

        assert!(state.editing_id.is_none());
        assert!(state.description_input.is_empty());

        let service = PurchaseService::new(ctx.db.clone());
        let reloaded = service.get_bill(bill_id).unwrap();
        assert_eq!(reloaded.lines[0].description, "Office supplies");
    }

    #[test]
    fn saving_changes_to_a_draft_updates_it_in_place_and_clears_editing_state() {
        let (ctx, vendor, _ap, expense) = ctx_with_vendor_and_accounts();
        let mut state = State {
            vendors: vec![vendor.clone()],
            accounts: vec![expense.clone()],
            ..Default::default()
        };
        let bill_id = save_draft_bill(&mut state, &ctx, &vendor, &expense, "500");

        update(&mut state, Message::KebabForRow(bill_id), &ctx);
        state.description_input = "Revised office supplies".into();
        state.amount_input = "800".into();
        update(&mut state, Message::SaveDraft, &ctx);

        assert!(state.error.is_none());
        assert!(state.editing_id.is_none());
        assert_eq!(state.bills.len(), 1);
        assert_eq!(state.bills[0].total(), rust_decimal_macros::dec!(800));

        let service = PurchaseService::new(ctx.db.clone());
        let reloaded = service.get_bill(bill_id).unwrap();
        assert_eq!(reloaded.lines[0].description, "Revised office supplies");
    }

    #[test]
    fn row_selected_toggles_membership_in_the_selection() {
        let (ctx, _vendor, _ap, _expense) = ctx_with_vendor_and_accounts();
        let mut state = State::default();

        update(&mut state, Message::RowSelected(1, true), &ctx);
        update(&mut state, Message::RowSelected(2, true), &ctx);
        assert_eq!(state.selected, HashSet::from([1, 2]));

        update(&mut state, Message::RowSelected(1, false), &ctx);
        assert_eq!(state.selected, HashSet::from([2]));
    }

    #[test]
    fn select_all_changed_selects_or_clears_every_given_id() {
        let (ctx, _vendor, _ap, _expense) = ctx_with_vendor_and_accounts();
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
        let (ctx, _vendor, _ap, _expense) = ctx_with_vendor_and_accounts();
        let mut state = State {
            selected: HashSet::from([1, 2, 3]),
            ..Default::default()
        };

        update(&mut state, Message::ClearSelection, &ctx);

        assert!(state.selected.is_empty());
    }

    #[test]
    fn bulk_delete_removes_every_selected_draft_bill() {
        let (ctx, vendor, _ap, expense) = ctx_with_vendor_and_accounts();
        let mut state = State {
            vendors: vec![vendor.clone()],
            ..Default::default()
        };
        save_draft_bill(&mut state, &ctx, &vendor, &expense, "500");
        save_draft_bill(&mut state, &ctx, &vendor, &expense, "300");
        let ids: Vec<i64> = state.bills.iter().map(|b| b.id.unwrap()).collect();
        state.selected = ids.iter().copied().collect();

        update(&mut state, Message::BulkDelete, &ctx);

        assert!(state.error.is_none());
        assert!(state.bills.is_empty());
        assert!(state.selected.is_empty());
        assert!(state
            .message
            .as_deref()
            .unwrap_or_default()
            .contains("Deleted 2"));
    }

    #[test]
    fn bulk_export_csv_writes_only_the_selected_bills() {
        let (ctx, vendor, _ap, expense) = ctx_with_vendor_and_accounts();
        let ctx = Ctx {
            export_dir: std::env::temp_dir().join(format!(
                "memonbooks_bills_view_test_{:?}",
                std::thread::current().id()
            )),
            ..ctx
        };
        let mut state = State {
            vendors: vec![vendor.clone()],
            ..Default::default()
        };
        let export_id = save_draft_bill(&mut state, &ctx, &vendor, &expense, "500");
        save_draft_bill(&mut state, &ctx, &vendor, &expense, "300");
        state.selected = HashSet::from([export_id]);

        update(&mut state, Message::BulkExportCsv, &ctx);

        assert!(state.error.is_none());
        let contents = std::fs::read_to_string(ctx.export_path("bills_export.csv")).unwrap();
        assert!(contents.contains(&format!("BILL-{export_id}")));

        let _ = std::fs::remove_dir_all(&ctx.export_dir);
    }

    #[test]
    fn post_for_row_sets_the_target_bill() {
        let (ctx, vendor, _ap, expense) = ctx_with_vendor_and_accounts();
        let mut state = State {
            vendors: vec![vendor.clone()],
            ..Default::default()
        };
        let bill_id = save_draft_bill(&mut state, &ctx, &vendor, &expense, "500");

        update(&mut state, Message::PostForRow(bill_id), &ctx);

        assert_eq!(state.post_bill_id, Some(bill_id));
    }

    #[test]
    fn submitting_post_without_choosing_a_draft_and_an_ap_account_sets_an_error() {
        let (ctx, _vendor, _ap, _expense) = ctx_with_vendor_and_accounts();
        let mut state = State::default();

        update(&mut state, Message::SubmitPost, &ctx);

        assert_eq!(
            state.error.as_deref(),
            Some("choose a draft (via a row's Post button) and an AP account")
        );
    }

    #[test]
    fn posting_a_draft_via_its_post_button_moves_it_to_posted_and_creates_a_payable() {
        let (ctx, vendor, ap, expense) = ctx_with_vendor_and_accounts();
        let mut state = State {
            vendors: vec![vendor.clone()],
            ..Default::default()
        };
        let bill_id = save_draft_bill(&mut state, &ctx, &vendor, &expense, "500");

        update(&mut state, Message::PostForRow(bill_id), &ctx);
        update(&mut state, Message::PostApAccountSelected(ap), &ctx);
        update(&mut state, Message::SubmitPost, &ctx);

        assert!(state.error.is_none());
        assert_eq!(state.bills[0].status, BillStatus::Posted);
        let service = PurchaseService::new(ctx.db.clone());
        assert_eq!(
            service.bill_remaining_balance(bill_id).unwrap(),
            rust_decimal_macros::dec!(500)
        );
    }

    /// The exact scenario requested when this Draft/Edit/Post pattern
    /// was added for Invoices, applied here from day one: create as
    /// Draft, edit it, edit it again, then Post -- the final Posted
    /// record must reflect the *last* edit, not the original draft
    /// values or the first edit.
    #[test]
    fn creating_a_draft_editing_it_twice_then_posting_reflects_the_last_edit_not_the_original() {
        let (ctx, vendor, ap, expense) = ctx_with_vendor_and_accounts();
        let mut state = State {
            vendors: vec![vendor.clone()],
            accounts: vec![ap.clone(), expense.clone()],
            ..Default::default()
        };

        let bill_id = save_draft_bill(&mut state, &ctx, &vendor, &expense, "1000");
        assert_eq!(state.bills[0].status, BillStatus::Draft);

        update(&mut state, Message::KebabForRow(bill_id), &ctx);
        assert_eq!(state.editing_id, Some(bill_id));
        state.description_input = "Revised office supplies".into();
        state.amount_input = "1200".into();
        update(&mut state, Message::SaveDraft, &ctx);
        assert!(state.error.is_none());
        assert!(state.editing_id.is_none());

        update(&mut state, Message::KebabForRow(bill_id), &ctx);
        assert_eq!(state.description_input, "Revised office supplies");
        assert_eq!(state.amount_input, "1200");
        state.description_input = "Final office supplies".into();
        state.amount_input = "1500".into();
        update(&mut state, Message::SaveDraft, &ctx);
        assert!(state.error.is_none());

        update(&mut state, Message::PostForRow(bill_id), &ctx);
        update(&mut state, Message::PostApAccountSelected(ap), &ctx);
        update(&mut state, Message::SubmitPost, &ctx);

        assert!(state.error.is_none());
        let posted = state.bills.iter().find(|b| b.id == Some(bill_id)).unwrap();
        assert_eq!(posted.status, BillStatus::Posted);
        assert_eq!(posted.total(), rust_decimal_macros::dec!(1500));

        let service = PurchaseService::new(ctx.db.clone());
        let reloaded = service.get_bill(bill_id).unwrap();
        assert_eq!(reloaded.lines[0].description, "Final office supplies");
        assert_eq!(reloaded.total(), rust_decimal_macros::dec!(1500));
        assert_ne!(reloaded.lines[0].description, "Original office supplies");
    }
}
