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
        })
        .collect();

    let table = data_table::view(columns, rows, None, |_index, _checked| Message::RowAction(0), Message::RowAction);

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

    let mut content = column![header, toolbar, heading, table, form].spacing(20).padding(24).width(Length::Fill);

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
}
