//! The landing screen after login: a snapshot of the company's books.

use chrono::Utc;
use iced::widget::{column, container, row, text};
use iced::{Element, Length};

use crate::backend::core::errors::AppResult;
use crate::backend::repositories::accounting_repository::AccountingRepository;
use crate::frontend::styles::colors;
use crate::frontend::views::Ctx;

#[derive(Debug, Clone, Default)]
pub struct State {
    pub total_assets: String,
    pub total_liabilities: String,
    pub total_equity: String,
    pub open_receivables_count: usize,
    pub open_payables_count: usize,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    Refresh,
}

pub fn load(ctx: &Ctx) -> State {
    match load_summary(ctx) {
        Ok(state) => state,
        Err(err) => State {
            error: Some(err.to_string()),
            ..Default::default()
        },
    }
}

fn load_summary(ctx: &Ctx) -> AppResult<State> {
    let accounting = AccountingRepository::new(ctx.db.clone());
    let today = Utc::now().date_naive();
    let trial_balance = accounting.trial_balance_as_of(ctx.company_id, today)?;

    Ok(State {
        total_assets: trial_balance.total_debits().to_string(),
        total_liabilities: String::new(),
        total_equity: trial_balance.total_credits().to_string(),
        open_receivables_count: accounting.list_open_receivables_for_company(ctx.company_id)?.len(),
        open_payables_count: accounting.list_open_payables_for_company(ctx.company_id)?.len(),
        error: None,
    })
}

pub fn update(state: &mut State, message: Message, ctx: &Ctx) {
    match message {
        Message::Refresh => *state = load(ctx),
    }
}

fn summary_card(label: impl Into<String>, value: impl Into<String>) -> Element<'static, Message> {
    container(
        column![text(label.into()).size(14).color(colors::WHITE_PRIMARY), text(value.into()).size(24)].spacing(6),
    )
    .padding(16)
    .width(Length::FillPortion(1))
    .into()
}

pub fn view(state: &State) -> Element<'_, Message> {
    if let Some(error) = &state.error {
        return container(text(format!("Could not load dashboard: {error}")).color(colors::WHITE_NEGATIVE_FRONT))
            .padding(20)
            .into();
    }

    column![
        text("Dashboard").size(24),
        row![
            summary_card("Total Debits (Trial Balance)", state.total_assets.clone()),
            summary_card("Total Credits (Trial Balance)", state.total_equity.clone()),
        ]
        .spacing(16),
        row![
            summary_card("Open Receivables", state.open_receivables_count.to_string()),
            summary_card("Open Payables", state.open_payables_count.to_string()),
        ]
        .spacing(16),
    ]
    .spacing(20)
    .padding(20)
    .into()
}
