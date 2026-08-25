//! List invoices and create + post a simple single-line invoice.

use chrono::{Days, Utc};
use rust_decimal::Decimal;

use iced::widget::{button, column, container, pick_list, scrollable, text, text_input};
use iced::{Element, Length};

use crate::backend::accounting::chart_of_accounts::Account;
use crate::backend::sales::invoices::{Invoice, InvoiceLine};
use crate::backend::services::sales_service::SalesService;
use crate::frontend::components::common::data_table::{self, Cell, ColumnDef, Row, SortDirection};
use crate::frontend::components::common::{page_header, section_heading, toolbar};
use crate::frontend::styles::badges::BadgeTone;
use crate::frontend::styles::colors::Theme;
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
    /// The invoice a row's "Record Payment" button was last clicked for
    /// -- `None` until then, and again after a successful submission.
    payment_invoice_id: Option<i64>,
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
    /// invoice's id -- looks up its own remaining balance and prefills
    /// the amount field with it. `SubmitPayment` posts against exactly
    /// this invoice's receivable, capped at its own balance -- other
    /// open invoices for the same customer are never touched.
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

    let heading = section_heading::view(theme, "Invoice list", "Every invoice on this company's books.");

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

    let invoice_ids: Vec<i64> = filtered.iter().map(|invoice| invoice.id.unwrap_or_default()).collect();
    let table = data_table::view(theme, columns, rows, None, |_index, _checked| Message::RowAction(0), |_checked| Message::RowAction(0), Message::RowAction, move |index| Message::RecordPaymentForRow(invoice_ids[index]));

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

    let payment_target_label = state
        .payment_invoice_id
        .and_then(|id| state.invoices.iter().find(|invoice| invoice.id == Some(id)).map(|invoice| (id, invoice.customer_id)))
        .map(|(id, customer_id)| {
            let customer_name = state.customers.iter().find(|c| c.id == customer_id).map(|c| c.label.clone()).unwrap_or_else(|| "unknown customer".to_string());
            format!("INV-{id} ({customer_name})")
        })
        .unwrap_or_else(|| "Click a row's Record Payment button to choose an invoice".to_string());
    let payment_form = column![
        text("Record a payment").size(13),
        text(format!("Invoice: {payment_target_label}")).size(12).color(theme.neutral_front()),
        pick_list(state.accounts.as_slice(), state.payment_cash_account.clone(), Message::PaymentCashAccountSelected).placeholder("Cash account"),
        pick_list(state.accounts.as_slice(), state.payment_ar_account.clone(), Message::PaymentArAccountSelected).placeholder("AR account"),
        text_input("Amount", &state.payment_amount_input).on_input(Message::PaymentAmountChanged).width(Length::Fill).padding(8),
        button(text("Record payment")).on_press(Message::SubmitPayment).padding(8),
    ]
    .spacing(8);

    let mut content = column![header, toolbar, heading, table, form, payment_form].spacing(20).padding(24).width(Length::Fill);

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
}
