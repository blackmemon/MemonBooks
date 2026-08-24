//! List bills and create + post a simple single-line bill. Mirrors
//! `sales::invoices_view`.

use chrono::{Days, Utc};
use rust_decimal::Decimal;

use iced::widget::{button, column, container, pick_list, scrollable, text, text_input};
use iced::{Element, Length};

use crate::backend::accounting::chart_of_accounts::Account;
use crate::backend::purchases::bills::{Bill, BillLine};
use crate::backend::services::purchase_service::PurchaseService;
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
    bills: Vec<Bill>,
    vendors: Vec<Choice>,
    accounts: Vec<Choice>,
    vendor: Option<Choice>,
    ap_account: Option<Choice>,
    expense_account: Option<Choice>,
    description_input: String,
    amount_input: String,
    search: String,
    /// The bill a row's "Pay" button was last clicked for -- `None`
    /// until then, and again after a successful submission.
    payment_bill_id: Option<i64>,
    payment_ap_account: Option<Choice>,
    payment_cash_account: Option<Choice>,
    payment_amount_input: String,
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
    CreateAndPost,
    SearchChanged(String),
    TabSelected(usize),
    FiltersClicked,
    SortClicked,
    RowAction(usize),
    /// Fired by a bill row's "Pay" button, carrying that bill's id --
    /// looks up its own remaining balance and prefills the amount field
    /// with it. `SubmitPayment` posts against exactly this bill's
    /// payable, capped at its own balance -- other open bills for the
    /// same vendor are never touched.
    PayForRow(i64),
    PaymentApAccountSelected(Choice),
    PaymentCashAccountSelected(Choice),
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
    let service = PurchaseService::new(ctx.db.clone());

    let vendors = service
        .list_vendors(ctx.company_id)
        .map(|vendors| vendors.iter().map(|v| Choice { id: v.id.unwrap_or_default(), label: v.name.clone() }).collect())
        .unwrap_or_default();
    let accounts: Vec<Choice> = service
        .list_gl_accounts(ctx.company_id)
        .map(|accounts: Vec<Account>| accounts.iter().map(|a| Choice { id: a.id.unwrap_or_default(), label: format!("{} - {}", a.code, a.name) }).collect())
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
        Message::CreateAndPost => {
            state.message = None;
            state.error = None;

            let (Some(vendor), Some(ap_account), Some(expense_account)) = (&state.vendor, &state.ap_account, &state.expense_account) else {
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
            bill.lines.push(BillLine::new(expense_account.id, state.description_input.clone(), Decimal::ONE, amount));

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
                    state.message = Some(format!("Posted bill #{}.", posted.id.unwrap_or_default()));
                    state.bills.push(posted);
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

            let (Some(bill_id), Some(ap_account), Some(cash_account)) = (state.payment_bill_id, &state.payment_ap_account, &state.payment_cash_account) else {
                state.error = Some("choose a bill (via a row's Pay button), AP account, and cash account".into());
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
            match service.record_payment_for_bill(bill_id, today, amount, ap_account.id, cash_account.id) {
                Ok(recorded) => {
                    state.message = Some(format!("Recorded payment #{} against BILL-{bill_id}.", recorded.id.unwrap_or_default()));
                    state.payment_amount_input.clear();
                    state.payment_bill_id = None;
                }
                Err(err) => state.error = Some(err.to_string()),
            }
        }
    }
}

pub fn view(state: &State) -> Element<'_, Message> {
    let header = page_header::view("Bills", "Everything you owe vendors, in one place.", "Create & post", Message::CreateAndPost);

    let toolbar = toolbar::view(
        &["All Bills"],
        0,
        Message::TabSelected,
        &state.search,
        "Search bills",
        Message::SearchChanged,
        "Filters",
        Message::FiltersClicked,
        "Sort by",
        Message::SortClicked,
    );

    let heading = section_heading::view("Bill list", "Every bill on this company's books.");

    let query = state.search.trim().to_lowercase();
    let filtered: Vec<&Bill> = state.bills.iter().filter(|bill| query.is_empty() || bill.id.map(|id| format!("BILL-{id}")).unwrap_or_default().to_lowercase().contains(&query)).collect();

    let columns = vec![
        ColumnDef::new("Bill", Length::FillPortion(1)).sorted(SortDirection::Descending),
        ColumnDef::new("Date", Length::FillPortion(1)),
        ColumnDef::new("Status", Length::FillPortion(1)),
        ColumnDef::new("Total", Length::FillPortion(1)),
    ];

    let rows = filtered
        .iter()
        .map(|bill| {
            Row::new(vec![
                Cell::text(bill.id.map(|id| format!("BILL-{id}")).unwrap_or_default()),
                Cell::date(bill.bill_date.to_string()),
                Cell::status(bill.status.as_str(), status_tone(bill.status.as_str())),
                Cell::text(bill.total().to_string()),
            ])
            .with_primary_action("Pay")
        })
        .collect();

    let bill_ids: Vec<i64> = filtered.iter().map(|bill| bill.id.unwrap_or_default()).collect();
    let table = data_table::view(columns, rows, None, |_index, _checked| Message::RowAction(0), |_checked| Message::RowAction(0), Message::RowAction, move |index| Message::PayForRow(bill_ids[index]));

    let form = column![
        text("Create & post a bill").size(13),
        pick_list(state.vendors.as_slice(), state.vendor.clone(), Message::VendorSelected).placeholder("Vendor"),
        pick_list(state.accounts.as_slice(), state.ap_account.clone(), Message::ApAccountSelected).placeholder("AP account"),
        pick_list(state.accounts.as_slice(), state.expense_account.clone(), Message::ExpenseAccountSelected).placeholder("Expense account"),
        text_input("Description", &state.description_input).on_input(Message::DescriptionChanged).width(Length::Fill).padding(8),
        text_input("Amount", &state.amount_input).on_input(Message::AmountChanged).width(Length::Fill).padding(8),
        button(text("Create & post")).on_press(Message::CreateAndPost).padding(8),
    ]
    .spacing(8);

    let payment_target_label = state
        .payment_bill_id
        .and_then(|id| state.bills.iter().find(|bill| bill.id == Some(id)).map(|bill| (id, bill.vendor_id)))
        .map(|(id, vendor_id)| {
            let vendor_name = state.vendors.iter().find(|v| v.id == vendor_id).map(|v| v.label.clone()).unwrap_or_else(|| "unknown vendor".to_string());
            format!("BILL-{id} ({vendor_name})")
        })
        .unwrap_or_else(|| "Click a row's Pay button to choose a bill".to_string());
    let payment_form = column![
        text("Pay a bill").size(13),
        text(format!("Bill: {payment_target_label}")).size(12).color(colors::WHITE_NEUTRAL_FRONT),
        pick_list(state.accounts.as_slice(), state.payment_ap_account.clone(), Message::PaymentApAccountSelected).placeholder("AP account"),
        pick_list(state.accounts.as_slice(), state.payment_cash_account.clone(), Message::PaymentCashAccountSelected).placeholder("Cash account"),
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
        db.run_migrations(organization_repository::MIGRATIONS).unwrap();
        db.run_migrations(party_repository::MIGRATIONS).unwrap();
        db.run_migrations(accounting_repository::MIGRATIONS).unwrap();
        db.run_migrations(crate::backend::audit::audit_logs::MIGRATIONS).unwrap();
        db.run_migrations(purchase_repository::MIGRATIONS).unwrap();

        let org_repo = OrganizationRepository::new(db.clone());
        let company_id = org_repo.create_company(&Company::new("Acme Trading Co", "USD")).unwrap().id.unwrap();

        let party_repo = PartyRepository::new(db.clone());
        let vendor = party_repo.create_vendor(&Vendor::new(company_id, "Acme Supplies")).unwrap();

        let accounting_repo = AccountingRepository::new(db.clone());
        accounting_repo.create_fiscal_period(&FiscalPeriod::new(company_id, "Open Period", date(2026, 1, 1), date(2030, 12, 31))).unwrap();
        let ap = accounting_repo.create_account(&Account::new(company_id, "2000", "Accounts Payable", AccountType::Liability)).unwrap();
        let expense = accounting_repo.create_account(&Account::new(company_id, "6000", "Office Supplies", AccountType::Expense)).unwrap();

        let ctx = Ctx { db, company_id, current_user_id: None, export_dir: PathBuf::from(".") };
        let vendor_choice = Choice { id: vendor.id.unwrap(), label: vendor.name.clone() };
        let ap_choice = Choice { id: ap.id.unwrap(), label: "2000 - Accounts Payable".into() };
        let expense_choice = Choice { id: expense.id.unwrap(), label: "6000 - Office Supplies".into() };
        (ctx, vendor_choice, ap_choice, expense_choice)
    }

    #[test]
    fn field_changed_messages_update_the_corresponding_field() {
        let (ctx, vendor, ap, expense) = ctx_with_vendor_and_accounts();
        let mut state = State::default();

        update(&mut state, Message::VendorSelected(vendor.clone()), &ctx);
        update(&mut state, Message::ApAccountSelected(ap.clone()), &ctx);
        update(&mut state, Message::ExpenseAccountSelected(expense.clone()), &ctx);
        update(&mut state, Message::DescriptionChanged("Office supplies".into()), &ctx);
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
        let mut state = State { vendor: Some(vendor), ..Default::default() };

        update(&mut state, Message::CreateAndPost, &ctx);

        assert_eq!(state.error.as_deref(), Some("choose a vendor, AP account, and expense account"));
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
        assert!(state.message.as_deref().unwrap_or_default().starts_with("Posted bill"));
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
    fn create_and_post_bill(state: &mut State, ctx: &Ctx, vendor: &Choice, ap: &Choice, expense: &Choice, amount: &str) -> i64 {
        state.vendor = Some(vendor.clone());
        state.ap_account = Some(ap.clone());
        state.expense_account = Some(expense.clone());
        state.description_input = "Office supplies".into();
        state.amount_input = amount.into();
        update(state, Message::CreateAndPost, ctx);
        state.bills.last().unwrap().id.unwrap()
    }

    #[test]
    fn paying_from_a_row_prefills_the_amount_with_that_bill_s_own_balance() {
        let (ctx, vendor, ap, expense) = ctx_with_vendor_and_accounts();
        let mut state = State { vendors: vec![vendor.clone()], ..Default::default() };
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

        assert_eq!(state.error.as_deref(), Some("choose a bill (via a row's Pay button), AP account, and cash account"));
    }

    #[test]
    fn submitting_a_payment_with_a_non_numeric_amount_sets_an_error() {
        let (ctx, vendor, ap, expense) = ctx_with_vendor_and_accounts();
        let cash = ap.clone();
        let mut state = State { vendors: vec![vendor.clone()], ..Default::default() };
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
        let mut state = State { vendors: vec![vendor.clone()], ..Default::default() };
        let bill_id = create_and_post_bill(&mut state, &ctx, &vendor, &ap, &expense, "450");

        update(&mut state, Message::PayForRow(bill_id), &ctx);
        update(&mut state, Message::PaymentApAccountSelected(ap), &ctx);
        update(&mut state, Message::PaymentCashAccountSelected(cash), &ctx);
        update(&mut state, Message::SubmitPayment, &ctx);

        assert!(state.error.is_none());
        assert!(state.message.as_deref().unwrap_or_default().contains(&format!("BILL-{bill_id}")));
        assert!(state.payment_bill_id.is_none());

        let service = PurchaseService::new(ctx.db.clone());
        assert!(service.bill_remaining_balance(bill_id).is_err(), "a fully-settled bill has no open payable left");
    }

    #[test]
    fn a_partial_payment_reduces_only_that_bill_s_balance() {
        let (ctx, vendor, ap, expense) = ctx_with_vendor_and_accounts();
        let cash = ap.clone();
        let mut state = State { vendors: vec![vendor.clone()], ..Default::default() };
        let bill_id = create_and_post_bill(&mut state, &ctx, &vendor, &ap, &expense, "450");

        update(&mut state, Message::PayForRow(bill_id), &ctx);
        state.payment_amount_input = "200".into();
        update(&mut state, Message::PaymentApAccountSelected(ap), &ctx);
        update(&mut state, Message::PaymentCashAccountSelected(cash), &ctx);
        update(&mut state, Message::SubmitPayment, &ctx);

        assert!(state.error.is_none());
        let service = PurchaseService::new(ctx.db.clone());
        assert_eq!(service.bill_remaining_balance(bill_id).unwrap(), rust_decimal_macros::dec!(250));
    }

    #[test]
    fn an_overpayment_attempt_beyond_the_bill_s_own_balance_is_rejected() {
        let (ctx, vendor, ap, expense) = ctx_with_vendor_and_accounts();
        let cash = ap.clone();
        let mut state = State { vendors: vec![vendor.clone()], ..Default::default() };
        let bill_id = create_and_post_bill(&mut state, &ctx, &vendor, &ap, &expense, "450");

        update(&mut state, Message::PayForRow(bill_id), &ctx);
        state.payment_amount_input = "600".into();
        update(&mut state, Message::PaymentApAccountSelected(ap), &ctx);
        update(&mut state, Message::PaymentCashAccountSelected(cash), &ctx);
        update(&mut state, Message::SubmitPayment, &ctx);

        assert!(state.error.as_deref().unwrap_or_default().contains("exceeds"));
        assert!(state.message.is_none());

        let service = PurchaseService::new(ctx.db.clone());
        assert_eq!(service.bill_remaining_balance(bill_id).unwrap(), rust_decimal_macros::dec!(450), "a rejected overpayment must not touch the bill's balance at all");
    }

    /// Regression test for the exact bug this feature replaces: paying
    /// one bill via its row must never touch a *different* open bill for
    /// the same vendor, even partially.
    #[test]
    fn paying_one_bill_via_its_row_leaves_the_vendor_s_other_open_bill_completely_untouched() {
        let (ctx, vendor, ap, expense) = ctx_with_vendor_and_accounts();
        let cash = ap.clone();
        let mut state = State { vendors: vec![vendor.clone()], ..Default::default() };
        let first_bill_id = create_and_post_bill(&mut state, &ctx, &vendor, &ap, &expense, "450");
        let second_bill_id = create_and_post_bill(&mut state, &ctx, &vendor, &ap, &expense, "300");

        update(&mut state, Message::PayForRow(first_bill_id), &ctx);
        update(&mut state, Message::PaymentApAccountSelected(ap), &ctx);
        update(&mut state, Message::PaymentCashAccountSelected(cash), &ctx);
        update(&mut state, Message::SubmitPayment, &ctx);

        assert!(state.error.is_none());

        let service = PurchaseService::new(ctx.db.clone());
        assert!(service.bill_remaining_balance(first_bill_id).is_err(), "the paid bill should be fully settled");
        assert_eq!(
            service.bill_remaining_balance(second_bill_id).unwrap(),
            rust_decimal_macros::dec!(300),
            "the vendor's other open bill must be completely untouched"
        );
    }
}
