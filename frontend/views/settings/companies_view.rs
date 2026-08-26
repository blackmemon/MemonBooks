//! Multi-company: list every company, create additional ones, and
//! switch which one the rest of the app is working in. Company
//! *editing* (name/legal name) stays on `settings_view.rs`; this is
//! specifically the "which books am I in, and how do I get more of
//! them" screen -- part of onboarding, not yet a full company admin
//! screen (no delete/archive here).

use iced::widget::{button, column, container, scrollable, text, text_input};
use iced::{Element, Length};

use crate::backend::organization::companies::Company;
use crate::backend::services::organization_service::OrganizationService;
use crate::frontend::components::common::data_table::{self, Cell, ColumnDef, Row, SortDirection};
use crate::frontend::components::common::{page_header, section_heading, toolbar};
use crate::frontend::styles::colors::Theme;
use crate::frontend::styles::button as button_style;
use crate::frontend::views::Ctx;

#[derive(Debug, Clone, Default)]
pub struct State {
    companies: Vec<Company>,
    name_input: String,
    search: String,
    error: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    NameChanged(String),
    Create,
    Switch(i64),
    SearchChanged(String),
    TabSelected(usize),
    FiltersClicked,
    SortClicked,
    RowAction(usize),
}

pub enum Event {
    None,
    Switched(i64),
}

pub fn load(ctx: &Ctx) -> State {
    let service = OrganizationService::new(ctx.db.clone());
    match service.list_companies() {
        Ok(companies) => State { companies, ..Default::default() },
        Err(err) => State { error: Some(err.to_string()), ..Default::default() },
    }
}

pub fn update(state: &mut State, message: Message, ctx: &Ctx) -> Event {
    match message {
        Message::NameChanged(value) => {
            state.name_input = value;
            Event::None
        }
        Message::Create => {
            let service = OrganizationService::new(ctx.db.clone());
            match service.create_company(state.name_input.clone(), "USD") {
                Ok(created) => {
                    state.companies.push(created);
                    state.companies.sort_by(|a, b| a.name.cmp(&b.name));
                    state.name_input.clear();
                    state.error = None;
                    Event::None
                }
                Err(err) => {
                    state.error = Some(err.to_string());
                    Event::None
                }
            }
        }
        Message::Switch(company_id) => Event::Switched(company_id),
        Message::SearchChanged(value) => {
            state.search = value;
            Event::None
        }
        // This view only has one tab, "Filters" / "Sort by" are rendered
        // by the shared toolbar component now (Section 6), and the kebab
        // menu has no secondary action yet -- wiring real tab/filter/
        // sort/secondary-action behavior is follow-up work, not part of
        // this layout pass. Switching companies is this row's primary
        // action (the visible button), not a kebab action.
        Message::TabSelected(_) | Message::FiltersClicked | Message::SortClicked | Message::RowAction(_) => Event::None,
    }
}

pub fn view(state: &State, theme: Theme) -> Element<'_, Message> {
    let header = page_header::view(theme, "Companies", "Every company on this installation's books.", "Create company", Message::Create);

    let toolbar = toolbar::view(
        theme,
        &["All Companies"],
        0,
        Message::TabSelected,
        &state.search,
        "Search companies",
        Message::SearchChanged,
        "Filters",
        Message::FiltersClicked,
        "Sort by",
        Message::SortClicked,
    );

    let heading = section_heading::view(theme, "Company list", "Click a company's row to switch the rest of the app into it.");

    let query = state.search.trim().to_lowercase();
    let filtered: Vec<&Company> = state.companies.iter().filter(|company| query.is_empty() || company.name.to_lowercase().contains(&query)).collect();

    let columns = vec![
        ColumnDef::new("Company", Length::FillPortion(3)).sorted(SortDirection::Ascending),
        ColumnDef::new("Currency", Length::FillPortion(1)),
    ];

    let rows = filtered
        .iter()
        .map(|company| Row::new(vec![Cell::text(company.name.clone()), Cell::text(company.base_currency_code.clone())]).with_primary_action("Switch to"))
        .collect();

    let company_ids: Vec<i64> = filtered.iter().map(|company| company.id.unwrap_or_default()).collect();
    let table = data_table::view(theme, columns, rows, None, |_index, _checked| Message::RowAction(0), |_checked| Message::RowAction(0), Message::RowAction, move |index| Message::Switch(company_ids[index]));

    let form = column![
        text("Create a company").size(13),
        text_input("New company name", &state.name_input).on_input(Message::NameChanged).width(Length::Fill).padding(8),
        button(text("Create company")).style(button_style::primary(theme)).on_press(Message::Create).padding(8),
    ]
    .spacing(8);

    let mut content = column![header, toolbar, heading, table, form].spacing(20).padding(24).width(Length::Fill);

    if let Some(error) = &state.error {
        content = content.push(text(error.clone()).color(theme.negative_front()));
    }

    container(scrollable(content).height(Length::Fill)).width(Length::Fill).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::repositories::accounting_repository;
    use crate::backend::repositories::organization_repository::{self, OrganizationRepository};
    use crate::database::database::Database;
    use std::path::PathBuf;

    fn ctx_with_a_company() -> (Ctx, i64) {
        let db = Database::open_in_memory().unwrap();
        db.run_migrations(organization_repository::MIGRATIONS).unwrap();
        db.run_migrations(accounting_repository::MIGRATIONS).unwrap();
        db.run_migrations(crate::backend::audit::audit_logs::MIGRATIONS).unwrap();

        let org_repo = OrganizationRepository::new(db.clone());
        let company_id = org_repo.create_company(&Company::new("Acme Trading Co", "USD")).unwrap().id.unwrap();

        let ctx = Ctx { db, company_id, current_user_id: None, export_dir: PathBuf::from(".") };
        (ctx, company_id)
    }

    #[test]
    fn field_changed_messages_update_the_corresponding_field() {
        let (ctx, _company_id) = ctx_with_a_company();
        let mut state = State::default();

        update(&mut state, Message::NameChanged("Acme Holdings".into()), &ctx);

        assert_eq!(state.name_input, "Acme Holdings");
    }

    #[test]
    fn creating_with_a_blank_name_sets_an_error() {
        let (ctx, _company_id) = ctx_with_a_company();
        let mut state = State { name_input: "".into(), ..Default::default() };

        let event = update(&mut state, Message::Create, &ctx);

        assert!(matches!(event, Event::None));
        assert!(state.error.is_some());
    }

    #[test]
    fn creating_a_valid_company_adds_it_with_an_open_fiscal_period() {
        let (ctx, _company_id) = ctx_with_a_company();
        let mut state = State { name_input: "Acme Holdings".into(), ..Default::default() };

        let event = update(&mut state, Message::Create, &ctx);

        assert!(matches!(event, Event::None));
        assert!(state.error.is_none());
        assert_eq!(state.companies.len(), 1);
        assert_eq!(state.companies[0].name, "Acme Holdings");
        assert!(state.name_input.is_empty());

        // The new company has a usable, open fiscal period, same as
        // bootstrap -- proven by successfully posting to it, which
        // requires a fiscal period that accepts postings on today's date.
        use crate::backend::accounting::accounting_rules::AccountType;
        use crate::backend::accounting::chart_of_accounts::Account;
        use crate::backend::repositories::accounting_repository::AccountingRepository;
        use crate::backend::transactions::transaction::Transaction;
        use crate::backend::transactions::transaction_lines::TransactionLine;
        use crate::backend::transactions::transaction_types::TransactionType;

        let new_company_id = state.companies[0].id.unwrap();
        let accounting = AccountingRepository::new(ctx.db.clone());
        let cash = accounting.create_account(&Account::new(new_company_id, "1000", "Cash", AccountType::Asset)).unwrap();
        let capital = accounting.create_account(&Account::new(new_company_id, "3000", "Owner's Capital", AccountType::Equity)).unwrap();
        let today = chrono::Utc::now().date_naive();
        let posted = accounting.post_transaction(
            &Transaction::new(new_company_id, TransactionType::OpeningBalance, today)
                .with_line(TransactionLine::debit(cash.id.unwrap(), rust_decimal_macros::dec!(100)))
                .with_line(TransactionLine::credit(capital.id.unwrap(), rust_decimal_macros::dec!(100))),
        );
        assert!(posted.is_ok());
    }

    #[test]
    fn switching_emits_the_selected_companys_id() {
        let (ctx, company_id) = ctx_with_a_company();
        let mut state = load(&ctx);

        let event = update(&mut state, Message::Switch(company_id), &ctx);

        assert!(matches!(event, Event::Switched(id) if id == company_id));
    }

    #[test]
    fn search_changed_updates_the_search_field() {
        let (ctx, _company_id) = ctx_with_a_company();
        let mut state = State::default();

        update(&mut state, Message::SearchChanged("acme".into()), &ctx);

        assert_eq!(state.search, "acme");
    }
}
