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

use iced::widget::{button, column, container, pick_list, row, scrollable, text, text_input};
use iced::{Element, Length};

use crate::backend::accounting::chart_of_accounts::Account;
use crate::backend::expenses::expense_categories::ExpenseCategory;
use crate::backend::expenses::expenses::Expense;
use crate::backend::repositories::accounting_repository::AccountingRepository;
use crate::backend::repositories::expense_repository::ExpenseRepository;
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
    let accounting_repo = AccountingRepository::new(ctx.db.clone());
    let accounts: Vec<Choice> = accounting_repo
        .list_accounts_for_company(ctx.company_id)
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
    let repo = ExpenseRepository::new(ctx.db.clone());
    let category = repo.create_category(&ExpenseCategory::new(ctx.company_id, expense_account.label.clone(), expense_account.id))?;
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

            let repo = ExpenseRepository::new(ctx.db.clone());
            match repo.record_expense(&expense) {
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
    let mut content = column![
        text("Expenses").size(24),
        text("Expense account"),
        pick_list(state.expense_accounts.as_slice(), state.expense_account.clone(), Message::ExpenseAccountSelected).placeholder("Expense account"),
        text("Payment account"),
        pick_list(state.payment_accounts.as_slice(), state.payment_account.clone(), Message::PaymentAccountSelected).placeholder("Payment account"),
        row![
            text_input("Description", &state.description_input).on_input(Message::DescriptionChanged).width(Length::Fill).padding(6),
            text_input("Amount", &state.amount_input).on_input(Message::AmountChanged).width(Length::Fixed(120.0)).padding(6),
        ]
        .spacing(8),
        button(text("Record expense")).on_press(Message::Record).padding(8),
    ]
    .spacing(10)
    .padding(20);

    if let Some(message) = &state.message {
        content = content.push(text(message.clone()).color(colors::WHITE_POSITIVE_FRONT));
    }
    if let Some(error) = &state.error {
        content = content.push(text(error.clone()).color(colors::WHITE_NEGATIVE_FRONT));
    }

    container(scrollable(content)).into()
}
