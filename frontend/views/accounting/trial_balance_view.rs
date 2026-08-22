//! Display the trial balance (Development Order step 7) as of today, and
//! export it to CSV.

use chrono::Utc;

use iced::widget::{button, column, container, row, scrollable, text};
use iced::{Element, Length};

use crate::backend::accounting::trial_balance::TrialBalance;
use crate::backend::reports::trial_balance as trial_balance_report;
use crate::backend::services::report_service::ReportService;
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
    let service = ReportService::new(ctx.db.clone());
    let today = Utc::now().date_naive();
    match service.trial_balance_as_of(ctx.company_id, today) {
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
    use crate::backend::transactions::transaction_types::TransactionType;
    use crate::database::database::Database;
    use chrono::NaiveDate;
    use rust_decimal_macros::dec;

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    fn ctx_with_a_posted_transaction() -> Ctx {
        let db = Database::open_in_memory().unwrap();
        db.run_migrations(organization_repository::MIGRATIONS).unwrap();
        db.run_migrations(accounting_repository::MIGRATIONS).unwrap();
        db.run_migrations(crate::backend::audit::audit_logs::MIGRATIONS).unwrap();

        let org_repo = OrganizationRepository::new(db.clone());
        let company_id = org_repo.create_company(&Company::new("Acme Trading Co", "USD")).unwrap().id.unwrap();

        let accounting_repo = AccountingRepository::new(db.clone());
        accounting_repo.create_fiscal_period(&FiscalPeriod::new(company_id, "Open Period", date(2026, 1, 1), date(2030, 12, 31))).unwrap();
        let cash = accounting_repo.create_account(&Account::new(company_id, "1000", "Cash", AccountType::Asset)).unwrap();
        let capital = accounting_repo.create_account(&Account::new(company_id, "3000", "Owner's Capital", AccountType::Equity)).unwrap();
        accounting_repo
            .post_transaction(
                &Transaction::new(company_id, TransactionType::OpeningBalance, date(2026, 1, 1))
                    .with_line(TransactionLine::debit(cash.id.unwrap(), dec!(1000)))
                    .with_line(TransactionLine::credit(capital.id.unwrap(), dec!(1000))),
            )
            .unwrap();

        let export_dir = std::env::temp_dir().join(format!("memonbooks_trial_balance_view_test_{:?}", std::thread::current().id()));
        Ctx { db, company_id, current_user_id: None, export_dir }
    }

    #[test]
    fn refresh_loads_a_balanced_report_reflecting_posted_transactions() {
        let ctx = ctx_with_a_posted_transaction();
        let mut state = State::default();

        update(&mut state, Message::Refresh, &ctx);

        let report = state.report.expect("report should have loaded");
        assert!(report.is_balanced());
        assert_eq!(report.total_debits(), dec!(1000));
    }

    #[test]
    fn export_csv_with_no_report_loaded_is_a_no_op() {
        let ctx = ctx_with_a_posted_transaction();
        let mut state = State::default();

        update(&mut state, Message::ExportCsv, &ctx);

        assert!(state.export_message.is_none());
    }

    #[test]
    fn export_csv_writes_a_file_and_sets_the_export_message() {
        let ctx = ctx_with_a_posted_transaction();
        let mut state = load(&ctx);

        update(&mut state, Message::ExportCsv, &ctx);

        assert!(state.export_message.is_some());
        assert!(state.error.is_none());
        assert!(ctx.export_path("trial_balance.csv").exists());

        let _ = std::fs::remove_dir_all(&ctx.export_dir);
    }
}
