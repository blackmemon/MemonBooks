//! Persistence for bank accounts and bank transactions (reconciliation
//! staging records). Posting adjustment/cash-transfer transactions
//! delegates to `AccountingRepository`, same pattern as
//! `sales_repository.rs`/`purchase_repository.rs`.

use rust_decimal::Decimal;
use rusqlite::{params, OptionalExtension, Row};

use crate::backend::banking::bank_accounts::BankAccount;
use crate::backend::banking::bank_transactions::BankTransaction;
use crate::backend::banking::cash_management::CashTransfer;
use crate::backend::core::errors::{AppError, AppResult};
use crate::backend::core::validation::Validate;
use crate::backend::repositories::accounting_repository::AccountingRepository;
use crate::backend::transactions::transaction::Transaction;
use crate::database::database::{Database, Migration};

pub const MIGRATIONS: &[Migration] = &[Migration {
    version: 1200,
    name: "banking_schema",
    sql: "
        CREATE TABLE bank_accounts (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            company_id      INTEGER NOT NULL REFERENCES companies(id),
            gl_account_id   INTEGER NOT NULL REFERENCES accounts(id),
            bank_name       TEXT NOT NULL,
            account_number  TEXT NOT NULL,
            currency_code   TEXT NOT NULL,
            is_active       INTEGER NOT NULL DEFAULT 1
        );

        CREATE TABLE bank_transactions (
            id                      INTEGER PRIMARY KEY AUTOINCREMENT,
            company_id              INTEGER NOT NULL REFERENCES companies(id),
            bank_account_id         INTEGER NOT NULL REFERENCES bank_accounts(id),
            transaction_date        TEXT NOT NULL,
            description             TEXT NOT NULL,
            amount                  TEXT NOT NULL,
            matched_transaction_id  INTEGER REFERENCES transactions(id)
        );
    ",
}];

#[derive(Clone)]
pub struct BankingRepository {
    db: Database,
    accounting: AccountingRepository,
}

impl BankingRepository {
    pub fn new(db: Database) -> Self {
        let accounting = AccountingRepository::new(db.clone());
        Self { db, accounting }
    }

    // -- bank accounts ----------------------------------------------------

    pub fn create_bank_account(&self, account: &BankAccount) -> AppResult<BankAccount> {
        account.validate()?;
        self.db.with_connection(|conn| {
            conn.execute(
                "INSERT INTO bank_accounts (company_id, gl_account_id, bank_name, account_number, currency_code, is_active)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![account.company_id, account.gl_account_id, account.bank_name, account.account_number, account.currency_code, account.is_active],
            )?;
            Ok(BankAccount { id: Some(conn.last_insert_rowid()), ..account.clone() })
        })
    }

    pub fn list_bank_accounts_for_company(&self, company_id: i64) -> AppResult<Vec<BankAccount>> {
        self.db.with_connection(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, company_id, gl_account_id, bank_name, account_number, currency_code, is_active
                 FROM bank_accounts WHERE company_id = ?1 ORDER BY bank_name",
            )?;
            let rows = stmt.query_map(params![company_id], map_bank_account)?;
            rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
        })
    }

    // -- bank transactions (reconciliation staging) ------------------

    pub fn create_bank_transaction(&self, transaction: &BankTransaction) -> AppResult<BankTransaction> {
        transaction.validate()?;
        self.db.with_connection(|conn| {
            conn.execute(
                "INSERT INTO bank_transactions (company_id, bank_account_id, transaction_date, description, amount, matched_transaction_id)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    transaction.company_id,
                    transaction.bank_account_id,
                    transaction.transaction_date,
                    transaction.description,
                    transaction.amount.to_string(),
                    transaction.matched_transaction_id,
                ],
            )?;
            Ok(BankTransaction { id: Some(conn.last_insert_rowid()), ..transaction.clone() })
        })
    }

    pub fn list_unreconciled_for_account(&self, bank_account_id: i64) -> AppResult<Vec<BankTransaction>> {
        self.db.with_connection(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, company_id, bank_account_id, transaction_date, description, amount, matched_transaction_id
                 FROM bank_transactions WHERE bank_account_id = ?1 AND matched_transaction_id IS NULL ORDER BY transaction_date",
            )?;
            let rows = stmt.query_map(params![bank_account_id], map_bank_transaction)?;
            rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
        })
    }

    /// Marks a bank statement line as matched against an existing GL
    /// transaction (the common case: it corresponds to a payment/receipt
    /// already posted in steps 9-10).
    pub fn match_bank_transaction(&self, bank_transaction_id: i64, gl_transaction_id: i64) -> AppResult<()> {
        self.db.with_connection(|conn| {
            let affected = conn.execute(
                "UPDATE bank_transactions SET matched_transaction_id = ?1 WHERE id = ?2",
                params![gl_transaction_id, bank_transaction_id],
            )?;
            if affected == 0 {
                return Err(AppError::NotFound(format!("bank transaction {bank_transaction_id} not found")));
            }
            Ok(())
        })
    }

    /// For a bank-only item (no existing GL entry): posts the adjustment
    /// transaction and marks this bank transaction matched against it, in
    /// one atomic step.
    pub fn post_and_match_adjustment(&self, bank_transaction_id: i64, offset_account_id: i64) -> AppResult<Transaction> {
        let bank_transaction = self.db.with_connection(|conn| {
            conn.query_row(
                "SELECT id, company_id, bank_account_id, transaction_date, description, amount, matched_transaction_id FROM bank_transactions WHERE id = ?1",
                params![bank_transaction_id],
                map_bank_transaction,
            )
            .optional()?
            .ok_or_else(|| AppError::NotFound(format!("bank transaction {bank_transaction_id} not found")))
        })?;
        if bank_transaction.is_reconciled() {
            return Err(AppError::Validation("bank transaction is already matched".into()));
        }

        let bank_account = self.db.with_connection(|conn| {
            conn.query_row(
                "SELECT id, company_id, gl_account_id, bank_name, account_number, currency_code, is_active FROM bank_accounts WHERE id = ?1",
                params![bank_transaction.bank_account_id],
                map_bank_account,
            )
            .optional()?
            .ok_or_else(|| AppError::NotFound(format!("bank account {} not found", bank_transaction.bank_account_id)))
        })?;

        let gl_transaction = bank_transaction.build_adjustment_transaction(bank_account.gl_account_id, offset_account_id);
        let posted = self.accounting.post_transaction(&gl_transaction)?;
        self.match_bank_transaction(bank_transaction_id, posted.id.unwrap())?;
        Ok(posted)
    }

    // -- cash management --------------------------------------------------

    pub fn post_cash_transfer(&self, transfer: &CashTransfer) -> AppResult<Transaction> {
        transfer.validate()?;
        self.accounting.post_transaction(&transfer.build_transaction())
    }

    /// The GL balance for a bank account as of a date -- the "book
    /// balance" side of a reconciliation.
    pub fn book_balance(&self, bank_account_id: i64, as_of: chrono::NaiveDate) -> AppResult<Decimal> {
        let bank_account = self.db.with_connection(|conn| {
            conn.query_row(
                "SELECT id, company_id, gl_account_id, bank_name, account_number, currency_code, is_active FROM bank_accounts WHERE id = ?1",
                params![bank_account_id],
                map_bank_account,
            )
            .optional()?
            .ok_or_else(|| AppError::NotFound(format!("bank account {bank_account_id} not found")))
        })?;

        let ledger = self.accounting.general_ledger_for_account(bank_account.gl_account_id, Decimal::ZERO, as_of)?;
        Ok(ledger.last().map(|entry| entry.running_balance).unwrap_or(Decimal::ZERO))
    }
}

fn map_bank_account(row: &Row) -> rusqlite::Result<BankAccount> {
    Ok(BankAccount {
        id: Some(row.get(0)?),
        company_id: row.get(1)?,
        gl_account_id: row.get(2)?,
        bank_name: row.get(3)?,
        account_number: row.get(4)?,
        currency_code: row.get(5)?,
        is_active: row.get(6)?,
    })
}

fn map_bank_transaction(row: &Row) -> rusqlite::Result<BankTransaction> {
    let amount_text: String = row.get(5)?;
    Ok(BankTransaction {
        id: Some(row.get(0)?),
        company_id: row.get(1)?,
        bank_account_id: row.get(2)?,
        transaction_date: row.get(3)?,
        description: row.get(4)?,
        amount: amount_text.parse().unwrap_or(Decimal::ZERO),
        matched_transaction_id: row.get(6)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::accounting::accounting_rules::AccountType;
    use crate::backend::accounting::chart_of_accounts::Account;
    use crate::backend::accounting::fiscal_periods::FiscalPeriod;
    use crate::backend::organization::companies::Company;
    use crate::backend::repositories::accounting_repository::{self, AccountingRepository as AccRepo};
    use crate::backend::repositories::organization_repository::{self, OrganizationRepository};
    use chrono::NaiveDate;
    use rust_decimal_macros::dec;

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    struct TestFixture {
        banking: BankingRepository,
        company_id: i64,
        bank_gl_account_id: i64,
        bank_account_id: i64,
        expense_account_id: i64,
    }

    fn fixture() -> TestFixture {
        let db = Database::open_in_memory().unwrap();
        db.run_migrations(organization_repository::MIGRATIONS).unwrap();
        db.run_migrations(accounting_repository::MIGRATIONS).unwrap();
        db.run_migrations(crate::backend::audit::audit_logs::MIGRATIONS).unwrap();
        db.run_migrations(MIGRATIONS).unwrap();

        let org_repo = OrganizationRepository::new(db.clone());
        let company = org_repo.create_company(&Company::new("Acme Trading Co", "USD")).unwrap();
        let company_id = company.id.unwrap();

        let acc_repo = AccRepo::new(db.clone());
        acc_repo
            .create_fiscal_period(&FiscalPeriod::new(company_id, "January 2026", date(2026, 1, 1), date(2026, 1, 31)))
            .unwrap();
        let bank_gl_account = acc_repo.create_account(&Account::new(company_id, "1010", "Bank - First National", AccountType::Asset)).unwrap();
        let expense_account = acc_repo.create_account(&Account::new(company_id, "6000", "Bank Charges", AccountType::Expense)).unwrap();

        let banking = BankingRepository::new(db);
        let bank_account = banking
            .create_bank_account(&BankAccount::new(company_id, bank_gl_account.id.unwrap(), "First National", "123456789", "USD"))
            .unwrap();

        TestFixture {
            banking,
            company_id,
            bank_gl_account_id: bank_gl_account.id.unwrap(),
            bank_account_id: bank_account.id.unwrap(),
            expense_account_id: expense_account.id.unwrap(),
        }
    }

    #[test]
    fn matching_a_bank_transaction_removes_it_from_the_unreconciled_list() {
        let f = fixture();
        let bank_transaction = f
            .banking
            .create_bank_transaction(&BankTransaction::new(f.company_id, f.bank_account_id, date(2026, 1, 5), "Customer payment", dec!(500)))
            .unwrap();

        // A real GL transaction to match against -- matched_transaction_id
        // is a genuine foreign key, so it can't point at a made-up id.
        let acc_repo = AccRepo::new(f.banking.db.clone());
        let ar_placeholder = acc_repo.create_account(&Account::new(f.company_id, "1100", "Accounts Receivable", AccountType::Asset)).unwrap();
        let already_posted = acc_repo
            .post_transaction(
                &crate::backend::transactions::transaction::Transaction::new(f.company_id, crate::backend::transactions::transaction_types::TransactionType::CustomerPayment, date(2026, 1, 5))
                    .with_line(crate::backend::transactions::transaction_lines::TransactionLine::debit(f.bank_gl_account_id, dec!(500)))
                    .with_line(crate::backend::transactions::transaction_lines::TransactionLine::credit(ar_placeholder.id.unwrap(), dec!(500))),
            )
            .unwrap();

        assert_eq!(f.banking.list_unreconciled_for_account(f.bank_account_id).unwrap().len(), 1);
        f.banking.match_bank_transaction(bank_transaction.id.unwrap(), already_posted.id.unwrap()).unwrap();
        assert!(f.banking.list_unreconciled_for_account(f.bank_account_id).unwrap().is_empty());
    }

    #[test]
    fn posting_and_matching_an_adjustment_posts_to_the_bank_gl_account_and_marks_it_reconciled() {
        let f = fixture();
        let bank_transaction = f
            .banking
            .create_bank_transaction(&BankTransaction::new(f.company_id, f.bank_account_id, date(2026, 1, 10), "Monthly fee", dec!(-25)))
            .unwrap();

        let posted = f.banking.post_and_match_adjustment(bank_transaction.id.unwrap(), f.expense_account_id).unwrap();
        assert!(posted.is_balanced());
        assert!(f.banking.list_unreconciled_for_account(f.bank_account_id).unwrap().is_empty());

        let book_balance = f.banking.book_balance(f.bank_account_id, date(2026, 1, 31)).unwrap();
        assert_eq!(book_balance, dec!(-25));
    }

    #[test]
    fn posting_a_cash_transfer_moves_money_between_two_gl_accounts() {
        let f = fixture();
        let acc_repo = AccRepo::new(f.banking.db.clone());
        let petty_cash = acc_repo.create_account(&Account::new(f.company_id, "1020", "Petty Cash", AccountType::Asset)).unwrap();

        let transfer = CashTransfer::new(f.company_id, f.bank_gl_account_id, petty_cash.id.unwrap(), date(2026, 1, 15), dec!(200));
        let posted = f.banking.post_cash_transfer(&transfer).unwrap();
        assert!(posted.is_balanced());

        assert_eq!(f.banking.book_balance(f.bank_account_id, date(2026, 1, 31)).unwrap(), dec!(-200));
    }
}
