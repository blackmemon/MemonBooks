//! Integration test for banking (Development Order step 12): a cash
//! transfer between two GL accounts, an unmatched bank fee reconciled
//! via post_and_match_adjustment, and the resulting book balance.

use chrono::NaiveDate;
use rust_decimal_macros::dec;

use memonbooks::backend::accounting::accounting_rules::AccountType;
use memonbooks::backend::accounting::chart_of_accounts::Account;
use memonbooks::backend::accounting::fiscal_periods::FiscalPeriod;
use memonbooks::backend::banking::bank_accounts::BankAccount;
use memonbooks::backend::banking::bank_transactions::BankTransaction;
use memonbooks::backend::banking::cash_management::CashTransfer;
use memonbooks::backend::organization::companies::Company;
use memonbooks::backend::repositories::accounting_repository::{self, AccountingRepository};
use memonbooks::backend::repositories::banking_repository::{self, BankingRepository};
use memonbooks::backend::repositories::organization_repository::{self, OrganizationRepository};
use memonbooks::database::database::Database;

fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
}

#[test]
fn cash_transfer_and_bank_fee_reconciliation_keep_the_ledger_balanced() {
    let db = Database::open_in_memory().unwrap();
    db.run_migrations(organization_repository::MIGRATIONS)
        .unwrap();
    db.run_migrations(accounting_repository::MIGRATIONS)
        .unwrap();
    db.run_migrations(memonbooks::backend::audit::audit_logs::MIGRATIONS)
        .unwrap();
    db.run_migrations(banking_repository::MIGRATIONS).unwrap();

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
            "January 2026",
            date(2026, 1, 1),
            date(2026, 1, 31),
        ))
        .unwrap();
    let bank_gl = accounting_repo
        .create_account(&Account::new(
            company_id,
            "1010",
            "Bank",
            AccountType::Asset,
        ))
        .unwrap();
    let petty_cash = accounting_repo
        .create_account(&Account::new(
            company_id,
            "1020",
            "Petty Cash",
            AccountType::Asset,
        ))
        .unwrap();
    let bank_charges = accounting_repo
        .create_account(&Account::new(
            company_id,
            "6300",
            "Bank Charges",
            AccountType::Expense,
        ))
        .unwrap();

    let banking_repo = BankingRepository::new(db);
    let bank_account = banking_repo
        .create_bank_account(&BankAccount::new(
            company_id,
            bank_gl.id.unwrap(),
            "First National",
            "123456789",
            "USD",
        ))
        .unwrap();

    // Fund the bank account first so it has a positive balance to work with.
    accounting_repo
        .post_transaction(
            &memonbooks::backend::transactions::transaction::Transaction::new(company_id, memonbooks::backend::transactions::transaction_types::TransactionType::OpeningBalance, date(2026, 1, 1))
                .with_line(memonbooks::backend::transactions::transaction_lines::TransactionLine::debit(bank_gl.id.unwrap(), dec!(5000)))
                .with_line(memonbooks::backend::transactions::transaction_lines::TransactionLine::credit(petty_cash.id.unwrap(), dec!(5000))),
        )
        .unwrap();

    let transfer = CashTransfer::new(
        company_id,
        bank_gl.id.unwrap(),
        petty_cash.id.unwrap(),
        date(2026, 1, 10),
        dec!(200),
    );
    banking_repo.post_cash_transfer(&transfer).unwrap();

    let fee = banking_repo
        .create_bank_transaction(&BankTransaction::new(
            company_id,
            bank_account.id.unwrap(),
            date(2026, 1, 15),
            "Monthly fee",
            dec!(-25),
        ))
        .unwrap();
    banking_repo
        .post_and_match_adjustment(fee.id.unwrap(), bank_charges.id.unwrap())
        .unwrap();

    assert!(banking_repo
        .list_unreconciled_for_account(bank_account.id.unwrap())
        .unwrap()
        .is_empty());

    // 5000 opening - 200 transfer out - 25 fee = 4775.
    let book_balance = banking_repo
        .book_balance(bank_account.id.unwrap(), date(2026, 1, 31))
        .unwrap();
    assert_eq!(book_balance, dec!(4775));

    let trial_balance = accounting_repo
        .trial_balance_as_of(company_id, date(2026, 1, 31))
        .unwrap();
    assert!(trial_balance.is_balanced());
}
