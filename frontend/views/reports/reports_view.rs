//! Balance sheet and profit & loss, with CSV export. Trial balance has
//! its own screen (`accounting::trial_balance_view`); this covers the
//! two reports (Development Order step 15) that need fresh aggregation
//! rather than just formatting an existing domain type.

use chrono::{Datelike, NaiveDate, Utc};

use iced::widget::{button, column, container, row, scrollable, text};
use iced::{Element, Length};

use crate::backend::reports::{balance_sheet, profit_loss};
use crate::backend::repositories::accounting_repository::AccountingRepository;
use crate::database::exports::csv;
use crate::frontend::styles::colors;
use crate::frontend::views::Ctx;

#[derive(Debug, Clone, Default)]
pub struct State {
    balance_sheet: Option<balance_sheet::BalanceSheet>,
    profit_and_loss: Option<profit_loss::ProfitAndLoss>,
    export_message: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    Refresh,
    ExportBalanceSheetCsv,
    ExportProfitAndLossCsv,
}

fn year_start(today: NaiveDate) -> NaiveDate {
    NaiveDate::from_ymd_opt(today.year(), 1, 1).unwrap_or(today)
}

pub fn load(ctx: &Ctx) -> State {
    let repo = AccountingRepository::new(ctx.db.clone());
    let today = Utc::now().date_naive();
    let period_start = year_start(today);

    let accounts = match repo.list_accounts_for_company(ctx.company_id) {
        Ok(accounts) => accounts,
        Err(err) => return State { error: Some(err.to_string()), ..Default::default() },
    };
    let trial_balance = match repo.trial_balance_as_of(ctx.company_id, today) {
        Ok(tb) => tb,
        Err(err) => return State { error: Some(err.to_string()), ..Default::default() },
    };
    let pl_balances = match repo.revenue_and_expense_balances_as_of(ctx.company_id, today) {
        Ok(balances) => balances,
        Err(err) => return State { error: Some(err.to_string()), ..Default::default() },
    };

    let account_lookup = |account_id: i64| accounts.iter().find(|a| a.id == Some(account_id));

    let balance_sheet_input: Vec<_> = trial_balance
        .lines
        .iter()
        .filter_map(|line| {
            let account = account_lookup(line.account_id)?;
            Some((line.account_id, line.account_code.clone(), line.account_name.clone(), account.account_type, line.debit - line.credit))
        })
        .collect();

    let pl_accounts: Vec<(String, String)> = pl_balances
        .iter()
        .filter_map(|b| account_lookup(b.account_id).map(|a| (a.code.clone(), a.name.clone())))
        .collect();
    let profit_and_loss = profit_loss::build(period_start, today, &pl_accounts, &pl_balances);
    let current_year_earnings = profit_and_loss.net_income();

    State {
        balance_sheet: Some(balance_sheet::build(today, &balance_sheet_input, current_year_earnings)),
        profit_and_loss: Some(profit_and_loss),
        export_message: None,
        error: None,
    }
}

pub fn update(state: &mut State, message: Message, ctx: &Ctx) {
    match message {
        Message::Refresh => *state = load(ctx),
        Message::ExportBalanceSheetCsv => {
            let Some(report) = &state.balance_sheet else { return };
            let (headers, rows) = report.to_rows();
            let path = ctx.export_path("balance_sheet.csv");
            match std::fs::write(&path, csv::export_csv(&headers, &rows)) {
                Ok(()) => state.export_message = Some(format!("Exported to {}", path.display())),
                Err(err) => state.error = Some(err.to_string()),
            }
        }
        Message::ExportProfitAndLossCsv => {
            let Some(report) = &state.profit_and_loss else { return };
            let (headers, rows) = report.to_rows();
            let path = ctx.export_path("profit_and_loss.csv");
            match std::fs::write(&path, csv::export_csv(&headers, &rows)) {
                Ok(()) => state.export_message = Some(format!("Exported to {}", path.display())),
                Err(err) => state.error = Some(err.to_string()),
            }
        }
    }
}

pub fn view(state: &State) -> Element<'_, Message> {
    if let Some(error) = &state.error {
        return container(text(format!("Could not load reports: {error}")).color(colors::WHITE_NEGATIVE_FRONT)).padding(20).into();
    }
    let (Some(balance_sheet), Some(profit_and_loss)) = (&state.balance_sheet, &state.profit_and_loss) else {
        return container(text("Loading...")).padding(20).into();
    };

    let mut bs_column = column![
        text("Balance Sheet").size(20),
        text(format!("Total assets: {}", balance_sheet.total_assets())),
        text(format!("Total liabilities: {}", balance_sheet.total_liabilities())),
        text(format!("Total equity (incl. current year earnings): {}", balance_sheet.total_equity())),
    ]
    .spacing(6);
    bs_column = bs_column.push(if balance_sheet.is_balanced() {
        text("Balanced").color(colors::WHITE_POSITIVE_FRONT)
    } else {
        text("Not balanced").color(colors::WHITE_NEGATIVE_FRONT)
    });
    bs_column = bs_column.push(button(text("Export CSV")).on_press(Message::ExportBalanceSheetCsv).padding(6));

    let pl_column = column![
        text("Profit & Loss (year to date)").size(20),
        text(format!("Total revenue: {}", profit_and_loss.total_revenue())),
        text(format!("Total expenses: {}", profit_and_loss.total_expenses())),
        text(format!("Net income: {}", profit_and_loss.net_income())),
        button(text("Export CSV")).on_press(Message::ExportProfitAndLossCsv).padding(6),
    ]
    .spacing(6);

    let mut content = column![text("Reports").size(24), row![bs_column, pl_column].spacing(40)].spacing(16).padding(20);

    if let Some(export_message) = &state.export_message {
        content = content.push(text(export_message.clone()).color(colors::WHITE_POSITIVE_FRONT));
    }

    scrollable(container(content).width(Length::Fill)).into()
}
