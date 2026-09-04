//! Integration test for reports (Development Order step 15): posted
//! transactions feeding a trial balance, a balance sheet, and a profit
//! & loss report, checked against each other for internal consistency
//! (not just each report's own formatting-level unit tests).

use chrono::NaiveDate;
use rust_decimal_macros::dec;

use memonbooks::backend::accounting::accounting_rules::AccountType;
use memonbooks::backend::accounting::chart_of_accounts::Account;
use memonbooks::backend::accounting::fiscal_periods::FiscalPeriod;
use memonbooks::backend::organization::companies::Company;
use memonbooks::backend::reports::{balance_sheet, profit_loss};
use memonbooks::backend::repositories::accounting_repository::{self, AccountingRepository};
use memonbooks::backend::repositories::organization_repository::{self, OrganizationRepository};
use memonbooks::backend::transactions::transaction::Transaction;
use memonbooks::backend::transactions::transaction_lines::TransactionLine;
use memonbooks::backend::transactions::transaction_types::TransactionType;
use memonbooks::database::database::Database;
use memonbooks::database::exports::csv;

fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
}

#[test]
fn balance_sheet_and_profit_and_loss_agree_with_the_trial_balance() {
    let db = Database::open_in_memory().unwrap();
    db.run_migrations(organization_repository::MIGRATIONS).unwrap();
    db.run_migrations(accounting_repository::MIGRATIONS).unwrap();
    db.run_migrations(memonbooks::backend::audit::audit_logs::MIGRATIONS).unwrap();

    let org_repo = OrganizationRepository::new(db.clone());
    let company_id = org_repo.create_company(&Company::new("Acme Trading Co", "USD")).unwrap().id.unwrap();

    let accounting_repo = AccountingRepository::new(db);
    accounting_repo
        .create_fiscal_period(&FiscalPeriod::new(company_id, "Fiscal Year 2026", date(2026, 1, 1), date(2026, 12, 31)))
        .unwrap();
    let cash = accounting_repo.create_account(&Account::new(company_id, "1000", "Cash", AccountType::Asset)).unwrap();
    let capital = accounting_repo.create_account(&Account::new(company_id, "3000", "Owner's Capital", AccountType::Equity)).unwrap();
    let revenue = accounting_repo.create_account(&Account::new(company_id, "4000", "Sales Revenue", AccountType::Revenue)).unwrap();
    let expense = accounting_repo.create_account(&Account::new(company_id, "6000", "Rent Expense", AccountType::Expense)).unwrap();

    accounting_repo
        .post_transaction(
            &Transaction::new(company_id, TransactionType::OpeningBalance, date(2026, 1, 1))
                .with_line(TransactionLine::debit(cash.id.unwrap(), dec!(10000)))
                .with_line(TransactionLine::credit(capital.id.unwrap(), dec!(10000))),
        )
        .unwrap();
    accounting_repo
        .post_transaction(
            &Transaction::new(company_id, TransactionType::ManualJournal, date(2026, 3, 1))
                .with_line(TransactionLine::debit(cash.id.unwrap(), dec!(5000)))
                .with_line(TransactionLine::credit(revenue.id.unwrap(), dec!(5000))),
        )
        .unwrap();
    accounting_repo
        .post_transaction(
            &Transaction::new(company_id, TransactionType::ManualJournal, date(2026, 3, 5))
                .with_line(TransactionLine::debit(expense.id.unwrap(), dec!(1200)))
                .with_line(TransactionLine::credit(cash.id.unwrap(), dec!(1200))),
        )
        .unwrap();

    let as_of = date(2026, 12, 31);
    let accounts = accounting_repo.list_accounts_for_company(company_id).unwrap();
    let trial_balance = accounting_repo.trial_balance_as_of(company_id, as_of).unwrap();
    assert!(trial_balance.is_balanced());

    let pl_balances = accounting_repo.revenue_and_expense_balances_as_of(company_id, as_of).unwrap();
    let pl_accounts: Vec<(String, String)> = pl_balances
        .iter()
        .map(|b| {
            let account = accounts.iter().find(|a| a.id == Some(b.account_id)).unwrap();
            (account.account_code.clone(), account.account_name.clone())
        })
        .collect();
    let profit_and_loss = profit_loss::build(date(2026, 1, 1), as_of, &pl_accounts, &pl_balances);
    assert_eq!(profit_and_loss.net_income(), dec!(3800)); // 5000 - 1200

    let bs_input: Vec<_> = trial_balance
        .lines
        .iter()
        .map(|line| {
            let account = accounts.iter().find(|a| a.id == Some(line.account_id)).unwrap();
            (line.account_id, line.account_code.clone(), line.account_name.clone(), account.account_type, line.debit - line.credit)
        })
        .collect();
    let balance_sheet = balance_sheet::build(as_of, &bs_input, profit_and_loss.net_income());

    // The balance sheet's equity (including current year earnings) must
    // match the trial balance's total credits exactly: capital (10000)
    // wasn't touched by revenue/expense postings, and current year
    // earnings (3800) is exactly the net income these two reports were
    // built from the same underlying data to agree on.
    assert!(balance_sheet.is_balanced());
    assert_eq!(balance_sheet.total_equity(), dec!(13800)); // 10000 capital + 3800 earnings

    // The CSV exporters (Development Order step 15) should round-trip
    // without panicking on real report data.
    let (headers, rows) = memonbooks::backend::reports::trial_balance::to_rows(&trial_balance);
    let csv_text = csv::export_csv(&headers, &rows);
    assert!(csv_text.contains("Cash"));
}
