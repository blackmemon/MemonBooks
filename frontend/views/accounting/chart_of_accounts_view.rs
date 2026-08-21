//! List the chart of accounts and add new accounts to it.

use iced::widget::{button, column, container, pick_list, row, scrollable, text, text_input};
use iced::{Element, Length};

use crate::backend::accounting::accounting_rules::AccountType;
use crate::backend::accounting::chart_of_accounts::Account;
use crate::backend::core::validation::Validate;
use crate::backend::repositories::accounting_repository::AccountingRepository;
use crate::frontend::styles::colors;
use crate::frontend::views::Ctx;

const ACCOUNT_TYPES: [AccountType; 5] = [
    AccountType::Asset,
    AccountType::Liability,
    AccountType::Equity,
    AccountType::Revenue,
    AccountType::Expense,
];

#[derive(Debug, Clone, Default)]
pub struct State {
    accounts: Vec<Account>,
    code_input: String,
    name_input: String,
    type_input: Option<AccountType>,
    error: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    CodeChanged(String),
    NameChanged(String),
    TypeSelected(AccountType),
    Create,
}

pub fn load(ctx: &Ctx) -> State {
    let repo = AccountingRepository::new(ctx.db.clone());
    match repo.list_accounts_for_company(ctx.company_id) {
        Ok(accounts) => State { accounts, ..Default::default() },
        Err(err) => State { error: Some(err.to_string()), ..Default::default() },
    }
}

pub fn update(state: &mut State, message: Message, ctx: &Ctx) {
    match message {
        Message::CodeChanged(value) => state.code_input = value,
        Message::NameChanged(value) => state.name_input = value,
        Message::TypeSelected(account_type) => state.type_input = Some(account_type),
        Message::Create => {
            let Some(account_type) = state.type_input else {
                state.error = Some("choose an account type".into());
                return;
            };
            let account = Account::new(ctx.company_id, state.code_input.clone(), state.name_input.clone(), account_type);
            if let Err(err) = account.validate() {
                state.error = Some(err.to_string());
                return;
            }
            let repo = AccountingRepository::new(ctx.db.clone());
            match repo.create_account(&account) {
                Ok(created) => {
                    state.accounts.push(created);
                    state.accounts.sort_by(|a, b| a.code.cmp(&b.code));
                    state.code_input.clear();
                    state.name_input.clear();
                    state.type_input = None;
                    state.error = None;
                }
                Err(err) => state.error = Some(err.to_string()),
            }
        }
    }
}

pub fn view(state: &State) -> Element<'_, Message> {
    let mut list = column![row![
        text("Code").width(Length::Fixed(80.0)),
        text("Name").width(Length::Fill),
        text("Type").width(Length::Fixed(100.0)),
    ]]
    .spacing(4);

    for account in &state.accounts {
        list = list.push(row![
            text(account.code.clone()).width(Length::Fixed(80.0)),
            text(account.name.clone()).width(Length::Fill),
            text(account.account_type.as_str()).width(Length::Fixed(100.0)),
        ]);
    }

    let form = row![
        text_input("Code", &state.code_input).on_input(Message::CodeChanged).width(Length::Fixed(80.0)).padding(6),
        text_input("Name", &state.name_input).on_input(Message::NameChanged).width(Length::Fill).padding(6),
        pick_list(ACCOUNT_TYPES, state.type_input, Message::TypeSelected).placeholder("Type"),
        button(text("Add account")).on_press(Message::Create).padding(6),
    ]
    .spacing(8);

    let mut content = column![text("Chart of Accounts").size(24), scrollable(list).height(Length::Fill), form]
        .spacing(16)
        .padding(20);

    if let Some(error) = &state.error {
        content = content.push(text(error.clone()).color(colors::WHITE_NEGATIVE_FRONT));
    }

    container(content).into()
}
