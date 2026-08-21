//! Integration test for expenses (Development Order step 13): a direct
//! expense, and an employee expense claim through its full
//! submit -> approve -> reimburse lifecycle.

use chrono::NaiveDate;
use rust_decimal_macros::dec;

use memonbooks::backend::accounting::accounting_rules::AccountType;
use memonbooks::backend::accounting::chart_of_accounts::Account;
use memonbooks::backend::accounting::fiscal_periods::FiscalPeriod;
use memonbooks::backend::expenses::expense_categories::ExpenseCategory;
use memonbooks::backend::expenses::expense_claims::{ExpenseClaim, ExpenseClaimStatus};
use memonbooks::backend::expenses::expenses::Expense;
use memonbooks::backend::organization::companies::Company;
use memonbooks::backend::repositories::accounting_repository::{self, AccountingRepository};
use memonbooks::backend::repositories::expense_repository::{self, ExpenseRepository};
use memonbooks::backend::repositories::organization_repository::{self, OrganizationRepository};
use memonbooks::database::database::Database;

fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
}

#[test]
fn direct_expense_and_reimbursed_claim_both_post_and_keep_the_ledger_balanced() {
    let db = Database::open_in_memory().unwrap();
    db.run_migrations(organization_repository::MIGRATIONS).unwrap();
    db.run_migrations(accounting_repository::MIGRATIONS).unwrap();
    db.run_migrations(memonbooks::backend::audit::audit_logs::MIGRATIONS).unwrap();
    db.run_migrations(expense_repository::MIGRATIONS).unwrap();

    let org_repo = OrganizationRepository::new(db.clone());
    let company_id = org_repo.create_company(&Company::new("Acme Trading Co", "USD")).unwrap().id.unwrap();

    let accounting_repo = AccountingRepository::new(db.clone());
    accounting_repo
        .create_fiscal_period(&FiscalPeriod::new(company_id, "January 2026", date(2026, 1, 1), date(2026, 1, 31)))
        .unwrap();
    let travel_expense = accounting_repo.create_account(&Account::new(company_id, "6100", "Travel Expense", AccountType::Expense)).unwrap();
    let cash = accounting_repo.create_account(&Account::new(company_id, "1000", "Cash", AccountType::Asset)).unwrap();

    let expense_repo = ExpenseRepository::new(db);
    let category = expense_repo.create_category(&ExpenseCategory::new(company_id, "Travel", travel_expense.id.unwrap())).unwrap();

    let (_recorded, direct_transaction) = expense_repo
        .record_expense(&Expense::new(company_id, category.id.unwrap(), date(2026, 1, 5), dec!(45), "Taxi fare", cash.id.unwrap()))
        .unwrap();
    assert!(direct_transaction.is_balanced());

    let claim = expense_repo
        .create_claim(&ExpenseClaim::new(company_id, "Jane Doe", category.id.unwrap(), date(2026, 1, 10), dec!(120), "Client dinner"))
        .unwrap();
    expense_repo.approve_claim(claim.id.unwrap()).unwrap();
    let (reimbursed, claim_transaction) = expense_repo.reimburse_claim(claim.id.unwrap(), cash.id.unwrap(), date(2026, 1, 12)).unwrap();
    assert_eq!(reimbursed.status, ExpenseClaimStatus::Reimbursed);
    assert!(claim_transaction.is_balanced());

    let trial_balance = accounting_repo.trial_balance_as_of(company_id, date(2026, 1, 31)).unwrap();
    assert!(trial_balance.is_balanced());
    let travel_line = trial_balance.lines.iter().find(|l| l.account_id == travel_expense.id.unwrap()).unwrap();
    assert_eq!(travel_line.debit, dec!(165)); // 45 + 120
}
