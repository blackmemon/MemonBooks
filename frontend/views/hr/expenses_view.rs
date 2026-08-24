//! Record direct expenses. Grouped under `hr/` rather than a dedicated
//! `expenses/` directory -- the frozen frontend/views/ structure
//! (Section 2) has no slot for one; expense claims are employee-facing,
//! so this pairs naturally with HR in the same way it does in most ERP
//! navigation.
//!
//! Simplified for this first pass: rather than a full category
//! management screen, picking an expense GL account transparently
//! reuses (or creates, the first time this session) a matching
//! `ExpenseCategory` for it -- the category concept still goes through
//! the real domain model, it's just not separately managed here yet.

use std::collections::HashMap;

use chrono::Utc;
use rust_decimal::Decimal;

use iced::widget::{column, container, pick_list, row, scrollable, text, text_input};
use iced::{Element, Length};

use crate::backend::accounting::chart_of_accounts::Account;
use crate::backend::expenses::expense_categories::ExpenseCategory;
use crate::backend::expenses::expenses::Expense;
use crate::backend::services::expense_service::ExpenseService;
use crate::frontend::components::common::page_header;
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
    expense_accounts: Vec<Choice>,
    payment_accounts: Vec<Choice>,
    expense_account: Option<Choice>,
    payment_account: Option<Choice>,
    description_input: String,
    amount_input: String,
    /// expense_account_id -> category_id, created on first use this session.
    category_for_account: HashMap<i64, i64>,
    message: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    ExpenseAccountSelected(Choice),
    PaymentAccountSelected(Choice),
    DescriptionChanged(String),
    AmountChanged(String),
    Record,
}

pub fn load(ctx: &Ctx) -> State {
    let service = ExpenseService::new(ctx.db.clone());
    let accounts: Vec<Choice> = service
        .list_gl_accounts(ctx.company_id)
        .map(|accounts: Vec<Account>| accounts.iter().map(|a| Choice { id: a.id.unwrap_or_default(), label: format!("{} - {}", a.code, a.name) }).collect())
        .unwrap_or_default();

    State {
        expense_accounts: accounts.clone(),
        payment_accounts: accounts,
        ..Default::default()
    }
}

fn category_id_for(state: &mut State, ctx: &Ctx, expense_account: &Choice) -> AppResultId {
    if let Some(id) = state.category_for_account.get(&expense_account.id) {
        return Ok(*id);
    }
    let service = ExpenseService::new(ctx.db.clone());
    let category = service.create_category(&ExpenseCategory::new(ctx.company_id, expense_account.label.clone(), expense_account.id))?;
    let id = category.id.unwrap_or_default();
    state.category_for_account.insert(expense_account.id, id);
    Ok(id)
}

type AppResultId = crate::backend::core::errors::AppResult<i64>;

pub fn update(state: &mut State, message: Message, ctx: &Ctx) {
    match message {
        Message::ExpenseAccountSelected(choice) => state.expense_account = Some(choice),
        Message::PaymentAccountSelected(choice) => state.payment_account = Some(choice),
        Message::DescriptionChanged(value) => state.description_input = value,
        Message::AmountChanged(value) => state.amount_input = value,
        Message::Record => {
            state.message = None;
            state.error = None;

            let (Some(expense_account), Some(payment_account)) = (state.expense_account.clone(), state.payment_account.clone()) else {
                state.error = Some("choose an expense account and a payment account".into());
                return;
            };
            let amount: Decimal = match state.amount_input.parse() {
                Ok(amount) => amount,
                Err(_) => {
                    state.error = Some("amount is not a valid number".into());
                    return;
                }
            };

            let category_id = match category_id_for(state, ctx, &expense_account) {
                Ok(id) => id,
                Err(err) => {
                    state.error = Some(err.to_string());
                    return;
                }
            };

            let today = Utc::now().date_naive();
            let expense = Expense::new(ctx.company_id, category_id, today, amount, state.description_input.clone(), payment_account.id);

            let service = ExpenseService::new(ctx.db.clone());
            match service.record_expense(&expense) {
                Ok((_recorded, transaction)) => {
                    state.message = Some(format!("Recorded and posted transaction #{}.", transaction.id.unwrap_or_default()));
                    state.description_input.clear();
                    state.amount_input.clear();
                }
                Err(err) => state.error = Some(err.to_string()),
            }
        }
    }
}

pub fn view(state: &State) -> Element<'_, Message> {
    let header = page_header::view("Expenses", "Record a direct expense and post it.", "Record expense", Message::Record);

    let form = column![
        text("Expense account"),
        pick_list(state.expense_accounts.as_slice(), state.expense_account.clone(), Message::ExpenseAccountSelected).placeholder("Expense account"),
        text("Payment account"),
        pick_list(state.payment_accounts.as_slice(), state.payment_account.clone(), Message::PaymentAccountSelected).placeholder("Payment account"),
        row![
            text_input("Description", &state.description_input).on_input(Message::DescriptionChanged).width(Length::Fill).padding(6),
            text_input("Amount", &state.amount_input).on_input(Message::AmountChanged).width(Length::Fixed(120.0)).padding(6),
        ]
        .spacing(8),
    ]
    .spacing(10);

    let mut content = column![header, form].spacing(20).padding(24).width(Length::Fill);

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
    use crate::backend::repositories::accounting_repository::{self, AccountingRepository};
    use crate::backend::repositories::expense_repository;
    use crate::backend::repositories::organization_repository::{self, OrganizationRepository};
    use chrono::NaiveDate;
    use crate::database::database::Database;
    use std::path::PathBuf;

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    fn ctx_with_two_accounts() -> (Ctx, Choice, Choice) {
        let db = Database::open_in_memory().unwrap();
        db.run_migrations(organization_repository::MIGRATIONS).unwrap();
        db.run_migrations(accounting_repository::MIGRATIONS).unwrap();
        db.run_migrations(crate::backend::audit::audit_logs::MIGRATIONS).unwrap();
        db.run_migrations(expense_repository::MIGRATIONS).unwrap();

        let org_repo = OrganizationRepository::new(db.clone());
        let company_id = org_repo.create_company(&Company::new("Acme Trading Co", "USD")).unwrap().id.unwrap();

        let accounting_repo = AccountingRepository::new(db.clone());
        accounting_repo.create_fiscal_period(&FiscalPeriod::new(company_id, "Open Period", date(2026, 1, 1), date(2030, 12, 31))).unwrap();
        let rent = accounting_repo.create_account(&Account::new(company_id, "6000", "Rent Expense", AccountType::Expense)).unwrap();
        let cash = accounting_repo.create_account(&Account::new(company_id, "1000", "Cash", AccountType::Asset)).unwrap();

        let ctx = Ctx { db, company_id, current_user_id: None, export_dir: PathBuf::from(".") };
        let expense_account = Choice { id: rent.id.unwrap(), label: "6000 - Rent Expense".into() };
        let payment_account = Choice { id: cash.id.unwrap(), label: "1000 - Cash".into() };
        (ctx, expense_account, payment_account)
    }

    #[test]
    fn field_changed_messages_update_the_corresponding_field() {
        let (ctx, expense_account, payment_account) = ctx_with_two_accounts();
        let mut state = State::default();

        update(&mut state, Message::ExpenseAccountSelected(expense_account.clone()), &ctx);
        update(&mut state, Message::PaymentAccountSelected(payment_account.clone()), &ctx);
        update(&mut state, Message::DescriptionChanged("Office rent".into()), &ctx);
        update(&mut state, Message::AmountChanged("1200".into()), &ctx);

        assert_eq!(state.expense_account, Some(expense_account));
        assert_eq!(state.payment_account, Some(payment_account));
        assert_eq!(state.description_input, "Office rent");
        assert_eq!(state.amount_input, "1200");
    }

    #[test]
    fn recording_without_choosing_both_accounts_sets_an_error() {
        let (ctx, expense_account, _payment_account) = ctx_with_two_accounts();
        let mut state = State { expense_account: Some(expense_account), ..Default::default() };

        update(&mut state, Message::Record, &ctx);

        assert_eq!(state.error.as_deref(), Some("choose an expense account and a payment account"));
    }

    #[test]
    fn recording_a_non_numeric_amount_sets_an_error() {
        let (ctx, expense_account, payment_account) = ctx_with_two_accounts();
        let mut state = State {
            expense_account: Some(expense_account),
            payment_account: Some(payment_account),
            amount_input: "not a number".into(),
            ..Default::default()
        };

        update(&mut state, Message::Record, &ctx);

        assert_eq!(state.error.as_deref(), Some("amount is not a valid number"));
    }

    #[test]
    fn recording_a_valid_expense_posts_it_and_clears_the_form() {
        let (ctx, expense_account, payment_account) = ctx_with_two_accounts();
        let mut state = State {
            expense_account: Some(expense_account.clone()),
            payment_account: Some(payment_account),
            description_input: "Office rent".into(),
            amount_input: "1200".into(),
            ..Default::default()
        };

        update(&mut state, Message::Record, &ctx);

        assert!(state.error.is_none());
        assert!(state.message.as_deref().unwrap_or_default().starts_with("Recorded and posted transaction"));
        assert!(state.description_input.is_empty());
        assert!(state.amount_input.is_empty());
        assert!(state.category_for_account.contains_key(&expense_account.id));
    }

    #[test]
    fn recording_a_second_expense_on_the_same_account_reuses_the_cached_category() {
        let (ctx, expense_account, payment_account) = ctx_with_two_accounts();
        let mut state = State {
            expense_account: Some(expense_account.clone()),
            payment_account: Some(payment_account.clone()),
            amount_input: "1200".into(),
            ..Default::default()
        };
        update(&mut state, Message::Record, &ctx);
        let category_id_after_first = *state.category_for_account.get(&expense_account.id).unwrap();

        state.amount_input = "300".into();
        update(&mut state, Message::Record, &ctx);
        let category_id_after_second = *state.category_for_account.get(&expense_account.id).unwrap();

        assert_eq!(category_id_after_first, category_id_after_second);
    }
}
