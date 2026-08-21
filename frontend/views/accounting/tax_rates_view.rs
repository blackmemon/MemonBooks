//! List tax rates and add new ones. Grouped under `accounting/` -- the
//! frozen frontend/views/ structure (Section 2) has no dedicated `tax/`
//! directory, and tax rates are closely tied to the chart of accounts.

use rust_decimal::Decimal;

use iced::widget::{button, column, container, pick_list, row, scrollable, text, text_input};
use iced::{Element, Length};

use crate::backend::accounting::chart_of_accounts::Account;
use crate::backend::repositories::accounting_repository::AccountingRepository;
use crate::backend::repositories::tax_repository::TaxRepository;
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
    let tax_repo = TaxRepository::new(ctx.db.clone());
    let accounting_repo = AccountingRepository::new(ctx.db.clone());

    let tax_rates = tax_repo.list_tax_rates_for_company(ctx.company_id).unwrap_or_default();
    let gl_accounts = accounting_repo
        .list_accounts_for_company(ctx.company_id)
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
            let repo = TaxRepository::new(ctx.db.clone());
            match repo.create_tax_rate(&tax_rate) {
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
