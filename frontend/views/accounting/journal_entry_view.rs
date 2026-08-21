//! Create and post a simple two-line manual journal entry (one debit
//! account, one credit account, one amount). Multi-line entries are
//! fully supported by the domain layer (`accounting::journal_entries`)
//! even though this first-pass screen only exposes the common
//! single-debit/single-credit case.

use chrono::{NaiveDate, Utc};
use rust_decimal::Decimal;

use iced::widget::{button, column, container, pick_list, text, text_input};
use iced::Element;

use crate::backend::accounting::chart_of_accounts::Account;
use crate::backend::accounting::journal_entries::JournalEntry;
use crate::backend::repositories::accounting_repository::AccountingRepository;
use crate::frontend::styles::colors;
use crate::frontend::views::Ctx;

#[derive(Debug, Clone, PartialEq)]
pub struct AccountOption {
    id: i64,
    label: String,
}

impl std::fmt::Display for AccountOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.label)
    }
}

#[derive(Debug, Clone, Default)]
pub struct State {
    accounts: Vec<AccountOption>,
    debit_account: Option<AccountOption>,
    credit_account: Option<AccountOption>,
    amount_input: String,
    memo_input: String,
    message: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    DebitAccountSelected(AccountOption),
    CreditAccountSelected(AccountOption),
    AmountChanged(String),
    MemoChanged(String),
    Post,
}

fn to_option(account: &Account) -> AccountOption {
    AccountOption {
        id: account.id.unwrap_or_default(),
        label: format!("{} - {}", account.code, account.name),
    }
}

pub fn load(ctx: &Ctx) -> State {
    let repo = AccountingRepository::new(ctx.db.clone());
    match repo.list_accounts_for_company(ctx.company_id) {
        Ok(accounts) => State {
            accounts: accounts.iter().map(to_option).collect(),
            ..Default::default()
        },
        Err(err) => State { error: Some(err.to_string()), ..Default::default() },
    }
}

pub fn update(state: &mut State, message: Message, ctx: &Ctx) {
    match message {
        Message::DebitAccountSelected(account) => state.debit_account = Some(account),
        Message::CreditAccountSelected(account) => state.credit_account = Some(account),
        Message::AmountChanged(value) => state.amount_input = value,
        Message::MemoChanged(value) => state.memo_input = value,
        Message::Post => {
            state.message = None;
            state.error = None;

            let (Some(debit), Some(credit)) = (&state.debit_account, &state.credit_account) else {
                state.error = Some("choose both a debit and a credit account".into());
                return;
            };
            let amount: Decimal = match state.amount_input.parse() {
                Ok(amount) => amount,
                Err(_) => {
                    state.error = Some("amount is not a valid number".into());
                    return;
                }
            };

            let today: NaiveDate = Utc::now().date_naive();
            let entry = JournalEntry::new(ctx.company_id, today)
                .with_memo(state.memo_input.clone())
                .debit(debit.id, amount)
                .credit(credit.id, amount)
                .into_transaction();

            let repo = AccountingRepository::new(ctx.db.clone());
            match repo.post_transaction(&entry) {
                Ok(posted) => {
                    state.message = Some(format!("Posted transaction #{}.", posted.id.unwrap_or_default()));
                    state.amount_input.clear();
                    state.memo_input.clear();
                }
                Err(err) => state.error = Some(err.to_string()),
            }
        }
    }
}

pub fn view(state: &State) -> Element<'_, Message> {
    let mut content = column![
        text("New Journal Entry").size(24),
        text("Debit account"),
        pick_list(state.accounts.as_slice(), state.debit_account.clone(), Message::DebitAccountSelected).placeholder("Select account"),
        text("Credit account"),
        pick_list(state.accounts.as_slice(), state.credit_account.clone(), Message::CreditAccountSelected).placeholder("Select account"),
        text("Amount"),
        text_input("0.00", &state.amount_input).on_input(Message::AmountChanged).padding(8),
        text("Memo"),
        text_input("Description", &state.memo_input).on_input(Message::MemoChanged).padding(8),
        button(text("Post entry")).on_press(Message::Post).padding(8),
    ]
    .spacing(10)
    .padding(20);

    if let Some(message) = &state.message {
        content = content.push(text(message.clone()).color(colors::WHITE_POSITIVE_FRONT));
    }
    if let Some(error) = &state.error {
        content = content.push(text(error.clone()).color(colors::WHITE_NEGATIVE_FRONT));
    }

    container(content).into()
}
