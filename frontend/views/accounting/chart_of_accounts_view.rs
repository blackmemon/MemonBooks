//! List the chart of accounts and add new accounts to it.

use iced::widget::{button, column, container, pick_list, scrollable, text, text_input};
use iced::{Element, Length};

use crate::backend::accounting::accounting_rules::AccountType;
use crate::backend::accounting::chart_of_accounts::Account;
use crate::backend::core::validation::Validate;
use crate::backend::services::accounting_service::AccountingService;
use crate::frontend::components::common::data_table::{self, Cell, ColumnDef, Row, SortDirection};
use crate::frontend::components::common::{page_header, section_heading, toolbar};
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
    search: String,
    error: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    CodeChanged(String),
    NameChanged(String),
    TypeSelected(AccountType),
    Create,
    SearchChanged(String),
    TabSelected(usize),
    FiltersClicked,
    SortClicked,
    RowAction(usize),
}

pub fn load(ctx: &Ctx) -> State {
    let service = AccountingService::new(ctx.db.clone());
    match service.list_accounts(ctx.company_id) {
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
            let service = AccountingService::new(ctx.db.clone());
            match service.create_account(&account) {
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
    let header = page_header::view("Chart of Accounts", "Every general ledger account on this company's books.", "Add account", Message::Create);

    let toolbar = toolbar::view(
        &["All Accounts"],
        0,
        Message::TabSelected,
        &state.search,
        "Search accounts",
        Message::SearchChanged,
        "Filters",
        Message::FiltersClicked,
        "Sort by",
        Message::SortClicked,
    );

    let heading = section_heading::view("Account list", "Every account, grouped by code order.");

    let query = state.search.trim().to_lowercase();
    let filtered: Vec<&Account> =
        state.accounts.iter().filter(|account| query.is_empty() || account.name.to_lowercase().contains(&query) || account.code.to_lowercase().contains(&query)).collect();

    let columns = vec![
        ColumnDef::new("Code", Length::FillPortion(1)).sorted(SortDirection::Ascending),
        ColumnDef::new("Name", Length::FillPortion(3)),
        ColumnDef::new("Type", Length::FillPortion(1)),
    ];

    let rows = filtered.iter().map(|account| Row::new(vec![Cell::text(account.code.clone()), Cell::text(account.name.clone()), Cell::text(account.account_type.as_str())])).collect();

    let table = data_table::view(columns, rows, None, |_index, _checked| Message::RowAction(0), Message::RowAction);

    let form = column![
        text("Add an account").size(13),
        text_input("Code", &state.code_input).on_input(Message::CodeChanged).width(Length::Fill).padding(8),
        text_input("Name", &state.name_input).on_input(Message::NameChanged).width(Length::Fill).padding(8),
        pick_list(ACCOUNT_TYPES, state.type_input, Message::TypeSelected).placeholder("Type"),
        button(text("Add account")).on_press(Message::Create).padding(8),
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
    use crate::backend::organization::companies::Company;
    use crate::backend::repositories::accounting_repository;
    use crate::backend::repositories::organization_repository::{self, OrganizationRepository};
    use crate::database::database::Database;
    use std::path::PathBuf;

    fn ctx_with_company() -> Ctx {
        let db = Database::open_in_memory().unwrap();
        db.run_migrations(organization_repository::MIGRATIONS).unwrap();
        db.run_migrations(accounting_repository::MIGRATIONS).unwrap();
        db.run_migrations(crate::backend::audit::audit_logs::MIGRATIONS).unwrap();

        let org_repo = OrganizationRepository::new(db.clone());
        let company_id = org_repo.create_company(&Company::new("Acme Trading Co", "USD")).unwrap().id.unwrap();

        Ctx { db, company_id, current_user_id: None, export_dir: PathBuf::from(".") }
    }

    #[test]
    fn field_changed_messages_update_the_corresponding_field() {
        let ctx = ctx_with_company();
        let mut state = State::default();

        update(&mut state, Message::CodeChanged("1000".into()), &ctx);
        update(&mut state, Message::NameChanged("Cash".into()), &ctx);
        update(&mut state, Message::TypeSelected(AccountType::Asset), &ctx);

        assert_eq!(state.code_input, "1000");
        assert_eq!(state.name_input, "Cash");
        assert_eq!(state.type_input, Some(AccountType::Asset));
    }

    #[test]
    fn creating_without_choosing_a_type_sets_an_error() {
        let ctx = ctx_with_company();
        let mut state = State { code_input: "1000".into(), name_input: "Cash".into(), ..Default::default() };

        update(&mut state, Message::Create, &ctx);

        assert_eq!(state.error.as_deref(), Some("choose an account type"));
        assert!(state.accounts.is_empty());
    }

    #[test]
    fn creating_a_valid_account_adds_it_to_the_list_and_clears_the_form() {
        let ctx = ctx_with_company();
        let mut state = State { code_input: "1000".into(), name_input: "Cash".into(), type_input: Some(AccountType::Asset), ..Default::default() };

        update(&mut state, Message::Create, &ctx);

        assert!(state.error.is_none());
        assert_eq!(state.accounts.len(), 1);
        assert_eq!(state.accounts[0].code, "1000");
        assert!(state.code_input.is_empty());
        assert!(state.name_input.is_empty());
        assert!(state.type_input.is_none());
    }

    #[test]
    fn search_changed_updates_the_search_field() {
        let ctx = ctx_with_company();
        let mut state = State::default();

        update(&mut state, Message::SearchChanged("cash".into()), &ctx);

        assert_eq!(state.search, "cash");
    }
}
