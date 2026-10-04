//! Display the trial balance (Development Order step 7) as of today, and
//! export it to CSV.

use chrono::Utc;

use iced::widget::{column, container, scrollable, text};
use iced::{Element, Length};

use crate::backend::accounting::trial_balance::TrialBalance;
use crate::backend::reports::trial_balance as trial_balance_report;
use crate::backend::services::report_service::ReportService;
use crate::database::exports::csv;
use crate::frontend::components::common::data_table::{self, Cell, ColumnDef, Row, SortDirection};
use crate::frontend::components::common::{page_header, section_heading, toolbar};
use crate::frontend::styles::colors::Theme;
use crate::frontend::styles::spacing;
use crate::frontend::views::Ctx;

#[derive(Debug, Clone, Default)]
pub struct State {
    report: Option<TrialBalance>,
    search: String,
    export_message: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    Refresh,
    ExportCsv,
    SearchChanged(String),
    TabSelected(usize),
    FiltersClicked,
    SortClicked,
    RowAction(usize),
}

pub fn load(ctx: &Ctx) -> State {
    let service = ReportService::new(ctx.db.clone());
    let today = Utc::now().date_naive();
    match service.trial_balance_as_of(ctx.company_id, today) {
        Ok(report) => State {
            report: Some(report),
            ..Default::default()
        },
        Err(err) => State {
            error: Some(err.to_string()),
            ..Default::default()
        },
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
        Message::SearchChanged(value) => state.search = value,
        // This view only has one tab, "Filters" / "Sort by" are rendered
        // by the shared toolbar component now (Section 6), and the kebab
        // menu has no per-row action -- this is a read-only report, not
        // an editable list. Wiring real tab/filter/sort behavior is
        // follow-up work, not part of this layout pass.
        Message::TabSelected(_)
        | Message::FiltersClicked
        | Message::SortClicked
        | Message::RowAction(_) => {}
    }
}

pub fn view(state: &State, theme: Theme) -> Element<'_, Message> {
    let Some(report) = &state.report else {
        let message = state
            .error
            .clone()
            .unwrap_or_else(|| "Loading...".to_string());
        return container(text(message).color(theme.negative_front()))
            .padding(spacing::SPACE_20)
            .into();
    };

    let header = page_header::view(
        theme,
        "Trial Balance",
        "Every account's debit and credit balance as of today.",
        "Export CSV",
        Message::ExportCsv,
    );

    let toolbar = toolbar::view(
        theme,
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

    let balanced_note = if report.is_balanced() {
        "Balanced"
    } else {
        "Not balanced"
    };
    let heading = section_heading::view(theme, "Account balances", balanced_note);

    let query = state.search.trim().to_lowercase();
    let filtered: Vec<_> = report
        .lines
        .iter()
        .filter(|line| {
            query.is_empty()
                || line.account_name.to_lowercase().contains(&query)
                || line.account_code.to_lowercase().contains(&query)
        })
        .collect();

    let columns = vec![
        ColumnDef::new("Code", Length::FillPortion(1)).sorted(SortDirection::Ascending),
        ColumnDef::new("Account", Length::FillPortion(3)),
        ColumnDef::new("Debit", Length::FillPortion(1)),
        ColumnDef::new("Credit", Length::FillPortion(1)),
    ];

    let mut rows: Vec<Row> = filtered
        .iter()
        .map(|line| {
            Row::new(vec![
                Cell::text(line.account_code.clone()),
                Cell::text(line.account_name.clone()),
                Cell::text(line.debit.to_string()),
                Cell::text(line.credit.to_string()),
            ])
        })
        .collect();
    rows.push(Row::new(vec![
        Cell::text(""),
        Cell::text("Total"),
        Cell::text(report.total_debits().to_string()),
        Cell::text(report.total_credits().to_string()),
    ]));

    let table = data_table::view(
        theme,
        columns,
        rows,
        None,
        |_index, _checked| Message::RowAction(0),
        |_checked| Message::RowAction(0),
        Message::RowAction,
        Message::RowAction,
    );

    let mut content = column![header, toolbar, heading, table]
        .spacing(spacing::SPACE_20)
        .padding(spacing::LG)
        .width(Length::Fill);

    if let Some(export_message) = &state.export_message {
        content = content.push(text(export_message.clone()).color(theme.positive_front()));
    }

    container(scrollable(content).height(Length::Fill))
        .width(Length::Fill)
        .into()
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
        db.run_migrations(organization_repository::MIGRATIONS)
            .unwrap();
        db.run_migrations(accounting_repository::MIGRATIONS)
            .unwrap();
        db.run_migrations(crate::backend::audit::audit_logs::MIGRATIONS)
            .unwrap();

        let org_repo = OrganizationRepository::new(db.clone());
        let company_id = org_repo
            .create_company(&Company::new("Acme Trading Co", "USD"))
            .unwrap()
            .id
            .unwrap();

        let accounting_repo = AccountingRepository::new(db.clone());
        accounting_repo
            .create_fiscal_period(&FiscalPeriod::new(
                company_id,
                "Open Period",
                date(2026, 1, 1),
                date(2030, 12, 31),
            ))
            .unwrap();
        let cash = accounting_repo
            .create_account(&Account::new(
                company_id,
                "1000",
                "Cash",
                AccountType::Asset,
            ))
            .unwrap();
        let capital = accounting_repo
            .create_account(&Account::new(
                company_id,
                "3000",
                "Owner's Capital",
                AccountType::Equity,
            ))
            .unwrap();
        accounting_repo
            .post_transaction(
                &Transaction::new(
                    company_id,
                    TransactionType::OpeningBalance,
                    date(2026, 1, 1),
                )
                .with_line(TransactionLine::debit(cash.id.unwrap(), dec!(1000)))
                .with_line(TransactionLine::credit(capital.id.unwrap(), dec!(1000))),
            )
            .unwrap();

        let export_dir = std::env::temp_dir().join(format!(
            "memonbooks_trial_balance_view_test_{:?}",
            std::thread::current().id()
        ));
        Ctx {
            db,
            company_id,
            current_user_id: None,
            export_dir: export_dir.clone(),
            uploads_dir: export_dir,
        }
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

    #[test]
    fn search_changed_updates_the_search_field() {
        let ctx = ctx_with_a_posted_transaction();
        let mut state = State::default();

        update(&mut state, Message::SearchChanged("cash".into()), &ctx);

        assert_eq!(state.search, "cash");
    }
}
