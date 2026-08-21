//! Integration test for the accounting core (Development Order steps
//! 5-8): chart of accounts, fiscal periods, the transaction engine, and
//! the trial balance -- exercised together through the crate's public
//! API, as an external consumer would, rather than each repository's
//! own inline unit tests in isolation.

use chrono::NaiveDate;
use rust_decimal_macros::dec;

use memonbooks::backend::accounting::accounting_rules::AccountType;
use memonbooks::backend::accounting::chart_of_accounts::Account;
use memonbooks::backend::accounting::fiscal_periods::FiscalPeriod;
use memonbooks::backend::organization::companies::Company;
use memonbooks::backend::repositories::accounting_repository::{self, AccountingRepository};
use memonbooks::backend::repositories::organization_repository::{self, OrganizationRepository};
use memonbooks::backend::transactions::transaction::Transaction;
use memonbooks::backend::transactions::transaction_lines::TransactionLine;
use memonbooks::backend::transactions::transaction_types::TransactionType;
use memonbooks::database::database::Database;

fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
}

#[test]
fn posting_balanced_entries_keeps_the_trial_balance_balanced_end_to_end() {
    let db = Database::open_in_memory().unwrap();
    db.run_migrations(organization_repository::MIGRATIONS).unwrap();
    db.run_migrations(accounting_repository::MIGRATIONS).unwrap();
    db.run_migrations(memonbooks::backend::audit::audit_logs::MIGRATIONS).unwrap();

    let org_repo = OrganizationRepository::new(db.clone());
    let company = org_repo.create_company(&Company::new("Acme Trading Co", "USD")).unwrap();
    let company_id = company.id.unwrap();

    let accounting_repo = AccountingRepository::new(db);
    accounting_repo
        .create_fiscal_period(&FiscalPeriod::new(company_id, "January 2026", date(2026, 1, 1), date(2026, 1, 31)))
        .unwrap();

    let cash = accounting_repo.create_account(&Account::new(company_id, "1000", "Cash", AccountType::Asset)).unwrap();
    let capital = accounting_repo.create_account(&Account::new(company_id, "3000", "Owner's Capital", AccountType::Equity)).unwrap();
    let revenue = accounting_repo.create_account(&Account::new(company_id, "4000", "Sales Revenue", AccountType::Revenue)).unwrap();

    // Opening capital.
    accounting_repo
        .post_transaction(
            &Transaction::new(company_id, TransactionType::OpeningBalance, date(2026, 1, 1))
                .with_line(TransactionLine::debit(cash.id.unwrap(), dec!(10000)))
                .with_line(TransactionLine::credit(capital.id.unwrap(), dec!(10000))),
        )
        .unwrap();

    // A sale.
    let sale = accounting_repo
        .post_transaction(
            &Transaction::new(company_id, TransactionType::ManualJournal, date(2026, 1, 15))
                .with_line(TransactionLine::debit(cash.id.unwrap(), dec!(2500)))
                .with_line(TransactionLine::credit(revenue.id.unwrap(), dec!(2500))),
        )
        .unwrap();

    let trial_balance = accounting_repo.trial_balance_as_of(company_id, date(2026, 1, 31)).unwrap();
    assert!(trial_balance.is_balanced(), "trial balance must balance after any number of balanced postings");
    assert_eq!(trial_balance.total_debits(), dec!(12500));

    // Reversing the sale should bring cash and revenue back to their
    // opening-only state, and the trial balance must still balance.
    accounting_repo.reverse_transaction(sale.id.unwrap(), date(2026, 1, 20)).unwrap();
    let trial_balance = accounting_repo.trial_balance_as_of(company_id, date(2026, 1, 31)).unwrap();
    assert!(trial_balance.is_balanced());
    assert_eq!(trial_balance.total_debits(), dec!(10000));
}

#[test]
fn an_unbalanced_transaction_is_rejected_before_touching_the_ledger() {
    let db = Database::open_in_memory().unwrap();
    db.run_migrations(organization_repository::MIGRATIONS).unwrap();
    db.run_migrations(accounting_repository::MIGRATIONS).unwrap();
    db.run_migrations(memonbooks::backend::audit::audit_logs::MIGRATIONS).unwrap();

    let org_repo = OrganizationRepository::new(db.clone());
    let company = org_repo.create_company(&Company::new("Acme Trading Co", "USD")).unwrap();
    let company_id = company.id.unwrap();

    let accounting_repo = AccountingRepository::new(db);
    accounting_repo
        .create_fiscal_period(&FiscalPeriod::new(company_id, "January 2026", date(2026, 1, 1), date(2026, 1, 31)))
        .unwrap();
    let cash = accounting_repo.create_account(&Account::new(company_id, "1000", "Cash", AccountType::Asset)).unwrap();
    let capital = accounting_repo.create_account(&Account::new(company_id, "3000", "Owner's Capital", AccountType::Equity)).unwrap();

    let result = accounting_repo.post_transaction(
        &Transaction::new(company_id, TransactionType::ManualJournal, date(2026, 1, 15))
            .with_line(TransactionLine::debit(cash.id.unwrap(), dec!(100)))
            .with_line(TransactionLine::credit(capital.id.unwrap(), dec!(99))),
    );
    assert!(result.is_err());
    assert!(accounting_repo.list_transactions_for_company(company_id).unwrap().is_empty());
}
