//! Bank account summary: book balance and open items. Grouped under
//! `reports/` rather than a dedicated `banking/` directory -- the frozen
//! frontend/views/ structure (Section 2) has no slot for one, and a
//! reconciliation summary is inherently report-like.

use chrono::Utc;

use iced::widget::{button, column, container, pick_list, scrollable, text, text_input};
use iced::{Element, Length};

use crate::backend::accounting::chart_of_accounts::Account;
use crate::backend::banking::bank_accounts::BankAccount;
use crate::backend::services::banking_service::BankingService;
use crate::frontend::components::common::data_table::{self, Cell, ColumnDef, Row, SortDirection};
use crate::frontend::components::common::{page_header, section_heading, toolbar};
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
    search: String,
    error: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    GlAccountSelected(Choice),
    BankNameChanged(String),
    AccountNumberChanged(String),
    Create,
    SearchChanged(String),
    TabSelected(usize),
    FiltersClicked,
    SortClicked,
    RowAction(usize),
}

pub fn load(ctx: &Ctx) -> State {
    let service = BankingService::new(ctx.db.clone());
    let today = Utc::now().date_naive();

    let bank_accounts = service
        .list_bank_accounts(ctx.company_id)
        .map(|accounts| {
            accounts
                .into_iter()
                .map(|account| {
                    let balance = service.book_balance(account.id.unwrap_or_default(), today).map(|b| b.to_string()).unwrap_or_else(|_| "?".to_string());
                    (account, balance)
                })
                .collect()
        })
        .unwrap_or_default();

    let gl_accounts = service
        .list_gl_accounts(ctx.company_id)
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
            let service = BankingService::new(ctx.db.clone());
            match service.create_bank_account(&bank_account) {
                Ok(created) => {
                    state.bank_accounts.push((created, "0".to_string()));
                    state.bank_name_input.clear();
                    state.account_number_input.clear();
                    state.error = None;
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
    let header = page_header::view("Bank Accounts", "Every bank account on this company's books.", "Add bank account", Message::Create);

    let toolbar = toolbar::view(
        &["All Bank Accounts"],
        0,
        Message::TabSelected,
        &state.search,
        "Search bank accounts",
        Message::SearchChanged,
        "Filters",
        Message::FiltersClicked,
        "Sort by",
        Message::SortClicked,
    );

    let heading = section_heading::view("Bank account list", "Book balance as of today for every account.");

    let query = state.search.trim().to_lowercase();
    let filtered: Vec<&(BankAccount, String)> =
        state.bank_accounts.iter().filter(|(account, _)| query.is_empty() || account.bank_name.to_lowercase().contains(&query) || account.account_number.to_lowercase().contains(&query)).collect();

    let columns = vec![
        ColumnDef::new("Bank", Length::FillPortion(3)).sorted(SortDirection::Ascending),
        ColumnDef::new("Account #", Length::FillPortion(2)),
        ColumnDef::new("Book Balance", Length::FillPortion(1)),
    ];

    let rows = filtered.iter().map(|(account, balance)| Row::new(vec![Cell::text(account.bank_name.clone()), Cell::text(account.account_number.clone()), Cell::text(balance.clone())])).collect();

    let table = data_table::view(columns, rows, None, |_index, _checked| Message::RowAction(0), |_checked| Message::RowAction(0), Message::RowAction, Message::RowAction);

    let form = column![
        text("Add a bank account").size(13),
        pick_list(state.gl_accounts.as_slice(), state.gl_account.clone(), Message::GlAccountSelected).placeholder("GL account"),
        text_input("Bank name", &state.bank_name_input).on_input(Message::BankNameChanged).width(Length::Fill).padding(8),
        text_input("Account number", &state.account_number_input).on_input(Message::AccountNumberChanged).width(Length::Fill).padding(8),
        button(text("Add bank account")).on_press(Message::Create).padding(8),
    ]
    .spacing(8);

    let mut content = column![header, toolbar, heading, table, form].spacing(20).padding(24).width(Length::Fill);

    if let Some(error) = &state.error {
        content = content.push(text(error.clone()).color(colors::WHITE_NEGATIVE_FRONT));
    }

    container(scrollable(content).height(Length::Fill)).width(Length::Fill).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::accounting::accounting_rules::AccountType;
    use crate::backend::organization::companies::Company;
    use crate::backend::repositories::accounting_repository::{self, AccountingRepository};
    use crate::backend::repositories::banking_repository;
    use crate::backend::repositories::organization_repository::{self, OrganizationRepository};
    use crate::database::database::Database;
    use std::path::PathBuf;

    fn ctx_with_a_gl_account() -> (Ctx, Choice) {
        let db = Database::open_in_memory().unwrap();
        db.run_migrations(organization_repository::MIGRATIONS).unwrap();
        db.run_migrations(accounting_repository::MIGRATIONS).unwrap();
        db.run_migrations(crate::backend::audit::audit_logs::MIGRATIONS).unwrap();
        db.run_migrations(banking_repository::MIGRATIONS).unwrap();

        let org_repo = OrganizationRepository::new(db.clone());
        let company_id = org_repo.create_company(&Company::new("Acme Trading Co", "USD")).unwrap().id.unwrap();

        let accounting_repo = AccountingRepository::new(db.clone());
        let cash = accounting_repo.create_account(&Account::new(company_id, "1000", "Cash", AccountType::Asset)).unwrap();

        let ctx = Ctx { db, company_id, current_user_id: None, export_dir: PathBuf::from(".") };
        let choice = Choice { id: cash.id.unwrap(), label: "1000 - Cash".into() };
        (ctx, choice)
    }

    #[test]
    fn field_changed_messages_update_the_corresponding_field() {
        let (ctx, gl_account) = ctx_with_a_gl_account();
        let mut state = State::default();

        update(&mut state, Message::GlAccountSelected(gl_account.clone()), &ctx);
        update(&mut state, Message::BankNameChanged("First National".into()), &ctx);
        update(&mut state, Message::AccountNumberChanged("12345".into()), &ctx);

        assert_eq!(state.gl_account, Some(gl_account));
        assert_eq!(state.bank_name_input, "First National");
        assert_eq!(state.account_number_input, "12345");
    }

    #[test]
    fn creating_without_a_gl_account_sets_an_error() {
        let (ctx, _gl_account) = ctx_with_a_gl_account();
        let mut state = State { bank_name_input: "First National".into(), ..Default::default() };

        update(&mut state, Message::Create, &ctx);

        assert_eq!(state.error.as_deref(), Some("choose a GL account"));
        assert!(state.bank_accounts.is_empty());
    }

    #[test]
    fn creating_a_valid_bank_account_adds_it_with_a_zero_balance_and_clears_the_form() {
        let (ctx, gl_account) = ctx_with_a_gl_account();
        let mut state = State {
            gl_account: Some(gl_account),
            bank_name_input: "First National".into(),
            account_number_input: "12345".into(),
            ..Default::default()
        };

        update(&mut state, Message::Create, &ctx);

        assert!(state.error.is_none());
        assert_eq!(state.bank_accounts.len(), 1);
        assert_eq!(state.bank_accounts[0].0.bank_name, "First National");
        assert_eq!(state.bank_accounts[0].1, "0");
        assert!(state.bank_name_input.is_empty());
        assert!(state.account_number_input.is_empty());
    }

    #[test]
    fn search_changed_updates_the_search_field() {
        let (ctx, _gl_account) = ctx_with_a_gl_account();
        let mut state = State::default();

        update(&mut state, Message::SearchChanged("first".into()), &ctx);

        assert_eq!(state.search, "first");
    }
}
