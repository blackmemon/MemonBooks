//! Persistence for bank accounts and bank transactions (reconciliation
//! staging records). Posting adjustment/cash-transfer transactions
//! delegates to `AccountingRepository`, same pattern as
//! `sales_repository.rs`/`purchase_repository.rs`.

use rusqlite::{params, Connection, OptionalExtension, Row};
use rust_decimal::Decimal;

use crate::backend::banking::bank_accounts::{self, BankAccount, BankAccountType};
use crate::backend::banking::bank_ledger::{BankLedger, BankLedgerEntry, BankLedgerLine};
use crate::backend::banking::bank_transactions::BankTransaction;
use crate::backend::banking::cash_management::CashTransfer;
use crate::backend::core::errors::{AppError, AppResult};
use crate::backend::core::validation::Validate;
use crate::backend::repositories::accounting_repository::AccountingRepository;
use crate::backend::transactions::transaction::Transaction;
use crate::database::database::{Database, Migration};

pub const MIGRATIONS: &[Migration] = &[
    Migration {
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
    },
    Migration {
        // 1201: retrofits the Bank Accounts page's profile fields onto
        // the table 1200 shipped -- additive only, so existing rows get
        // empty companies, the Debit type and a zero opening balance.
        // Account numbers are kept unique per company by the repository
        // rather than a UNIQUE index, so a database that already holds a
        // duplicate still migrates.
        version: 1201,
        name: "bank_accounts_profile_fields",
        sql: "
            ALTER TABLE bank_accounts ADD COLUMN parent_company TEXT NOT NULL DEFAULT '';
            ALTER TABLE bank_accounts ADD COLUMN sub_company TEXT NOT NULL DEFAULT '';
            ALTER TABLE bank_accounts ADD COLUMN account_type TEXT NOT NULL DEFAULT 'Debit';
            ALTER TABLE bank_accounts ADD COLUMN opening_balance TEXT NOT NULL DEFAULT '0';
        ",
    },
];

/// Column list matching [`map_bank_account`].
const BANK_ACCOUNT_COLUMNS: &str = "id, company_id, gl_account_id, bank_name, account_number, currency_code, is_active, parent_company, sub_company, account_type, opening_balance";

/// Documents that record the GL transaction they posted, with the kind
/// and reference prefix a bank ledger entry names them by.
const ORIGIN_DOCUMENTS: &[(&str, &str, &str)] = &[
    ("customer_payments", "Customer Receipt", "RCPT"),
    ("vendor_payments", "Vendor Payment", "PAY"),
    ("invoices", "Sales Invoice", "INV"),
    ("credit_notes", "Credit Note", "CN"),
    ("bills", "Bill", "BILL"),
    ("debit_notes", "Debit Note", "DN"),
    ("expenses", "Expense", "EXP"),
    ("expense_claims", "Expense Claim", "CLAIM"),
    ("pos_sales", "POS Sale", "POS"),
    ("payroll_runs", "Payroll", "PR"),
    ("assets", "Fixed Asset", "FA"),
    ("stock_movements", "Stock Movement", "SM"),
];

/// What [`BankingRepository::remove_bank_account`] did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Removal {
    Deleted,
    /// The account has history, so it was made inactive instead.
    Archived,
}

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
            ensure_unique_account_number(conn, account)?;
            conn.execute(
                "INSERT INTO bank_accounts (company_id, gl_account_id, bank_name, account_number, currency_code, is_active, parent_company, sub_company, account_type, opening_balance)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![account.company_id, account.gl_account_id, account.bank_name, account.account_number, account.currency_code, account.is_active, account.parent_company, account.sub_company, account.account_type.as_str(), account.opening_balance.to_string()],
            )?;
            Ok(BankAccount { id: Some(conn.last_insert_rowid()), ..account.clone() })
        })
    }

    pub fn list_bank_accounts_for_company(&self, company_id: i64) -> AppResult<Vec<BankAccount>> {
        self.db.with_connection(|conn| {
            let mut stmt = conn.prepare(&format!(
                "SELECT {BANK_ACCOUNT_COLUMNS} FROM bank_accounts WHERE company_id = ?1 ORDER BY bank_name"
            ))?;
            let rows = stmt.query_map(params![company_id], map_bank_account)?;
            rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
        })
    }

    pub fn update_bank_account(&self, account: &BankAccount) -> AppResult<()> {
        account.validate()?;
        let id = account.id.ok_or_else(|| {
            AppError::Validation("cannot update a bank account without an id".into())
        })?;
        self.db.with_connection(|conn| {
            ensure_unique_account_number(conn, account)?;
            let affected = conn.execute(
                "UPDATE bank_accounts SET gl_account_id = ?1, bank_name = ?2, account_number = ?3, currency_code = ?4, is_active = ?5, parent_company = ?6, sub_company = ?7, account_type = ?8, opening_balance = ?9 WHERE id = ?10",
                params![account.gl_account_id, account.bank_name, account.account_number, account.currency_code, account.is_active, account.parent_company, account.sub_company, account.account_type.as_str(), account.opening_balance.to_string(), id],
            )?;
            if affected == 0 {
                return Err(AppError::NotFound(format!("bank account {id} not found")));
            }
            Ok(())
        })
    }

    /// Rejects with a clear message, rather than a raw SQLite one, when
    /// the bank account is still referenced elsewhere -- bank
    /// transactions reference a bank account by id, and that foreign
    /// key is enforced.
    pub fn delete_bank_account(&self, id: i64) -> AppResult<()> {
        self.db
            .with_connection(|conn| {
                let affected = conn.execute("DELETE FROM bank_accounts WHERE id = ?1", params![id])?;
                if affected == 0 {
                    return Err(AppError::NotFound(format!("bank account {id} not found")));
                }
                Ok(())
            })
            .map_err(|err| {
                if err.is_constraint_violation() {
                    AppError::Validation(format!("cannot delete bank account {id}: it's still referenced by bank transactions or other records"))
                } else {
                    err
                }
            })
    }

    pub fn get_bank_account(&self, id: i64) -> AppResult<BankAccount> {
        self.db.with_connection(|conn| {
            conn.query_row(
                &format!("SELECT {BANK_ACCOUNT_COLUMNS} FROM bank_accounts WHERE id = ?1"),
                params![id],
                map_bank_account,
            )
            .optional()?
            .ok_or_else(|| AppError::NotFound(format!("bank account {id} not found")))
        })
    }

    /// Deletes a bank account with no history; one that has posted
    /// transactions or statement lines is archived (made inactive)
    /// instead, so its history stays intact.
    pub fn remove_bank_account(&self, id: i64) -> AppResult<Removal> {
        let account = self.get_bank_account(id)?;
        let statement_lines: i64 = self.db.with_connection(|conn| {
            conn.query_row(
                "SELECT COUNT(*) FROM bank_transactions WHERE bank_account_id = ?1",
                params![id],
                |row| row.get(0),
            )
            .map_err(AppError::from)
        })?;
        if statement_lines == 0 && self.bank_ledger_summary(id)?.entries.is_empty() {
            self.delete_bank_account(id)?;
            return Ok(Removal::Deleted);
        }
        self.update_bank_account(&BankAccount {
            is_active: false,
            ..account
        })?;
        Ok(Removal::Archived)
    }

    /// Every posted (non-draft) GL transaction that touches this bank
    /// account, read-only -- same approach as
    /// `PurchaseRepository::vendor_ledger`. A posting belongs to the bank
    /// account when it hits the account's linked GL account. When several
    /// bank accounts share one GL account a posting can't be told apart
    /// by its GL account alone, so only the postings reconciled against
    /// this account's own statement lines are listed.
    pub fn bank_ledger(&self, bank_account_id: i64) -> AppResult<BankLedger> {
        self.ledger(bank_account_id, true)
    }

    /// [`Self::bank_ledger`] without each entry's originating document and
    /// lines -- enough for balances, and much cheaper for a list of cards.
    pub fn bank_ledger_summary(&self, bank_account_id: i64) -> AppResult<BankLedger> {
        self.ledger(bank_account_id, false)
    }

    fn ledger(&self, bank_account_id: i64, details: bool) -> AppResult<BankLedger> {
        let account = self.get_bank_account(bank_account_id)?;
        let entries = self.db.with_connection(|conn| {
            let sharing: i64 = conn.query_row(
                "SELECT COUNT(*) FROM bank_accounts WHERE gl_account_id = ?1 AND id != ?2",
                params![account.gl_account_id, bank_account_id],
                |row| row.get(0),
            )?;
            let filter = if sharing > 0 {
                "t.id IN (SELECT matched_transaction_id FROM bank_transactions WHERE bank_account_id = ?2)"
            } else {
                "?2 IS NOT NULL"
            };
            let mut stmt = conn.prepare(&format!(
                "SELECT t.id, t.transaction_date, t.posted_at, t.transaction_type, t.description, tl.description, tl.debit, tl.credit
                 FROM transaction_lines tl
                 JOIN transactions t ON t.id = tl.transaction_id
                 WHERE tl.account_id = ?1 AND t.company_id = ?3 AND t.status != 'draft' AND {filter}
                 ORDER BY t.transaction_date, t.id, tl.line_number"
            ))?;
            let rows = stmt.query_map(
                params![account.gl_account_id, bank_account_id, account.company_id],
                map_ledger_entry,
            )?;
            let mut entries = rows.collect::<Result<Vec<_>, _>>()?;
            for entry in entries.iter_mut().filter(|_| details) {
                if let Some((kind, reference)) = origin_of(conn, entry.transaction_id)? {
                    entry.kind = kind;
                    entry.reference = reference;
                }
                entry.lines = transaction_lines(conn, entry.transaction_id)?;
            }
            Ok(entries)
        })?;
        Ok(BankLedger::new(
            account.account_type,
            account.opening_balance,
            entries,
        ))
    }

    // -- bank transactions (reconciliation staging) ------------------

    pub fn create_bank_transaction(
        &self,
        transaction: &BankTransaction,
    ) -> AppResult<BankTransaction> {
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

    pub fn list_unreconciled_for_account(
        &self,
        bank_account_id: i64,
    ) -> AppResult<Vec<BankTransaction>> {
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
    pub fn match_bank_transaction(
        &self,
        bank_transaction_id: i64,
        gl_transaction_id: i64,
    ) -> AppResult<()> {
        self.db.with_connection(|conn| {
            let affected = conn.execute(
                "UPDATE bank_transactions SET matched_transaction_id = ?1 WHERE id = ?2",
                params![gl_transaction_id, bank_transaction_id],
            )?;
            if affected == 0 {
                return Err(AppError::NotFound(format!(
                    "bank transaction {bank_transaction_id} not found"
                )));
            }
            Ok(())
        })
    }

    /// For a bank-only item (no existing GL entry): posts the adjustment
    /// transaction and marks this bank transaction matched against it, in
    /// one atomic step.
    pub fn post_and_match_adjustment(
        &self,
        bank_transaction_id: i64,
        offset_account_id: i64,
    ) -> AppResult<Transaction> {
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
            return Err(AppError::Validation(
                "bank transaction is already matched".into(),
            ));
        }

        let bank_account = self.get_bank_account(bank_transaction.bank_account_id)?;

        let gl_transaction = bank_transaction
            .build_adjustment_transaction(bank_account.gl_account_id, offset_account_id);
        let posted = self.accounting.post_transaction(&gl_transaction)?;
        self.match_bank_transaction(bank_transaction_id, posted.id.unwrap())?;
        Ok(posted)
    }

    // -- cash management --------------------------------------------------

    pub fn post_cash_transfer(&self, transfer: &CashTransfer) -> AppResult<Transaction> {
        transfer.validate()?;
        self.accounting
            .post_transaction(&transfer.build_transaction())
    }

    /// The GL balance for a bank account as of a date -- the "book
    /// balance" side of a reconciliation.
    pub fn book_balance(
        &self,
        bank_account_id: i64,
        as_of: chrono::NaiveDate,
    ) -> AppResult<Decimal> {
        let bank_account = self.get_bank_account(bank_account_id)?;

        let ledger = self.accounting.general_ledger_for_account(
            bank_account.gl_account_id,
            Decimal::ZERO,
            as_of,
        )?;
        Ok(ledger
            .last()
            .map(|entry| entry.running_balance)
            .unwrap_or(Decimal::ZERO))
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
        parent_company: row.get(7)?,
        sub_company: row.get(8)?,
        account_type: BankAccountType::parse(&row.get::<_, String>(9)?).unwrap_or_default(),
        opening_balance: row.get::<_, String>(10)?.parse().unwrap_or(Decimal::ZERO),
    })
}

/// One bank ledger row from `bank_ledger`'s query; the kind starts as the
/// transaction type and the reference as `TX-<id>` until a document
/// claims the transaction.
fn map_ledger_entry(row: &Row) -> rusqlite::Result<BankLedgerEntry> {
    let transaction_id: i64 = row.get(0)?;
    let posted_at: Option<String> = row.get(2)?;
    let debit: String = row.get(6)?;
    let credit: String = row.get(7)?;
    Ok(BankLedgerEntry {
        transaction_id,
        date: row.get(1)?,
        posted_at: posted_at.as_deref().and_then(parse_posted_at),
        kind: kind_label(&row.get::<_, String>(3)?),
        reference: format!("TX-{transaction_id}"),
        description: row
            .get::<_, Option<String>>(5)?
            .or(row.get::<_, Option<String>>(4)?)
            .unwrap_or_default(),
        debit: debit.parse().unwrap_or(Decimal::ZERO),
        credit: credit.parse().unwrap_or(Decimal::ZERO),
        running_balance: Decimal::ZERO,
        lines: Vec::new(),
    })
}

/// Rejects `account` when another bank account of the same company
/// already uses its account number (ignoring case, spaces and dashes).
fn ensure_unique_account_number(conn: &Connection, account: &BankAccount) -> AppResult<()> {
    let mut stmt =
        conn.prepare("SELECT id, account_number FROM bank_accounts WHERE company_id = ?1")?;
    let rows = stmt.query_map(params![account.company_id], |r| {
        Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
    })?;
    let wanted = account.normalized_account_number();
    for row in rows {
        let (id, number) = row?;
        if Some(id) != account.id && bank_accounts::normalize_account_number(&number) == wanted {
            return Err(AppError::Validation(format!(
                "account number {} is already used by another bank account",
                account.account_number.trim()
            )));
        }
    }
    Ok(())
}

/// `posted_at` as SQLite's `strftime('%Y-%m-%dT%H:%M:%fZ')` writes it.
fn parse_posted_at(value: &str) -> Option<chrono::NaiveDateTime> {
    chrono::NaiveDateTime::parse_from_str(value.trim_end_matches('Z'), "%Y-%m-%dT%H:%M:%S%.f").ok()
}

/// `vendor_payment` -> `Vendor Payment`.
fn kind_label(transaction_type: &str) -> String {
    transaction_type
        .split('_')
        .filter(|word| !word.is_empty())
        .map(|word| {
            let mut chars = word.chars();
            chars
                .next()
                .map(|first| first.to_uppercase().chain(chars).collect())
                .unwrap_or_default()
        })
        .collect::<Vec<String>>()
        .join(" ")
}

/// The document that posted `transaction_id`, if one of
/// [`ORIGIN_DOCUMENTS`] records it; tables this database lacks are
/// skipped.
fn origin_of(conn: &Connection, transaction_id: i64) -> AppResult<Option<(String, String)>> {
    for (table, kind, prefix) in ORIGIN_DOCUMENTS {
        let exists: bool = conn.query_row(
            "SELECT EXISTS (SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1)",
            params![table],
            |r| r.get(0),
        )?;
        if !exists {
            continue;
        }
        let id: Option<i64> = conn
            .query_row(
                &format!("SELECT id FROM {table} WHERE transaction_id = ?1 LIMIT 1"),
                params![transaction_id],
                |r| r.get(0),
            )
            .optional()?;
        if let Some(id) = id {
            return Ok(Some((kind.to_string(), format!("{prefix}-{id}"))));
        }
    }
    Ok(None)
}

/// Every line of a GL transaction with its account, for a ledger entry's
/// details.
fn transaction_lines(conn: &Connection, transaction_id: i64) -> AppResult<Vec<BankLedgerLine>> {
    let mut stmt = conn.prepare(
        "SELECT a.account_code, a.account_name, tl.debit, tl.credit
         FROM transaction_lines tl JOIN accounts a ON a.id = tl.account_id
         WHERE tl.transaction_id = ?1 ORDER BY tl.line_number",
    )?;
    let rows = stmt.query_map(params![transaction_id], |r| {
        let debit: String = r.get(2)?;
        let credit: String = r.get(3)?;
        Ok(BankLedgerLine {
            account_code: r.get(0)?,
            account_name: r.get(1)?,
            debit: debit.parse().unwrap_or(Decimal::ZERO),
            credit: credit.parse().unwrap_or(Decimal::ZERO),
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
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
    use crate::backend::repositories::accounting_repository::{
        self, AccountingRepository as AccRepo,
    };
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
        db.run_migrations(organization_repository::MIGRATIONS)
            .unwrap();
        db.run_migrations(accounting_repository::MIGRATIONS)
            .unwrap();
        db.run_migrations(crate::backend::audit::audit_logs::MIGRATIONS)
            .unwrap();
        db.run_migrations(MIGRATIONS).unwrap();

        let org_repo = OrganizationRepository::new(db.clone());
        let company = org_repo
            .create_company(&Company::new("Acme Trading Co", "USD"))
            .unwrap();
        let company_id = company.id.unwrap();

        let acc_repo = AccRepo::new(db.clone());
        acc_repo
            .create_fiscal_period(&FiscalPeriod::new(
                company_id,
                "January 2026",
                date(2026, 1, 1),
                date(2026, 1, 31),
            ))
            .unwrap();
        let bank_gl_account = acc_repo
            .create_account(&Account::new(
                company_id,
                "1010",
                "Bank - First National",
                AccountType::Asset,
            ))
            .unwrap();
        let expense_account = acc_repo
            .create_account(&Account::new(
                company_id,
                "6000",
                "Bank Charges",
                AccountType::Expense,
            ))
            .unwrap();

        let banking = BankingRepository::new(db);
        let bank_account = banking
            .create_bank_account(&BankAccount::new(
                company_id,
                bank_gl_account.id.unwrap(),
                "First National",
                "123456789",
                "USD",
            ))
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
    fn update_bank_account_persists_changed_fields() {
        let f = fixture();
        let account = f
            .banking
            .list_bank_accounts_for_company(f.company_id)
            .unwrap()
            .into_iter()
            .next()
            .unwrap();

        f.banking
            .update_bank_account(&BankAccount {
                bank_name: "Second National".into(),
                ..account
            })
            .unwrap();

        let updated = f
            .banking
            .list_bank_accounts_for_company(f.company_id)
            .unwrap();
        assert_eq!(updated[0].bank_name, "Second National");
    }

    #[test]
    fn delete_bank_account_removes_an_unreferenced_bank_account() {
        let f = fixture();

        f.banking.delete_bank_account(f.bank_account_id).unwrap();

        assert!(f
            .banking
            .list_bank_accounts_for_company(f.company_id)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn delete_bank_account_is_rejected_with_a_clear_message_when_still_referenced_by_a_bank_transaction(
    ) {
        let f = fixture();
        f.banking
            .create_bank_transaction(&BankTransaction::new(
                f.company_id,
                f.bank_account_id,
                date(2026, 1, 5),
                "Customer payment",
                dec!(500),
            ))
            .unwrap();

        let result = f.banking.delete_bank_account(f.bank_account_id);

        assert!(matches!(result, Err(AppError::Validation(_))));
        assert!(result.unwrap_err().to_string().contains("still referenced"));
    }

    #[test]
    fn matching_a_bank_transaction_removes_it_from_the_unreconciled_list() {
        let f = fixture();
        let bank_transaction = f
            .banking
            .create_bank_transaction(&BankTransaction::new(
                f.company_id,
                f.bank_account_id,
                date(2026, 1, 5),
                "Customer payment",
                dec!(500),
            ))
            .unwrap();

        // A real GL transaction to match against -- matched_transaction_id
        // is a genuine foreign key, so it can't point at a made-up id.
        let acc_repo = AccRepo::new(f.banking.db.clone());
        let ar_placeholder = acc_repo
            .create_account(&Account::new(
                f.company_id,
                "1100",
                "Accounts Receivable",
                AccountType::Asset,
            ))
            .unwrap();
        let already_posted = acc_repo
            .post_transaction(
                &crate::backend::transactions::transaction::Transaction::new(f.company_id, crate::backend::transactions::transaction_types::TransactionType::CustomerPayment, date(2026, 1, 5))
                    .with_line(crate::backend::transactions::transaction_lines::TransactionLine::debit(f.bank_gl_account_id, dec!(500)))
                    .with_line(crate::backend::transactions::transaction_lines::TransactionLine::credit(ar_placeholder.id.unwrap(), dec!(500))),
            )
            .unwrap();

        assert_eq!(
            f.banking
                .list_unreconciled_for_account(f.bank_account_id)
                .unwrap()
                .len(),
            1
        );
        f.banking
            .match_bank_transaction(bank_transaction.id.unwrap(), already_posted.id.unwrap())
            .unwrap();
        assert!(f
            .banking
            .list_unreconciled_for_account(f.bank_account_id)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn posting_and_matching_an_adjustment_posts_to_the_bank_gl_account_and_marks_it_reconciled() {
        let f = fixture();
        let bank_transaction = f
            .banking
            .create_bank_transaction(&BankTransaction::new(
                f.company_id,
                f.bank_account_id,
                date(2026, 1, 10),
                "Monthly fee",
                dec!(-25),
            ))
            .unwrap();

        let posted = f
            .banking
            .post_and_match_adjustment(bank_transaction.id.unwrap(), f.expense_account_id)
            .unwrap();
        assert!(posted.is_balanced());
        assert!(f
            .banking
            .list_unreconciled_for_account(f.bank_account_id)
            .unwrap()
            .is_empty());

        let book_balance = f
            .banking
            .book_balance(f.bank_account_id, date(2026, 1, 31))
            .unwrap();
        assert_eq!(book_balance, dec!(-25));
    }

    #[test]
    fn posting_a_cash_transfer_moves_money_between_two_gl_accounts() {
        let f = fixture();
        let acc_repo = AccRepo::new(f.banking.db.clone());
        let petty_cash = acc_repo
            .create_account(&Account::new(
                f.company_id,
                "1020",
                "Petty Cash",
                AccountType::Asset,
            ))
            .unwrap();

        let transfer = CashTransfer::new(
            f.company_id,
            f.bank_gl_account_id,
            petty_cash.id.unwrap(),
            date(2026, 1, 15),
            dec!(200),
        );
        let posted = f.banking.post_cash_transfer(&transfer).unwrap();
        assert!(posted.is_balanced());

        assert_eq!(
            f.banking
                .book_balance(f.bank_account_id, date(2026, 1, 31))
                .unwrap(),
            dec!(-200)
        );
    }

    /// Posts `amount` between the bank's GL account (debited when
    /// `into_bank`) and the expense account, returning the transaction id.
    fn post(
        f: &TestFixture,
        gl_account_id: i64,
        day: u32,
        amount: Decimal,
        into_bank: bool,
    ) -> i64 {
        use crate::backend::transactions::transaction_lines::TransactionLine;
        use crate::backend::transactions::transaction_types::TransactionType;
        let (bank, other) = if into_bank {
            (
                TransactionLine::debit(gl_account_id, amount),
                TransactionLine::credit(f.expense_account_id, amount),
            )
        } else {
            (
                TransactionLine::credit(gl_account_id, amount),
                TransactionLine::debit(f.expense_account_id, amount),
            )
        };
        AccRepo::new(f.banking.db.clone())
            .post_transaction(
                &Transaction::new(
                    f.company_id,
                    TransactionType::ManualJournal,
                    date(2026, 1, day),
                )
                .with_line(bank)
                .with_line(other),
            )
            .unwrap()
            .id
            .unwrap()
    }

    fn second_gl_account(f: &TestFixture, code: &str) -> i64 {
        AccRepo::new(f.banking.db.clone())
            .create_account(&Account::new(
                f.company_id,
                code,
                "Bank - Second",
                AccountType::Asset,
            ))
            .unwrap()
            .id
            .unwrap()
    }

    #[test]
    fn the_profile_fields_are_saved_and_reloaded() {
        let f = fixture();
        let mut account = BankAccount::new(
            f.company_id,
            f.bank_gl_account_id,
            "Meezan Bank",
            "PK00-MEZN-7788",
            "PKR",
        );
        account.parent_company = "Memon Group".into();
        account.sub_company = "Memon Textiles".into();
        account.account_type = BankAccountType::Credit;
        account.opening_balance = dec!(1250.50);
        let saved = f.banking.create_bank_account(&account).unwrap();

        let reloaded = f.banking.get_bank_account(saved.id.unwrap()).unwrap();
        assert_eq!(reloaded, saved);
        assert_eq!(reloaded.parent_company, "Memon Group");
        assert_eq!(reloaded.account_type, BankAccountType::Credit);
        assert_eq!(reloaded.opening_balance, dec!(1250.50));
    }

    #[test]
    fn a_row_from_before_the_profile_migration_still_loads_with_defaults() {
        let db = Database::open_in_memory().unwrap();
        db.run_migrations(organization_repository::MIGRATIONS)
            .unwrap();
        db.run_migrations(accounting_repository::MIGRATIONS)
            .unwrap();
        db.run_migrations(&MIGRATIONS[..1]).unwrap();
        let company_id = OrganizationRepository::new(db.clone())
            .create_company(&Company::new("Acme Trading Co", "USD"))
            .unwrap()
            .id
            .unwrap();
        let gl = AccRepo::new(db.clone())
            .create_account(&Account::new(
                company_id,
                "1010",
                "Bank",
                AccountType::Asset,
            ))
            .unwrap()
            .id
            .unwrap();
        db.with_connection(|conn| {
            conn.execute(
                "INSERT INTO bank_accounts (company_id, gl_account_id, bank_name, account_number, currency_code, is_active)
                 VALUES (?1, ?2, 'Old Bank', '0001', 'USD', 1)",
                params![company_id, gl],
            )?;
            Ok(())
        })
        .unwrap();

        db.run_migrations(MIGRATIONS).unwrap();

        let accounts = BankingRepository::new(db)
            .list_bank_accounts_for_company(company_id)
            .unwrap();
        assert_eq!(accounts.len(), 1);
        assert_eq!(accounts[0].bank_name, "Old Bank");
        assert_eq!(accounts[0].account_type, BankAccountType::Debit);
        assert_eq!(accounts[0].opening_balance, Decimal::ZERO);
        assert!(accounts[0].parent_company.is_empty());
    }

    #[test]
    fn an_account_number_already_in_use_is_rejected_on_create_and_update() {
        let f = fixture();
        let duplicate = BankAccount::new(
            f.company_id,
            f.bank_gl_account_id,
            "Other Bank",
            " 1234-56789 ",
            "USD",
        );
        let result = f.banking.create_bank_account(&duplicate);
        assert!(matches!(result, Err(AppError::Validation(_))));
        assert!(result.unwrap_err().to_string().contains("already used"));

        let other = f
            .banking
            .create_bank_account(&BankAccount::new(
                f.company_id,
                f.bank_gl_account_id,
                "Other Bank",
                "555",
                "USD",
            ))
            .unwrap();
        let renamed = BankAccount {
            account_number: "123456789".into(),
            ..other.clone()
        };
        assert!(f.banking.update_bank_account(&renamed).is_err());
        // Saving an account under its own number is not a clash.
        f.banking.update_bank_account(&other).unwrap();
    }

    #[test]
    fn the_ledger_balances_follow_the_account_type_from_the_opening_balance() {
        let f = fixture();
        let mut account = f.banking.get_bank_account(f.bank_account_id).unwrap();
        account.opening_balance = dec!(1000);
        f.banking.update_bank_account(&account).unwrap();
        post(&f, f.bank_gl_account_id, 5, dec!(400), true);
        post(&f, f.bank_gl_account_id, 9, dec!(150), false);

        let ledger = f.banking.bank_ledger(f.bank_account_id).unwrap();
        let balances = ledger.balances();
        assert_eq!(ledger.entries.len(), 2);
        assert_eq!(balances.debit, dec!(400));
        assert_eq!(balances.credit, dec!(150));
        assert_eq!(balances.net(), dec!(1250));
        assert_eq!(ledger.entries[1].running_balance, dec!(1250));
        assert_eq!(ledger.last_transaction_date(), Some(date(2026, 1, 9)));
        assert_eq!(ledger.entries[0].kind, "Manual Journal");
        assert!(ledger.entries[0].reference.starts_with("TX-"));
        assert_eq!(ledger.entries[0].lines.len(), 2);
        assert!(ledger.entries[0].posted_at.is_some());

        account.account_type = BankAccountType::Credit;
        f.banking.update_bank_account(&account).unwrap();
        let credit = f.banking.bank_ledger(f.bank_account_id).unwrap();
        assert_eq!(credit.balances().net(), dec!(750));
    }

    #[test]
    fn the_ledger_lists_only_this_accounts_transactions() {
        let f = fixture();
        let other_gl = second_gl_account(&f, "1011");
        let other = f
            .banking
            .create_bank_account(&BankAccount::new(
                f.company_id,
                other_gl,
                "Second National",
                "987654321",
                "USD",
            ))
            .unwrap();
        let mine = post(&f, f.bank_gl_account_id, 5, dec!(100), true);
        let theirs = post(&f, other_gl, 6, dec!(999), true);

        let ledger = f.banking.bank_ledger(f.bank_account_id).unwrap();
        let ids: Vec<i64> = ledger.entries.iter().map(|e| e.transaction_id).collect();
        assert_eq!(ids, vec![mine]);
        let other_ledger = f.banking.bank_ledger(other.id.unwrap()).unwrap();
        let other_ids: Vec<i64> = other_ledger
            .entries
            .iter()
            .map(|e| e.transaction_id)
            .collect();
        assert_eq!(other_ids, vec![theirs]);
    }

    #[test]
    fn accounts_sharing_a_gl_account_list_only_their_own_reconciled_postings() {
        let f = fixture();
        let sibling = f
            .banking
            .create_bank_account(&BankAccount::new(
                f.company_id,
                f.bank_gl_account_id,
                "First National",
                "111222333",
                "USD",
            ))
            .unwrap();
        let matched = post(&f, f.bank_gl_account_id, 5, dec!(100), true);
        post(&f, f.bank_gl_account_id, 6, dec!(50), true);
        let line = f
            .banking
            .create_bank_transaction(&BankTransaction::new(
                f.company_id,
                f.bank_account_id,
                date(2026, 1, 5),
                "Deposit",
                dec!(100),
            ))
            .unwrap();
        f.banking
            .match_bank_transaction(line.id.unwrap(), matched)
            .unwrap();

        let mine = f.banking.bank_ledger(f.bank_account_id).unwrap();
        assert_eq!(mine.entries.len(), 1);
        assert_eq!(mine.entries[0].transaction_id, matched);
        assert!(f
            .banking
            .bank_ledger(sibling.id.unwrap())
            .unwrap()
            .entries
            .is_empty());
    }

    #[test]
    fn removing_deletes_an_account_without_history_and_archives_one_with_it() {
        let f = fixture();
        let free_gl = second_gl_account(&f, "1012");
        let free = f
            .banking
            .create_bank_account(&BankAccount::new(
                f.company_id,
                free_gl,
                "Unused Bank",
                "000111",
                "USD",
            ))
            .unwrap();
        assert_eq!(
            f.banking.remove_bank_account(free.id.unwrap()).unwrap(),
            Removal::Deleted
        );

        post(&f, f.bank_gl_account_id, 5, dec!(100), true);
        assert_eq!(
            f.banking.remove_bank_account(f.bank_account_id).unwrap(),
            Removal::Archived
        );
        let archived = f.banking.get_bank_account(f.bank_account_id).unwrap();
        assert!(!archived.is_active);
        assert_eq!(
            f.banking
                .list_bank_accounts_for_company(f.company_id)
                .unwrap()
                .len(),
            1
        );
    }
}
