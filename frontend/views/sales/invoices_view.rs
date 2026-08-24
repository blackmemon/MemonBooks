//! List invoices and create + post a simple single-line invoice.

use chrono::{Days, Utc};
use rust_decimal::Decimal;

use iced::widget::{button, column, container, pick_list, scrollable, text, text_input};
use iced::{Element, Length};

use crate::backend::accounting::chart_of_accounts::Account;
use crate::backend::sales::invoices::{Invoice, InvoiceLine};
use crate::backend::sales::payments::CustomerPayment;
use crate::backend::services::sales_service::SalesService;
use crate::frontend::components::common::data_table::{self, Cell, ColumnDef, Row, SortDirection};
use crate::frontend::components::common::{page_header, section_heading, toolbar};
use crate::frontend::styles::badges::BadgeTone;
use crate::frontend::styles::colors;
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
    ar_account: Option<Choice>,
    revenue_account: Option<Choice>,
    description_input: String,
    amount_input: String,
    search: String,
    payment_customer: Option<Choice>,
    payment_cash_account: Option<Choice>,
    payment_ar_account: Option<Choice>,
    payment_amount_input: String,
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
    CreateAndPost,
    SearchChanged(String),
    TabSelected(usize),
    FiltersClicked,
    SortClicked,
    RowAction(usize),
    /// Fired by an invoice row's "Record Payment" button, carrying that
    /// invoice's customer id -- pre-selects the customer below. Payment
    /// still applies FIFO across the customer's whole open balance, not
    /// just the row that was clicked (see `SalesService::
    /// record_customer_payment`), so this only seeds the form.
    RecordPaymentForRow(i64),
    PaymentCashAccountSelected(Choice),
    PaymentArAccountSelected(Choice),
    PaymentAmountChanged(String),
    SubmitPayment,
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
        Message::SearchChanged(value) => state.search = value,
        // This view only has one tab, and "Filters" / "Sort by" / the
        // per-row kebab menu are rendered by the shared toolbar/data
        // table components now (Section 6) -- wiring real tab/filter/
        // sort/action behavior is follow-up work, not part of this
        // layout pass.
        Message::TabSelected(_) | Message::FiltersClicked | Message::SortClicked | Message::RowAction(_) => {}
        Message::RecordPaymentForRow(customer_id) => {
            state.payment_customer = state.customers.iter().find(|c| c.id == customer_id).cloned();
            state.message = None;
            state.error = None;
        }
        Message::PaymentCashAccountSelected(choice) => state.payment_cash_account = Some(choice),
        Message::PaymentArAccountSelected(choice) => state.payment_ar_account = Some(choice),
        Message::PaymentAmountChanged(value) => state.payment_amount_input = value,
        Message::SubmitPayment => {
            state.message = None;
            state.error = None;

            let (Some(customer), Some(cash_account), Some(ar_account)) = (&state.payment_customer, &state.payment_cash_account, &state.payment_ar_account) else {
                state.error = Some("choose a customer (via a row's Record Payment button), cash account, and AR account".into());
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
            let payment = CustomerPayment::new(ctx.company_id, customer.id, today, amount);
            let service = SalesService::new(ctx.db.clone());
            match service.record_customer_payment(&payment, cash_account.id, ar_account.id) {
                Ok(recorded) => {
                    state.message = Some(format!("Recorded payment #{} for {}.", recorded.id.unwrap_or_default(), customer.label));
                    state.payment_amount_input.clear();
                }
                Err(err) => state.error = Some(err.to_string()),
            }
        }
    }
}

pub fn view(state: &State) -> Element<'_, Message> {
    let header = page_header::view("Invoices", "Everything you've billed customers, in one place.", "Create & post", Message::CreateAndPost);

    let toolbar = toolbar::view(
        &["All Invoices"],
        0,
        Message::TabSelected,
        &state.search,
        "Search invoices",
        Message::SearchChanged,
        "Filters",
        Message::FiltersClicked,
        "Sort by",
        Message::SortClicked,
    );

    let heading = section_heading::view("Invoice list", "Every invoice on this company's books.");

    let query = state.search.trim().to_lowercase();
    let filtered: Vec<&Invoice> = state
        .invoices
        .iter()
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
            Row::new(vec![
                Cell::text(invoice.id.map(|id| format!("INV-{id}")).unwrap_or_default()),
                Cell::date(invoice.invoice_date.to_string()),
                Cell::status(invoice.status.as_str(), status_tone(invoice.status.as_str())),
                Cell::text(invoice.total().to_string()),
            ])
            .with_primary_action("Record Payment")
        })
        .collect();

    let customer_ids: Vec<i64> = filtered.iter().map(|invoice| invoice.customer_id).collect();
    let table = data_table::view(columns, rows, None, |_index, _checked| Message::RowAction(0), Message::RowAction, move |index| Message::RecordPaymentForRow(customer_ids[index]));

    let form = column![
        text("Create & post an invoice").size(13),
        pick_list(state.customers.as_slice(), state.customer.clone(), Message::CustomerSelected).placeholder("Customer"),
        pick_list(state.accounts.as_slice(), state.ar_account.clone(), Message::ArAccountSelected).placeholder("AR account"),
        pick_list(state.accounts.as_slice(), state.revenue_account.clone(), Message::RevenueAccountSelected).placeholder("Revenue account"),
        text_input("Description", &state.description_input).on_input(Message::DescriptionChanged).width(Length::Fill).padding(8),
        text_input("Amount", &state.amount_input).on_input(Message::AmountChanged).width(Length::Fill).padding(8),
        button(text("Create & post")).on_press(Message::CreateAndPost).padding(8),
    ]
    .spacing(8);

    let payment_customer_label = state.payment_customer.as_ref().map(|c| c.label.clone()).unwrap_or_else(|| "Click a row's Record Payment button to choose a customer".to_string());
    let payment_form = column![
        text("Record a payment").size(13),
        text(format!("Customer: {payment_customer_label}")).size(12).color(colors::WHITE_NEUTRAL_FRONT),
        pick_list(state.accounts.as_slice(), state.payment_cash_account.clone(), Message::PaymentCashAccountSelected).placeholder("Cash account"),
        pick_list(state.accounts.as_slice(), state.payment_ar_account.clone(), Message::PaymentArAccountSelected).placeholder("AR account"),
        text_input("Amount", &state.payment_amount_input).on_input(Message::PaymentAmountChanged).width(Length::Fill).padding(8),
        button(text("Record payment")).on_press(Message::SubmitPayment).padding(8),
    ]
    .spacing(8);

    let mut content = column![header, toolbar, heading, table, form, payment_form].spacing(20).padding(24).width(Length::Fill);

    if let Some(message) = &state.message {
        content = content.push(text(message.clone()).color(colors::WHITE_POSITIVE_FRONT));
    }
    if let Some(error) = &state.error {
        content = content.push(text(error.clone()).color(colors::WHITE_NEGATIVE_FRONT));
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

    #[test]
    fn recording_a_row_s_payment_preselects_that_invoice_s_customer() {
        let (ctx, customer, _ar, _revenue) = ctx_with_customer_and_accounts();
        let mut state = State { customers: vec![customer.clone()], ..Default::default() };

        update(&mut state, Message::RecordPaymentForRow(customer.id), &ctx);

        assert_eq!(state.payment_customer, Some(customer));
    }

    #[test]
    fn submitting_a_payment_without_choosing_a_customer_and_both_accounts_sets_an_error() {
        let (ctx, _customer, _ar, _revenue) = ctx_with_customer_and_accounts();
        let mut state = State::default();

        update(&mut state, Message::SubmitPayment, &ctx);

        assert_eq!(state.error.as_deref(), Some("choose a customer (via a row's Record Payment button), cash account, and AR account"));
    }

    #[test]
    fn submitting_a_payment_with_a_non_numeric_amount_sets_an_error() {
        let (ctx, customer, ar, _revenue) = ctx_with_customer_and_accounts();
        let cash = ar.clone();
        let mut state = State {
            customers: vec![customer.clone()],
            payment_customer: Some(customer),
            payment_cash_account: Some(cash),
            payment_ar_account: Some(ar),
            payment_amount_input: "not a number".into(),
            ..Default::default()
        };

        update(&mut state, Message::SubmitPayment, &ctx);

        assert_eq!(state.error.as_deref(), Some("amount is not a valid number"));
    }

    #[test]
    fn submitting_a_valid_payment_that_exceeds_the_open_balance_is_rejected() {
        let (ctx, customer, ar, _revenue) = ctx_with_customer_and_accounts();
        let cash = ar.clone();
        let mut state = State {
            customers: vec![customer.clone()],
            payment_customer: Some(customer),
            payment_cash_account: Some(cash),
            payment_ar_account: Some(ar),
            payment_amount_input: "500".into(),
            ..Default::default()
        };

        // No posted invoice exists yet for this customer, so any positive
        // payment exceeds their (zero) open balance -- proves the amount
        // actually reaches SalesService::record_customer_payment rather
        // than silently succeeding.
        update(&mut state, Message::SubmitPayment, &ctx);

        assert!(state.error.as_deref().unwrap_or_default().contains("exceeds"));
        assert!(state.message.is_none());
    }
}
