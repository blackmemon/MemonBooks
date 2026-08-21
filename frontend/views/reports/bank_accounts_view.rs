//! Bank account summary: book balance and open items. Grouped under
//! `reports/` rather than a dedicated `banking/` directory -- the frozen
//! frontend/views/ structure (Section 2) has no slot for one, and a
//! reconciliation summary is inherently report-like.

use chrono::Utc;

use iced::widget::{button, column, container, pick_list, row, scrollable, text, text_input};
use iced::{Element, Length};

use crate::backend::accounting::chart_of_accounts::Account;
use crate::backend::banking::bank_accounts::BankAccount;
use crate::backend::repositories::accounting_repository::AccountingRepository;
use crate::backend::repositories::banking_repository::BankingRepository;
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
    bank_accounts: Vec<(BankAccount, String)>,
    gl_accounts: Vec<Choice>,
    gl_account: Option<Choice>,
    bank_name_input: String,
    account_number_input: String,
    error: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    GlAccountSelected(Choice),
    BankNameChanged(String),
    AccountNumberChanged(String),
    Create,
}

pub fn load(ctx: &Ctx) -> State {
    let banking_repo = BankingRepository::new(ctx.db.clone());
    let accounting_repo = AccountingRepository::new(ctx.db.clone());
    let today = Utc::now().date_naive();

    let bank_accounts = banking_repo
        .list_bank_accounts_for_company(ctx.company_id)
        .map(|accounts| {
            accounts
                .into_iter()
                .map(|account| {
                    let balance = banking_repo.book_balance(account.id.unwrap_or_default(), today).map(|b| b.to_string()).unwrap_or_else(|_| "?".to_string());
                    (account, balance)
                })
                .collect()
        })
        .unwrap_or_default();

    let gl_accounts = accounting_repo
        .list_accounts_for_company(ctx.company_id)
        .map(|accounts: Vec<Account>| accounts.iter().map(|a| Choice { id: a.id.unwrap_or_default(), label: format!("{} - {}", a.code, a.name) }).collect())
        .unwrap_or_default();

    State {
        bank_accounts,
        gl_accounts,
        ..Default::default()
    }
}

pub fn update(state: &mut State, message: Message, ctx: &Ctx) {
    match message {
        Message::GlAccountSelected(choice) => state.gl_account = Some(choice),
        Message::BankNameChanged(value) => state.bank_name_input = value,
        Message::AccountNumberChanged(value) => state.account_number_input = value,
        Message::Create => {
            let Some(gl_account) = &state.gl_account else {
                state.error = Some("choose a GL account".into());
                return;
            };
            let bank_account = BankAccount::new(ctx.company_id, gl_account.id, state.bank_name_input.clone(), state.account_number_input.clone(), "USD");
            let repo = BankingRepository::new(ctx.db.clone());
            match repo.create_bank_account(&bank_account) {
                Ok(created) => {
                    state.bank_accounts.push((created, "0".to_string()));
                    state.bank_name_input.clear();
                    state.account_number_input.clear();
                    state.error = None;
                }
                Err(err) => state.error = Some(err.to_string()),
            }
        }
    }
}

pub fn view(state: &State) -> Element<'_, Message> {
    let mut list = column![row![
        text("Bank").width(Length::Fill),
        text("Account #").width(Length::Fixed(150.0)),
        text("Book Balance").width(Length::Fixed(120.0)),
    ]]
    .spacing(4);

    for (account, balance) in &state.bank_accounts {
        list = list.push(row![
            text(account.bank_name.clone()).width(Length::Fill),
            text(account.account_number.clone()).width(Length::Fixed(150.0)),
            text(balance.clone()).width(Length::Fixed(120.0)),
        ]);
    }

    let form = row![
        pick_list(state.gl_accounts.as_slice(), state.gl_account.clone(), Message::GlAccountSelected).placeholder("GL account"),
        text_input("Bank name", &state.bank_name_input).on_input(Message::BankNameChanged).width(Length::Fill).padding(6),
        text_input("Account number", &state.account_number_input).on_input(Message::AccountNumberChanged).width(Length::Fixed(150.0)).padding(6),
        button(text("Add bank account")).on_press(Message::Create).padding(6),
    ]
    .spacing(8);

    let mut content = column![text("Bank Accounts").size(24), scrollable(list).height(Length::Fill), form]
        .spacing(16)
        .padding(20);

    if let Some(error) = &state.error {
        content = content.push(text(error.clone()).color(colors::WHITE_NEGATIVE_FRONT));
    }

    container(content).into()
}
