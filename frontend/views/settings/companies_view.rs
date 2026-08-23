//! Multi-company: list every company, create additional ones, and
//! switch which one the rest of the app is working in. Company
//! *editing* (name/legal name) stays on `settings_view.rs`; this is
//! specifically the "which books am I in, and how do I get more of
//! them" screen -- part of onboarding, not yet a full company admin
//! screen (no delete/archive here).

use iced::widget::{button, column, container, row, scrollable, text, text_input};
use iced::{Element, Length};

use crate::backend::organization::companies::Company;
use crate::backend::services::organization_service::OrganizationService;
use crate::frontend::styles::colors;
use crate::frontend::views::Ctx;

#[derive(Debug, Clone, Default)]
pub struct State {
    companies: Vec<Company>,
    name_input: String,
    error: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    NameChanged(String),
    Create,
    Switch(i64),
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
    }
}

pub fn view(state: &State) -> Element<'_, Message> {
    let mut list = column![row![
        text("Company").width(Length::Fill),
        text("Currency").width(Length::Fixed(80.0)),
        text("").width(Length::Fixed(100.0)),
    ]]
    .spacing(4);

    for company in &state.companies {
        list = list.push(row![
            text(company.name.clone()).width(Length::Fill),
            text(company.base_currency_code.clone()).width(Length::Fixed(80.0)),
            button(text("Switch to")).on_press(Message::Switch(company.id.unwrap_or_default())).padding(4).width(Length::Fixed(100.0)),
        ]);
    }

    let form = row![
        text_input("New company name", &state.name_input).on_input(Message::NameChanged).width(Length::Fill).padding(6),
        button(text("Create company")).on_press(Message::Create).padding(6),
    ]
    .spacing(8);

    let mut content = column![text("Companies").size(24), scrollable(list).height(Length::Fill), form]
        .spacing(16)
        .padding(20);

    if let Some(error) = &state.error {
        content = content.push(text(error.clone()).color(colors::WHITE_NEGATIVE_FRONT));
    }

    container(content).into()
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
}
