//! Balance sheet and profit & loss, with CSV export. Trial balance has
//! its own screen (`accounting::trial_balance_view`); this covers the
//! two reports (Development Order step 15) that need fresh aggregation
//! rather than just formatting an existing domain type.

use chrono::{Datelike, NaiveDate, Utc};

use iced::widget::{button, column, container, row, scrollable, text};
use iced::{Element, Length};

use crate::backend::reports::{balance_sheet, profit_loss};
use crate::backend::services::report_service::ReportService;
use crate::database::exports::csv;
use crate::frontend::styles::colors::Theme;
use crate::frontend::styles::spacing;
use crate::frontend::styles::typography;
use crate::frontend::styles::button as button_style;
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
    let service = ReportService::new(ctx.db.clone());
    let today = Utc::now().date_naive();
    let period_start = year_start(today);

    let accounts = match service.list_accounts(ctx.company_id) {
        Ok(accounts) => accounts,
        Err(err) => return State { error: Some(err.to_string()), ..Default::default() },
    };
    let trial_balance = match service.trial_balance_as_of(ctx.company_id, today) {
        Ok(tb) => tb,
        Err(err) => return State { error: Some(err.to_string()), ..Default::default() },
    };
    let pl_balances = match service.revenue_and_expense_balances_as_of(ctx.company_id, today) {
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
        .filter_map(|b| account_lookup(b.account_id).map(|a| (a.account_code.clone(), a.account_name.clone())))
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

pub fn view(state: &State, theme: Theme) -> Element<'_, Message> {
    if let Some(error) = &state.error {
        return container(text(format!("Could not load reports: {error}")).color(theme.negative_front())).padding(spacing::SPACE_20).into();
    }
    let (Some(balance_sheet), Some(profit_and_loss)) = (&state.balance_sheet, &state.profit_and_loss) else {
        return container(text("Loading...")).padding(spacing::SPACE_20).into();
    };

    let mut bs_column = column![
        text("Balance Sheet").size(typography::subtitle()),
        text(format!("Total assets: {}", balance_sheet.total_assets())),
        text(format!("Total liabilities: {}", balance_sheet.total_liabilities())),
        text(format!("Total equity (incl. current year earnings): {}", balance_sheet.total_equity())),
    ]
    .spacing(spacing::SPACE_6);
    bs_column = bs_column.push(if balance_sheet.is_balanced() {
        text("Balanced").color(theme.positive_front())
    } else {
        text("Not balanced").color(theme.negative_front())
    });
    bs_column = bs_column.push(button(text("Export CSV")).style(button_style::secondary(theme)).on_press(Message::ExportBalanceSheetCsv).padding(spacing::SPACE_6));

    let pl_column = column![
        text("Profit & Loss (year to date)").size(typography::subtitle()),
        text(format!("Total revenue: {}", profit_and_loss.total_revenue())),
        text(format!("Total expenses: {}", profit_and_loss.total_expenses())),
        text(format!("Net income: {}", profit_and_loss.net_income())),
        button(text("Export CSV")).style(button_style::secondary(theme)).on_press(Message::ExportProfitAndLossCsv).padding(spacing::SPACE_6),
    ]
    .spacing(spacing::SPACE_6);

    let mut content = column![text("Reports").size(typography::title()), row![bs_column, pl_column].spacing(spacing::SPACE_40)].spacing(spacing::MD).padding(spacing::SPACE_20);

    if let Some(export_message) = &state.export_message {
        content = content.push(text(export_message.clone()).color(theme.positive_front()));
    }

    scrollable(container(content).width(Length::Fill)).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::accounting::accounting_rules::AccountType;
    use crate::backend::accounting::chart_of_accounts::Account;
    use crate::backend::accounting::fiscal_periods::FiscalPeriod;
    use crate::backend::organization::companies::Company;
    use crate::backend::repositories::accounting_repository::{self, AccountingRepository};
    use crate::backend::repositories::organization_repository::{self, OrganizationRepository};
    use crate::backend::transactions::transaction::Transaction;
    use crate::backend::transactions::transaction_lines::TransactionLine;
    use crate::database::database::Database;
    use rust_decimal_macros::dec;
    use std::path::PathBuf;

    fn ctx_with_a_profitable_year() -> Ctx {
        let db = Database::open_in_memory().unwrap();
        db.run_migrations(organization_repository::MIGRATIONS).unwrap();
        db.run_migrations(accounting_repository::MIGRATIONS).unwrap();
        db.run_migrations(crate::backend::audit::audit_logs::MIGRATIONS).unwrap();

        let org_repo = OrganizationRepository::new(db.clone());
        let company_id = org_repo.create_company(&Company::new("Acme Trading Co", "USD")).unwrap().id.unwrap();

        let accounting_repo = AccountingRepository::new(db.clone());
        let today = Utc::now().date_naive();
        accounting_repo
            .create_fiscal_period(&FiscalPeriod::new(company_id, "Open Period", year_start(today), today))
            .unwrap();
        let cash = accounting_repo.create_account(&Account::new(company_id, "1000", "Cash", AccountType::Asset)).unwrap();
        let capital = accounting_repo.create_account(&Account::new(company_id, "3000", "Owner's Capital", AccountType::Equity)).unwrap();
        let revenue = accounting_repo.create_account(&Account::new(company_id, "4000", "Sales Revenue", AccountType::Revenue)).unwrap();
        let expense = accounting_repo.create_account(&Account::new(company_id, "6000", "Rent Expense", AccountType::Expense)).unwrap();

        accounting_repo
            .post_transaction(
                &Transaction::new(company_id, crate::backend::transactions::transaction_types::TransactionType::OpeningBalance, year_start(today))
                    .with_line(TransactionLine::debit(cash.id.unwrap(), dec!(10000)))
                    .with_line(TransactionLine::credit(capital.id.unwrap(), dec!(10000))),
            )
            .unwrap();
        accounting_repo
            .post_transaction(
                &Transaction::new(company_id, crate::backend::transactions::transaction_types::TransactionType::ManualJournal, today)
                    .with_line(TransactionLine::debit(cash.id.unwrap(), dec!(5000)))
                    .with_line(TransactionLine::credit(revenue.id.unwrap(), dec!(5000))),
            )
            .unwrap();
        accounting_repo
            .post_transaction(
                &Transaction::new(company_id, crate::backend::transactions::transaction_types::TransactionType::ManualJournal, today)
                    .with_line(TransactionLine::debit(expense.id.unwrap(), dec!(1200)))
                    .with_line(TransactionLine::credit(cash.id.unwrap(), dec!(1200))),
            )
            .unwrap();

        Ctx { db, company_id, current_user_id: None, export_dir: PathBuf::from("."), uploads_dir: PathBuf::from(".") }
    }

    #[test]
    fn refresh_builds_a_balanced_balance_sheet_and_a_matching_profit_and_loss() {
        let ctx = ctx_with_a_profitable_year();
        let mut state = State::default();

        update(&mut state, Message::Refresh, &ctx);

        let balance_sheet = state.balance_sheet.expect("balance sheet should have loaded");
        let profit_and_loss = state.profit_and_loss.expect("profit and loss should have loaded");

        assert_eq!(profit_and_loss.net_income(), dec!(3800)); // 5000 revenue - 1200 expense
        assert!(balance_sheet.is_balanced());
        assert_eq!(balance_sheet.total_equity() - dec!(10000), profit_and_loss.net_income()); // current year earnings rolled in
    }

    #[test]
    fn export_with_nothing_loaded_is_a_no_op_for_both_reports() {
        let ctx = ctx_with_a_profitable_year();
        let mut state = State::default();

        update(&mut state, Message::ExportBalanceSheetCsv, &ctx);
        update(&mut state, Message::ExportProfitAndLossCsv, &ctx);

        assert!(state.export_message.is_none());
    }

    #[test]
    fn export_csv_writes_both_report_files() {
        let ctx = Ctx { export_dir: std::env::temp_dir().join(format!("memonbooks_reports_view_test_{:?}", std::thread::current().id())), ..ctx_with_a_profitable_year() };
        let mut state = load(&ctx);

        update(&mut state, Message::ExportBalanceSheetCsv, &ctx);
        assert!(state.export_message.as_deref().unwrap_or_default().contains("balance_sheet.csv"));

        update(&mut state, Message::ExportProfitAndLossCsv, &ctx);
        assert!(state.export_message.as_deref().unwrap_or_default().contains("profit_and_loss.csv"));

        assert!(ctx.export_path("balance_sheet.csv").exists());
        assert!(ctx.export_path("profit_and_loss.csv").exists());

        let _ = std::fs::remove_dir_all(&ctx.export_dir);
    }
}
