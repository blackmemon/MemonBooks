//! Display the trial balance (Development Order step 7) as of today, and
//! export it to CSV.

use chrono::Utc;

use iced::widget::{button, column, container, row, scrollable, text};
use iced::{Element, Length};

use crate::backend::accounting::trial_balance::TrialBalance;
use crate::backend::repositories::accounting_repository::AccountingRepository;
use crate::backend::reports::trial_balance as trial_balance_report;
use crate::database::exports::csv;
use crate::frontend::styles::colors;
use crate::frontend::views::Ctx;

#[derive(Debug, Clone, Default)]
pub struct State {
    report: Option<TrialBalance>,
    export_message: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    Refresh,
    ExportCsv,
}

pub fn load(ctx: &Ctx) -> State {
    let repo = AccountingRepository::new(ctx.db.clone());
    let today = Utc::now().date_naive();
    match repo.trial_balance_as_of(ctx.company_id, today) {
        Ok(report) => State { report: Some(report), export_message: None, error: None },
        Err(err) => State { error: Some(err.to_string()), ..Default::default() },
    }
}

pub fn update(state: &mut State, message: Message, ctx: &Ctx) {
    match message {
        Message::Refresh => *state = load(ctx),
        Message::ExportCsv => {
            let Some(report) = &state.report else { return };
            let (headers, rows) = trial_balance_report::to_rows(report);
            let csv_text = csv::export_csv(&headers, &rows);
            let path = ctx.export_path("trial_balance.csv");
            match std::fs::write(&path, csv_text) {
                Ok(()) => state.export_message = Some(format!("Exported to {}", path.display())),
                Err(err) => state.error = Some(err.to_string()),
            }
        }
    }
}

pub fn view(state: &State) -> Element<'_, Message> {
    let Some(report) = &state.report else {
        let message = state.error.clone().unwrap_or_else(|| "Loading...".to_string());
        return container(text(message).color(colors::WHITE_NEGATIVE_FRONT)).padding(20).into();
    };

    let mut list = column![row![
        text("Code").width(Length::Fixed(80.0)),
        text("Account").width(Length::Fill),
        text("Debit").width(Length::Fixed(100.0)),
        text("Credit").width(Length::Fixed(100.0)),
    ]]
    .spacing(4);

    for line in &report.lines {
        list = list.push(row![
            text(line.account_code.clone()).width(Length::Fixed(80.0)),
            text(line.account_name.clone()).width(Length::Fill),
            text(line.debit.to_string()).width(Length::Fixed(100.0)),
            text(line.credit.to_string()).width(Length::Fixed(100.0)),
        ]);
    }
    list = list.push(row![
        text("").width(Length::Fixed(80.0)),
        text("Total").width(Length::Fill),
        text(report.total_debits().to_string()).width(Length::Fixed(100.0)),
        text(report.total_credits().to_string()).width(Length::Fixed(100.0)),
    ]);

    let balanced_note = if report.is_balanced() {
        text("Balanced").color(colors::WHITE_POSITIVE_FRONT)
    } else {
        text("Not balanced").color(colors::WHITE_NEGATIVE_FRONT)
    };

    let mut content = column![
        text("Trial Balance").size(24),
        balanced_note,
        scrollable(list).height(Length::Fill),
        button(text("Export CSV")).on_press(Message::ExportCsv).padding(8),
    ]
    .spacing(16)
    .padding(20);

    if let Some(export_message) = &state.export_message {
        content = content.push(text(export_message.clone()).color(colors::WHITE_POSITIVE_FRONT));
    }

    container(content).into()
}
