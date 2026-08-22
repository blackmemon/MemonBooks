//! List tax rates and add new ones. Grouped under `accounting/` -- the
//! frozen frontend/views/ structure (Section 2) has no dedicated `tax/`
//! directory, and tax rates are closely tied to the chart of accounts.

use rust_decimal::Decimal;

use iced::widget::{button, column, container, pick_list, row, scrollable, text, text_input};
use iced::{Element, Length};

use crate::backend::accounting::chart_of_accounts::Account;
use crate::backend::services::tax_service::TaxService;
use crate::backend::tax::tax_rates::{TaxDirection, TaxRate};
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

const DIRECTIONS: [TaxDirection; 2] = [TaxDirection::Output, TaxDirection::Input];

#[derive(Debug, Clone, Default)]
pub struct State {
    tax_rates: Vec<TaxRate>,
    gl_accounts: Vec<Choice>,
    gl_account: Option<Choice>,
    direction: Option<TaxDirection>,
    name_input: String,
    rate_input: String,
    error: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    NameChanged(String),
    RateChanged(String),
    GlAccountSelected(Choice),
    DirectionSelected(TaxDirection),
    Create,
}

pub fn load(ctx: &Ctx) -> State {
    let service = TaxService::new(ctx.db.clone());

    let tax_rates = service.list_tax_rates(ctx.company_id).unwrap_or_default();
    let gl_accounts = service
        .list_gl_accounts(ctx.company_id)
        .map(|accounts: Vec<Account>| accounts.iter().map(|a| Choice { id: a.id.unwrap_or_default(), label: format!("{} - {}", a.code, a.name) }).collect())
        .unwrap_or_default();

    State { tax_rates, gl_accounts, ..Default::default() }
}

pub fn update(state: &mut State, message: Message, ctx: &Ctx) {
    match message {
        Message::NameChanged(value) => state.name_input = value,
        Message::RateChanged(value) => state.rate_input = value,
        Message::GlAccountSelected(choice) => state.gl_account = Some(choice),
        Message::DirectionSelected(direction) => state.direction = Some(direction),
        Message::Create => {
            let (Some(gl_account), Some(direction)) = (&state.gl_account, state.direction) else {
                state.error = Some("choose a GL account and a direction".into());
                return;
            };
            let rate_percent: Decimal = match state.rate_input.parse() {
                Ok(rate) => rate,
                Err(_) => {
                    state.error = Some("rate is not a valid number".into());
                    return;
                }
            };

            let tax_rate = TaxRate::new(ctx.company_id, state.name_input.clone(), rate_percent, direction, gl_account.id);
            let service = TaxService::new(ctx.db.clone());
            match service.create_tax_rate(&tax_rate) {
                Ok(created) => {
                    state.tax_rates.push(created);
                    state.name_input.clear();
                    state.rate_input.clear();
                    state.error = None;
                }
                Err(err) => state.error = Some(err.to_string()),
            }
        }
    }
}

pub fn view(state: &State) -> Element<'_, Message> {
    let mut list = column![row![
        text("Name").width(Length::Fill),
        text("Rate %").width(Length::Fixed(80.0)),
        text("Direction").width(Length::Fixed(80.0)),
    ]]
    .spacing(4);

    for rate in &state.tax_rates {
        list = list.push(row![
            text(rate.name.clone()).width(Length::Fill),
            text(rate.rate_percent.to_string()).width(Length::Fixed(80.0)),
            text(rate.direction.as_str()).width(Length::Fixed(80.0)),
        ]);
    }

    let form = column![
        row![
            text_input("Name", &state.name_input).on_input(Message::NameChanged).width(Length::Fill).padding(6),
            text_input("Rate %", &state.rate_input).on_input(Message::RateChanged).width(Length::Fixed(80.0)).padding(6),
        ]
        .spacing(8),
        row![
            pick_list(DIRECTIONS, state.direction, Message::DirectionSelected).placeholder("Direction"),
            pick_list(state.gl_accounts.as_slice(), state.gl_account.clone(), Message::GlAccountSelected).placeholder("GL account"),
            button(text("Add tax rate")).on_press(Message::Create).padding(6),
        ]
        .spacing(8),
    ]
    .spacing(8);

    let mut content = column![text("Tax Rates").size(24), scrollable(list).height(Length::Fill), form]
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
    use crate::backend::accounting::accounting_rules::AccountType;
    use crate::backend::organization::companies::Company;
    use crate::backend::repositories::accounting_repository::{self, AccountingRepository};
    use crate::backend::repositories::organization_repository::{self, OrganizationRepository};
    use crate::backend::repositories::tax_repository;
    use crate::database::database::Database;
    use std::path::PathBuf;

    fn ctx_with_a_gl_account() -> (Ctx, Choice) {
        let db = Database::open_in_memory().unwrap();
        db.run_migrations(organization_repository::MIGRATIONS).unwrap();
        db.run_migrations(accounting_repository::MIGRATIONS).unwrap();
        db.run_migrations(crate::backend::audit::audit_logs::MIGRATIONS).unwrap();
        db.run_migrations(tax_repository::MIGRATIONS).unwrap();

        let org_repo = OrganizationRepository::new(db.clone());
        let company_id = org_repo.create_company(&Company::new("Acme Trading Co", "USD")).unwrap().id.unwrap();

        let accounting_repo = AccountingRepository::new(db.clone());
        let tax_payable = accounting_repo.create_account(&Account::new(company_id, "2200", "Tax Payable", AccountType::Liability)).unwrap();

        let ctx = Ctx { db, company_id, current_user_id: None, export_dir: PathBuf::from(".") };
        let choice = Choice { id: tax_payable.id.unwrap(), label: "2200 - Tax Payable".into() };
        (ctx, choice)
    }

    #[test]
    fn field_changed_messages_update_the_corresponding_field() {
        let (ctx, gl_account) = ctx_with_a_gl_account();
        let mut state = State::default();

        update(&mut state, Message::NameChanged("Sales Tax".into()), &ctx);
        update(&mut state, Message::RateChanged("15".into()), &ctx);
        update(&mut state, Message::GlAccountSelected(gl_account.clone()), &ctx);
        update(&mut state, Message::DirectionSelected(TaxDirection::Output), &ctx);

        assert_eq!(state.name_input, "Sales Tax");
        assert_eq!(state.rate_input, "15");
        assert_eq!(state.gl_account, Some(gl_account));
        assert_eq!(state.direction, Some(TaxDirection::Output));
    }

    #[test]
    fn creating_without_a_gl_account_or_direction_sets_an_error() {
        let (ctx, _gl_account) = ctx_with_a_gl_account();
        let mut state = State { name_input: "Sales Tax".into(), rate_input: "15".into(), ..Default::default() };

        update(&mut state, Message::Create, &ctx);

        assert_eq!(state.error.as_deref(), Some("choose a GL account and a direction"));
    }

    #[test]
    fn creating_with_a_non_numeric_rate_sets_an_error() {
        let (ctx, gl_account) = ctx_with_a_gl_account();
        let mut state = State {
            name_input: "Sales Tax".into(),
            rate_input: "not a number".into(),
            gl_account: Some(gl_account),
            direction: Some(TaxDirection::Output),
            ..Default::default()
        };

        update(&mut state, Message::Create, &ctx);

        assert_eq!(state.error.as_deref(), Some("rate is not a valid number"));
    }

    #[test]
    fn creating_a_valid_tax_rate_adds_it_to_the_list_and_clears_the_form() {
        let (ctx, gl_account) = ctx_with_a_gl_account();
        let mut state = State {
            name_input: "Sales Tax".into(),
            rate_input: "15".into(),
            gl_account: Some(gl_account),
            direction: Some(TaxDirection::Output),
            ..Default::default()
        };

        update(&mut state, Message::Create, &ctx);

        assert!(state.error.is_none());
        assert_eq!(state.tax_rates.len(), 1);
        assert_eq!(state.tax_rates[0].name, "Sales Tax");
        assert!(state.name_input.is_empty());
        assert!(state.rate_input.is_empty());
    }
}
