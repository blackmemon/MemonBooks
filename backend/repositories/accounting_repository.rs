//! Persistence for the accounting domain. Starts with the chart of
//! accounts and fiscal periods (Development Order step 5); journal
//! entries, the general ledger, and trial balance land in the same
//! repository as later steps build them, since they all share this one
//! migration set and company-scoped schema.

use rust_decimal::Decimal;
use rusqlite::{params, OptionalExtension, Row};

use crate::backend::accounting::accounting_rules::AccountType;
use crate::backend::accounting::accounts_payable::payables::Payable;
use crate::backend::accounting::accounts_receivable::receivables::Receivable;
use crate::backend::accounting::chart_of_accounts::Account;
use crate::backend::accounting::fiscal_periods::{FiscalPeriod, FiscalPeriodStatus};
use crate::backend::accounting::general_ledger::{self, GeneralLedgerEntry, Posting};
use crate::backend::accounting::period_closing::{self, ClosingBalance};
use crate::backend::accounting::trial_balance::{TrialBalance, TrialBalanceLine};
use crate::backend::audit::audit_logs::{self, AuditLog};
use crate::backend::core::errors::{AppError, AppResult};
use crate::backend::core::validation::Validate;
use crate::backend::transactions::transaction::Transaction;
use crate::backend::transactions::transaction_engine;
use crate::backend::transactions::transaction_lines::TransactionLine;
use crate::backend::transactions::transaction_status::TransactionStatus;
use crate::backend::transactions::transaction_types::TransactionType;
use crate::database::database::{Database, Migration};

pub const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 500,
        name: "chart_of_accounts_and_fiscal_periods_schema",
        sql: "
            CREATE TABLE accounts (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                company_id  INTEGER NOT NULL REFERENCES companies(id),
                code        TEXT NOT NULL,
                name        TEXT NOT NULL,
                account_type TEXT NOT NULL,
                parent_id   INTEGER REFERENCES accounts(id),
                is_active   INTEGER NOT NULL DEFAULT 1,
                UNIQUE (company_id, code)
            );

            CREATE TABLE fiscal_periods (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                company_id  INTEGER NOT NULL REFERENCES companies(id),
                name        TEXT NOT NULL,
                start_date  TEXT NOT NULL,
                end_date    TEXT NOT NULL,
                status      TEXT NOT NULL
            );
        ",
    },
    Migration {
        // 501, not part of migration 500's own SQL: retrofits a column
        // onto a table that migration 500 already shipped, same pattern
        // as inventory_repository's "products_barcode_column" (version
        // 1600) -- editing already-applied migration SQL isn't safe.
        version: 501,
        name: "accounts_reference_code_column",
        sql: "
            ALTER TABLE accounts ADD COLUMN reference_code TEXT;
            -- A plain index, not a UNIQUE column constraint on the
            -- ALTER TABLE itself -- SQLite's ADD COLUMN doesn't support
            -- UNIQUE constraints on new columns. NULLs (accounts from
            -- before this column existed) are never considered equal
            -- to each other under a UNIQUE index, so pre-existing rows
            -- with no reference_code don't collide with one another.
            CREATE UNIQUE INDEX accounts_reference_code_unique ON accounts (reference_code) WHERE reference_code IS NOT NULL;
        ",
    },
    Migration {
        version: 600,
        name: "transactions_and_transaction_lines_schema",
        sql: "
            CREATE TABLE transactions (
                id                          INTEGER PRIMARY KEY AUTOINCREMENT,
                company_id                  INTEGER NOT NULL REFERENCES companies(id),
                transaction_type            TEXT NOT NULL,
                transaction_date            TEXT NOT NULL,
                description                 TEXT,
                status                      TEXT NOT NULL,
                reverses_transaction_id     INTEGER REFERENCES transactions(id),
                reversed_by_transaction_id  INTEGER REFERENCES transactions(id),
                posted_by_user_id           INTEGER,
                posted_at                   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
            );

            CREATE TABLE transaction_lines (
                id              INTEGER PRIMARY KEY AUTOINCREMENT,
                transaction_id  INTEGER NOT NULL REFERENCES transactions(id),
                account_id      INTEGER NOT NULL REFERENCES accounts(id),
                debit           TEXT NOT NULL,
                credit          TEXT NOT NULL,
                description     TEXT,
                line_number     INTEGER NOT NULL
            );
        ",
    },
    Migration {
        // 901, not 900: receivables/payables reference customers/vendors,
        // which party_repository's migration (version 900) creates.
        version: 901,
        name: "receivables_and_payables_schema",
        sql: "
            CREATE TABLE receivables (
                id              INTEGER PRIMARY KEY AUTOINCREMENT,
                company_id      INTEGER NOT NULL REFERENCES companies(id),
                customer_id     INTEGER NOT NULL REFERENCES customers(id),
                reference       TEXT,
                invoice_date    TEXT NOT NULL,
                due_date        TEXT NOT NULL,
                amount          TEXT NOT NULL,
                amount_paid     TEXT NOT NULL DEFAULT '0',
                transaction_id  INTEGER REFERENCES transactions(id)
            );

            CREATE TABLE payables (
                id              INTEGER PRIMARY KEY AUTOINCREMENT,
                company_id      INTEGER NOT NULL REFERENCES companies(id),
                vendor_id       INTEGER NOT NULL REFERENCES vendors(id),
                reference       TEXT,
                bill_date       TEXT NOT NULL,
                due_date        TEXT NOT NULL,
                amount          TEXT NOT NULL,
                amount_paid     TEXT NOT NULL DEFAULT '0',
                transaction_id  INTEGER REFERENCES transactions(id)
            );
        ",
    },
];

#[derive(Clone)]
pub struct AccountingRepository {
    db: Database,
}

impl AccountingRepository {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    // -- chart of accounts ----------------------------------------------

    pub fn create_account(&self, account: &Account) -> AppResult<Account> {
        account.validate()?;
        self.db.with_connection(|conn| {
            conn.execute(
                "INSERT INTO accounts (company_id, code, name, account_type, parent_id, is_active, reference_code)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    account.company_id,
                    account.code,
                    account.name,
                    account.account_type.as_str(),
                    account.parent_id,
                    account.is_active,
                    account.reference_code,
                ],
            )?;
            Ok(Account {
                id: Some(conn.last_insert_rowid()),
                ..account.clone()
            })
        })
    }

    pub fn get_account(&self, id: i64) -> AppResult<Account> {
        self.db.with_connection(|conn| {
            conn.query_row(
                "SELECT id, company_id, code, name, account_type, parent_id, is_active, reference_code FROM accounts WHERE id = ?1",
                params![id],
                map_account,
            )
            .optional()?
            .ok_or_else(|| AppError::NotFound(format!("account {id} not found")))
        })
    }

    pub fn get_account_by_code(&self, company_id: i64, code: &str) -> AppResult<Option<Account>> {
        self.db.with_connection(|conn| {
            conn.query_row(
                "SELECT id, company_id, code, name, account_type, parent_id, is_active, reference_code
                 FROM accounts WHERE company_id = ?1 AND code = ?2",
                params![company_id, code],
                map_account,
            )
            .optional()
            .map_err(AppError::from)
        })
    }

    pub fn list_accounts_for_company(&self, company_id: i64) -> AppResult<Vec<Account>> {
        self.db.with_connection(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, company_id, code, name, account_type, parent_id, is_active, reference_code
                 FROM accounts WHERE company_id = ?1 ORDER BY code",
            )?;
            let rows = stmt.query_map(params![company_id], map_account)?;
            rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
        })
    }

    pub fn update_account(&self, account: &Account) -> AppResult<()> {
        account.validate()?;
        let id = account
            .id
            .ok_or_else(|| AppError::Validation("cannot update an account without an id".into()))?;
        self.db.with_connection(|conn| {
            let affected = conn.execute(
                "UPDATE accounts SET code = ?1, name = ?2, account_type = ?3, parent_id = ?4, is_active = ?5 WHERE id = ?6",
                params![account.code, account.name, account.account_type.as_str(), account.parent_id, account.is_active, id],
            )?;
            if affected == 0 {
                return Err(AppError::NotFound(format!("account {id} not found")));
            }
            Ok(())
        })
    }

    /// Rejects with a clear message, rather than a raw SQLite one, when
    /// the account is still referenced elsewhere -- posted transaction
    /// lines, another account's `parent_id`, tax rates, fixed asset
    /// categories, and more all reference an account by id, and every
    /// one of those foreign keys is enforced (`PRAGMA foreign_keys` is
    /// on for every connection this app opens).
    pub fn delete_account(&self, id: i64) -> AppResult<()> {
        self.db
            .with_connection(|conn| {
                let affected = conn.execute("DELETE FROM accounts WHERE id = ?1", params![id])?;
                if affected == 0 {
                    return Err(AppError::NotFound(format!("account {id} not found")));
                }
                Ok(())
            })
            .map_err(|err| {
                if err.is_constraint_violation() {
                    AppError::Validation(format!("cannot delete account {id}: it's still referenced by transactions or other records"))
                } else {
                    err
                }
            })
    }

    // -- fiscal periods -------------------------------------------------

    pub fn create_fiscal_period(&self, period: &FiscalPeriod) -> AppResult<FiscalPeriod> {
        period.validate()?;
        self.db.with_connection(|conn| {
            conn.execute(
                "INSERT INTO fiscal_periods (company_id, name, start_date, end_date, status) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![period.company_id, period.name, period.start_date, period.end_date, period.status.as_str()],
            )?;
            Ok(FiscalPeriod {
                id: Some(conn.last_insert_rowid()),
                ..period.clone()
            })
        })
    }

    pub fn get_fiscal_period(&self, id: i64) -> AppResult<FiscalPeriod> {
        self.db.with_connection(|conn| {
            conn.query_row(
                "SELECT id, company_id, name, start_date, end_date, status FROM fiscal_periods WHERE id = ?1",
                params![id],
                map_fiscal_period,
            )
            .optional()?
            .ok_or_else(|| AppError::NotFound(format!("fiscal period {id} not found")))
        })
    }

    pub fn list_fiscal_periods_for_company(&self, company_id: i64) -> AppResult<Vec<FiscalPeriod>> {
        self.db.with_connection(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, company_id, name, start_date, end_date, status
                 FROM fiscal_periods WHERE company_id = ?1 ORDER BY start_date",
            )?;
            let rows = stmt.query_map(params![company_id], map_fiscal_period)?;
            rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
        })
    }

    /// The fiscal period (if any) that contains `date` for this company.
    pub fn fiscal_period_for_date(&self, company_id: i64, date: chrono::NaiveDate) -> AppResult<Option<FiscalPeriod>> {
        self.db.with_connection(|conn| {
            conn.query_row(
                "SELECT id, company_id, name, start_date, end_date, status FROM fiscal_periods
                 WHERE company_id = ?1 AND start_date <= ?2 AND end_date >= ?2",
                params![company_id, date],
                map_fiscal_period,
            )
            .optional()
            .map_err(AppError::from)
        })
    }

    /// Applies a validated status transition (see
    /// `FiscalPeriodStatus::can_transition_to`) and persists it.
    pub fn transition_fiscal_period_status(&self, id: i64, to: FiscalPeriodStatus) -> AppResult<FiscalPeriod> {
        let period = self.get_fiscal_period(id)?;
        if !period.status.can_transition_to(to) {
            return Err(AppError::Validation(format!(
                "cannot transition fiscal period from {} to {}",
                period.status.as_str(),
                to.as_str()
            )));
        }
        self.db.with_connection(|conn| {
            conn.execute(
                "UPDATE fiscal_periods SET status = ?1 WHERE id = ?2",
                params![to.as_str(), id],
            )?;
            Ok(())
        })?;
        Ok(FiscalPeriod { status: to, ..period })
    }

    // -- transactions (the transaction engine's persistence) -----------

    /// Validates `transaction` against the transaction engine (balanced,
    /// draft, dated within an open fiscal period) and, if it passes,
    /// posts it: the transaction header, every line, and an audit log
    /// entry all commit together or not at all (Architecture Rules,
    /// Section 3: atomic multi-table financial posting).
    pub fn post_transaction(&self, transaction: &Transaction) -> AppResult<Transaction> {
        self.validate_for_posting(transaction)?;
        self.db
            .with_transaction(|tx| Self::insert_transaction_and_lines(tx, transaction))
            .map(|transaction_id| Transaction {
                id: Some(transaction_id),
                status: TransactionStatus::Posted,
                ..transaction.clone()
            })
    }

    /// Posts `transaction` and records `receivable` in the same atomic
    /// write -- used by sales invoices/credit notes/payments (Development
    /// Order step 10), so a GL posting can never exist without its
    /// matching subledger entry, or vice versa.
    pub fn post_transaction_with_receivable(&self, transaction: &Transaction, receivable: &Receivable) -> AppResult<(Transaction, Receivable)> {
        self.validate_for_posting(transaction)?;
        receivable.validate()?;
        let (transaction_id, receivable_id) = self.db.with_transaction(|tx| {
            let transaction_id = Self::insert_transaction_and_lines(tx, transaction)?;
            tx.execute(
                "INSERT INTO receivables (company_id, customer_id, reference, invoice_date, due_date, amount, amount_paid, transaction_id)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    receivable.company_id,
                    receivable.customer_id,
                    receivable.reference,
                    receivable.invoice_date,
                    receivable.due_date,
                    receivable.amount.to_string(),
                    receivable.amount_paid.to_string(),
                    transaction_id,
                ],
            )?;
            Ok((transaction_id, tx.last_insert_rowid()))
        })?;
        Ok((
            Transaction { id: Some(transaction_id), status: TransactionStatus::Posted, ..transaction.clone() },
            Receivable { id: Some(receivable_id), transaction_id: Some(transaction_id), ..receivable.clone() },
        ))
    }

    /// Posts `transaction` and records `payable` atomically. Mirrors
    /// `post_transaction_with_receivable`.
    pub fn post_transaction_with_payable(&self, transaction: &Transaction, payable: &Payable) -> AppResult<(Transaction, Payable)> {
        self.validate_for_posting(transaction)?;
        payable.validate()?;
        let (transaction_id, payable_id) = self.db.with_transaction(|tx| {
            let transaction_id = Self::insert_transaction_and_lines(tx, transaction)?;
            tx.execute(
                "INSERT INTO payables (company_id, vendor_id, reference, bill_date, due_date, amount, amount_paid, transaction_id)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    payable.company_id,
                    payable.vendor_id,
                    payable.reference,
                    payable.bill_date,
                    payable.due_date,
                    payable.amount.to_string(),
                    payable.amount_paid.to_string(),
                    transaction_id,
                ],
            )?;
            Ok((transaction_id, tx.last_insert_rowid()))
        })?;
        Ok((
            Transaction { id: Some(transaction_id), status: TransactionStatus::Posted, ..transaction.clone() },
            Payable { id: Some(payable_id), transaction_id: Some(transaction_id), ..payable.clone() },
        ))
    }

    fn validate_for_posting(&self, transaction: &Transaction) -> AppResult<()> {
        let period = self
            .fiscal_period_for_date(transaction.company_id, transaction.transaction_date)?
            .ok_or_else(|| {
                AppError::Validation(format!(
                    "no fiscal period covers {}",
                    transaction.transaction_date
                ))
            })?;
        transaction_engine::validate_for_posting(transaction, &period)
    }

    /// The shared insert path: transaction header, every line, the
    /// reversal link on the original (if any), and the audit log entry --
    /// all within the caller's open `tx`. Returns the new transaction's id.
    fn insert_transaction_and_lines(tx: &rusqlite::Transaction, transaction: &Transaction) -> AppResult<i64> {
        tx.execute(
            "INSERT INTO transactions
             (company_id, transaction_type, transaction_date, description, status,
              reverses_transaction_id, reversed_by_transaction_id, posted_by_user_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                transaction.company_id,
                transaction.transaction_type.as_str(),
                transaction.transaction_date,
                transaction.description,
                TransactionStatus::Posted.as_str(),
                transaction.reverses_transaction_id,
                transaction.reversed_by_transaction_id,
                transaction.posted_by_user_id,
            ],
        )?;
        let transaction_id = tx.last_insert_rowid();

        for (line_number, line) in transaction.lines.iter().enumerate() {
            tx.execute(
                "INSERT INTO transaction_lines (transaction_id, account_id, debit, credit, description, line_number)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    transaction_id,
                    line.account_id,
                    line.debit.to_string(),
                    line.credit.to_string(),
                    line.description,
                    line_number as i64,
                ],
            )?;
        }

        if let Some(original_id) = transaction.reverses_transaction_id {
            tx.execute(
                "UPDATE transactions SET reversed_by_transaction_id = ?1, status = ?2 WHERE id = ?3",
                params![transaction_id, TransactionStatus::Reversed.as_str(), original_id],
            )?;
        }

        let mut entry = AuditLog::new(format!("accounting.transaction.{}", transaction.transaction_type.as_str()));
        entry.user_id = transaction.posted_by_user_id;
        entry.entity_type = Some("transaction".into());
        entry.entity_id = Some(transaction_id.to_string());
        audit_logs::record(tx, &entry)?;

        Ok(transaction_id)
    }

    pub fn get_transaction(&self, id: i64) -> AppResult<Transaction> {
        let (mut header, lines) = self.db.with_connection(|conn| {
            let header = conn
                .query_row(
                    "SELECT id, company_id, transaction_type, transaction_date, description, status,
                            reverses_transaction_id, reversed_by_transaction_id, posted_by_user_id
                     FROM transactions WHERE id = ?1",
                    params![id],
                    map_transaction,
                )
                .optional()?
                .ok_or_else(|| AppError::NotFound(format!("transaction {id} not found")))?;

            let mut stmt = conn.prepare(
                "SELECT id, transaction_id, account_id, debit, credit, description
                 FROM transaction_lines WHERE transaction_id = ?1 ORDER BY line_number",
            )?;
            let lines = stmt
                .query_map(params![id], map_transaction_line)?
                .collect::<Result<Vec<_>, _>>()?;

            Ok((header, lines))
        })?;
        header.lines = lines;
        Ok(header)
    }

    pub fn list_transactions_for_company(&self, company_id: i64) -> AppResult<Vec<Transaction>> {
        let ids: Vec<i64> = self.db.with_connection(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id FROM transactions WHERE company_id = ?1 ORDER BY transaction_date, id",
            )?;
            let rows = stmt.query_map(params![company_id], |row| row.get(0))?;
            rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
        })?;
        ids.into_iter().map(|id| self.get_transaction(id)).collect()
    }

    /// Builds and posts the reversal of an already-posted transaction.
    /// The original is *not* edited beyond its status/reversal link
    /// (Architecture Rules, Section 3: posted entries are immutable).
    pub fn reverse_transaction(&self, original_id: i64, reversal_date: chrono::NaiveDate) -> AppResult<Transaction> {
        let original = self.get_transaction(original_id)?;
        let reversal = original.build_reversal(reversal_date)?;
        self.post_transaction(&reversal)
    }

    // -- general ledger and trial balance (derived, not stored) --------

    /// The chronological posting history for one account, with a running
    /// balance after each line. `opening_balance` is 0 until
    /// Development Order step 8 (opening balances) supplies a real one.
    pub fn general_ledger_for_account(
        &self,
        account_id: i64,
        opening_balance: Decimal,
        up_to: chrono::NaiveDate,
    ) -> AppResult<Vec<GeneralLedgerEntry>> {
        let account = self.get_account(account_id)?;

        let postings: Vec<(Posting, Option<String>)> = self.db.with_connection(|conn| {
            let mut stmt = conn.prepare(
                "SELECT t.id, t.transaction_date, tl.debit, tl.credit, tl.description
                 FROM transaction_lines tl
                 JOIN transactions t ON t.id = tl.transaction_id
                 WHERE tl.account_id = ?1 AND t.status != 'draft' AND t.transaction_date <= ?2
                 ORDER BY t.transaction_date, t.id, tl.line_number",
            )?;
            let rows = stmt.query_map(params![account_id, up_to], |row| {
                let debit_text: String = row.get(2)?;
                let credit_text: String = row.get(3)?;
                Ok((
                    Posting {
                        transaction_id: row.get(0)?,
                        transaction_date: row.get(1)?,
                        debit: debit_text.parse().unwrap_or(Decimal::ZERO),
                        credit: credit_text.parse().unwrap_or(Decimal::ZERO),
                    },
                    row.get::<_, Option<String>>(4)?,
                ))
            })?;
            rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
        })?;

        let bare_postings: Vec<Posting> = postings.iter().map(|(p, _)| *p).collect();
        let balances = general_ledger::running_balances(account.normal_balance(), opening_balance, &bare_postings);

        Ok(postings
            .into_iter()
            .zip(balances)
            .map(|((posting, description), running_balance)| GeneralLedgerEntry {
                transaction_id: posting.transaction_id,
                transaction_date: posting.transaction_date,
                description,
                debit: posting.debit,
                credit: posting.credit,
                running_balance,
            })
            .collect())
    }

    /// Every account's net balance as of `as_of`, shown on whichever side
    /// it actually falls on. Derived entirely from posted transaction
    /// lines -- nothing here is a stored total.
    pub fn trial_balance_as_of(&self, company_id: i64, as_of: chrono::NaiveDate) -> AppResult<TrialBalance> {
        let accounts = self.list_accounts_for_company(company_id)?;

        let net_movements: std::collections::HashMap<i64, Decimal> = self.db.with_connection(|conn| {
            let mut stmt = conn.prepare(
                "SELECT tl.account_id, tl.debit, tl.credit
                 FROM transaction_lines tl
                 JOIN transactions t ON t.id = tl.transaction_id
                 WHERE t.company_id = ?1 AND t.status != 'draft' AND t.transaction_date <= ?2",
            )?;
            let rows = stmt.query_map(params![company_id, as_of], |row| {
                let debit_text: String = row.get(1)?;
                let credit_text: String = row.get(2)?;
                let debit: Decimal = debit_text.parse().unwrap_or(Decimal::ZERO);
                let credit: Decimal = credit_text.parse().unwrap_or(Decimal::ZERO);
                Ok((row.get::<_, i64>(0)?, debit - credit))
            })?;

            let mut totals = std::collections::HashMap::new();
            for row in rows {
                let (account_id, net) = row?;
                *totals.entry(account_id).or_insert(Decimal::ZERO) += net;
            }
            Ok(totals)
        })?;

        let lines = accounts
            .into_iter()
            .filter_map(|account| {
                let net = *net_movements.get(&account.id.unwrap())?;
                Some(TrialBalanceLine::from_net_movement(account.id.unwrap(), account.code, account.name, net))
            })
            .collect();

        Ok(TrialBalance { as_of, lines })
    }

    // -- period closing --------------------------------------------------

    /// Revenue and expense account balances as of a date, shaped for
    /// `period_closing::build_closing_transaction`.
    pub fn revenue_and_expense_balances_as_of(&self, company_id: i64, as_of: chrono::NaiveDate) -> AppResult<Vec<ClosingBalance>> {
        let accounts = self.list_accounts_for_company(company_id)?;
        let trial_balance = self.trial_balance_as_of(company_id, as_of)?;

        Ok(trial_balance
            .lines
            .iter()
            .filter_map(|line| {
                let account = accounts.iter().find(|a| a.id == Some(line.account_id))?;
                if !matches!(account.account_type, AccountType::Revenue | AccountType::Expense) {
                    return None;
                }
                Some(ClosingBalance {
                    account_id: line.account_id,
                    account_type: account.account_type,
                    net_balance: line.debit - line.credit,
                })
            })
            .collect())
    }

    /// Enforces chronological closing order (`period_closing::can_close`)
    /// and, if that passes, transitions the period to Closed. Does *not*
    /// generate a closing (revenue/expense-zeroing) entry -- that's a
    /// separate, opt-in step the caller takes with
    /// `revenue_and_expense_balances_as_of` +
    /// `period_closing::build_closing_transaction` + `post_transaction`,
    /// typically only at a fiscal year boundary.
    pub fn close_fiscal_period(&self, period_id: i64) -> AppResult<FiscalPeriod> {
        let period = self.get_fiscal_period(period_id)?;
        let other_periods: Vec<FiscalPeriod> = self
            .list_fiscal_periods_for_company(period.company_id)?
            .into_iter()
            .filter(|p| p.id != Some(period_id))
            .collect();
        period_closing::can_close(&period, &other_periods)?;
        self.transition_fiscal_period_status(period_id, FiscalPeriodStatus::Closed)
    }

    // -- receivables ------------------------------------------------------

    pub fn create_receivable(&self, receivable: &Receivable) -> AppResult<Receivable> {
        receivable.validate()?;
        self.db.with_connection(|conn| {
            conn.execute(
                "INSERT INTO receivables (company_id, customer_id, reference, invoice_date, due_date, amount, amount_paid, transaction_id)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    receivable.company_id,
                    receivable.customer_id,
                    receivable.reference,
                    receivable.invoice_date,
                    receivable.due_date,
                    receivable.amount.to_string(),
                    receivable.amount_paid.to_string(),
                    receivable.transaction_id,
                ],
            )?;
            Ok(Receivable {
                id: Some(conn.last_insert_rowid()),
                ..receivable.clone()
            })
        })
    }

    pub fn get_receivable(&self, id: i64) -> AppResult<Receivable> {
        self.db.with_connection(|conn| {
            conn.query_row(
                "SELECT id, company_id, customer_id, reference, invoice_date, due_date, amount, amount_paid, transaction_id
                 FROM receivables WHERE id = ?1",
                params![id],
                map_receivable,
            )
            .optional()?
            .ok_or_else(|| AppError::NotFound(format!("receivable {id} not found")))
        })
    }

    pub fn list_open_receivables_for_company(&self, company_id: i64) -> AppResult<Vec<Receivable>> {
        let all = self.db.with_connection(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, company_id, customer_id, reference, invoice_date, due_date, amount, amount_paid, transaction_id
                 FROM receivables WHERE company_id = ?1 ORDER BY due_date",
            )?;
            let rows = stmt.query_map(params![company_id], map_receivable)?;
            rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
        })?;
        Ok(all.into_iter().filter(|r| !r.is_settled()).collect())
    }

    /// Applies `(receivable_id, amount)` allocations -- typically from
    /// `accounts_receivable::collections::allocate_collection` -- as one
    /// atomic update.
    pub fn apply_receivable_payments(&self, allocations: &[(i64, Decimal)]) -> AppResult<()> {
        self.db.with_transaction(|tx| {
            for (id, amount) in allocations {
                tx.execute(
                    "UPDATE receivables SET amount_paid = amount_paid + ?1 WHERE id = ?2",
                    params![amount.to_string(), id],
                )?;
            }
            Ok(())
        })
    }

    // -- payables -----------------------------------------------------

    pub fn create_payable(&self, payable: &Payable) -> AppResult<Payable> {
        payable.validate()?;
        self.db.with_connection(|conn| {
            conn.execute(
                "INSERT INTO payables (company_id, vendor_id, reference, bill_date, due_date, amount, amount_paid, transaction_id)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    payable.company_id,
                    payable.vendor_id,
                    payable.reference,
                    payable.bill_date,
                    payable.due_date,
                    payable.amount.to_string(),
                    payable.amount_paid.to_string(),
                    payable.transaction_id,
                ],
            )?;
            Ok(Payable {
                id: Some(conn.last_insert_rowid()),
                ..payable.clone()
            })
        })
    }

    pub fn get_payable(&self, id: i64) -> AppResult<Payable> {
        self.db.with_connection(|conn| {
            conn.query_row(
                "SELECT id, company_id, vendor_id, reference, bill_date, due_date, amount, amount_paid, transaction_id
                 FROM payables WHERE id = ?1",
                params![id],
                map_payable,
            )
            .optional()?
            .ok_or_else(|| AppError::NotFound(format!("payable {id} not found")))
        })
    }

    pub fn list_open_payables_for_company(&self, company_id: i64) -> AppResult<Vec<Payable>> {
        let all = self.db.with_connection(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, company_id, vendor_id, reference, bill_date, due_date, amount, amount_paid, transaction_id
                 FROM payables WHERE company_id = ?1 ORDER BY due_date",
            )?;
            let rows = stmt.query_map(params![company_id], map_payable)?;
            rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
        })?;
        Ok(all.into_iter().filter(|p| !p.is_settled()).collect())
    }

    /// Applies `(payable_id, amount)` allocations -- typically from
    /// `accounts_payable::payments::allocate_payment` -- as one atomic
    /// update.
    pub fn apply_payable_payments(&self, allocations: &[(i64, Decimal)]) -> AppResult<()> {
        self.db.with_transaction(|tx| {
            for (id, amount) in allocations {
                tx.execute(
                    "UPDATE payables SET amount_paid = amount_paid + ?1 WHERE id = ?2",
                    params![amount.to_string(), id],
                )?;
            }
            Ok(())
        })
    }
}

fn map_account(row: &Row) -> rusqlite::Result<Account> {
    let account_type_text: String = row.get(4)?;
    let account_type = AccountType::parse(&account_type_text).ok_or_else(|| {
        rusqlite::Error::FromSqlConversionFailure(4, rusqlite::types::Type::Text, "invalid account_type".into())
    })?;
    Ok(Account {
        id: Some(row.get(0)?),
        company_id: row.get(1)?,
        code: row.get(2)?,
        name: row.get(3)?,
        account_type,
        parent_id: row.get(5)?,
        is_active: row.get(6)?,
        reference_code: row.get(7)?,
    })
}

fn map_fiscal_period(row: &Row) -> rusqlite::Result<FiscalPeriod> {
    let status_text: String = row.get(5)?;
    let status = FiscalPeriodStatus::parse(&status_text).ok_or_else(|| {
        rusqlite::Error::FromSqlConversionFailure(5, rusqlite::types::Type::Text, "invalid fiscal period status".into())
    })?;
    Ok(FiscalPeriod {
        id: Some(row.get(0)?),
        company_id: row.get(1)?,
        name: row.get(2)?,
        start_date: row.get(3)?,
        end_date: row.get(4)?,
        status,
    })
}

fn map_transaction(row: &Row) -> rusqlite::Result<Transaction> {
    let transaction_type_text: String = row.get(2)?;
    let transaction_type = TransactionType::parse(&transaction_type_text).ok_or_else(|| {
        rusqlite::Error::FromSqlConversionFailure(2, rusqlite::types::Type::Text, "invalid transaction_type".into())
    })?;
    let status_text: String = row.get(5)?;
    let status = TransactionStatus::parse(&status_text).ok_or_else(|| {
        rusqlite::Error::FromSqlConversionFailure(5, rusqlite::types::Type::Text, "invalid transaction status".into())
    })?;
    Ok(Transaction {
        id: Some(row.get(0)?),
        company_id: row.get(1)?,
        transaction_type,
        transaction_date: row.get(3)?,
        description: row.get(4)?,
        status,
        lines: Vec::new(),
        reverses_transaction_id: row.get(6)?,
        reversed_by_transaction_id: row.get(7)?,
        posted_by_user_id: row.get(8)?,
    })
}

fn map_receivable(row: &Row) -> rusqlite::Result<Receivable> {
    let amount_text: String = row.get(6)?;
    let amount_paid_text: String = row.get(7)?;
    Ok(Receivable {
        id: Some(row.get(0)?),
        company_id: row.get(1)?,
        customer_id: row.get(2)?,
        reference: row.get(3)?,
        invoice_date: row.get(4)?,
        due_date: row.get(5)?,
        amount: amount_text.parse().unwrap_or(Decimal::ZERO),
        amount_paid: amount_paid_text.parse().unwrap_or(Decimal::ZERO),
        transaction_id: row.get(8)?,
    })
}

fn map_payable(row: &Row) -> rusqlite::Result<Payable> {
    let amount_text: String = row.get(6)?;
    let amount_paid_text: String = row.get(7)?;
    Ok(Payable {
        id: Some(row.get(0)?),
        company_id: row.get(1)?,
        vendor_id: row.get(2)?,
        reference: row.get(3)?,
        bill_date: row.get(4)?,
        due_date: row.get(5)?,
        amount: amount_text.parse().unwrap_or(Decimal::ZERO),
        amount_paid: amount_paid_text.parse().unwrap_or(Decimal::ZERO),
        transaction_id: row.get(8)?,
    })
}

fn map_transaction_line(row: &Row) -> rusqlite::Result<TransactionLine> {
    let debit_text: String = row.get(3)?;
    let credit_text: String = row.get(4)?;
    let parse_decimal = |field: usize, text: &str| {
        text.parse().map_err(|_| {
            rusqlite::Error::FromSqlConversionFailure(field, rusqlite::types::Type::Text, "invalid decimal".into())
        })
    };
    Ok(TransactionLine {
        id: Some(row.get(0)?),
        transaction_id: Some(row.get(1)?),
        account_id: row.get(2)?,
        debit: parse_decimal(3, &debit_text)?,
        credit: parse_decimal(4, &credit_text)?,
        description: row.get(5)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::organization::companies::Company;
    use crate::backend::repositories::organization_repository::{self, OrganizationRepository};

    fn repo_with_company() -> (AccountingRepository, i64) {
        let (db, company_id) = db_with_company();
        (AccountingRepository::new(db), company_id)
    }

    fn db_with_company() -> (Database, i64) {
        let db = Database::open_in_memory().unwrap();
        db.run_migrations(organization_repository::MIGRATIONS).unwrap();
        db.run_migrations(crate::backend::repositories::party_repository::MIGRATIONS).unwrap();
        db.run_migrations(MIGRATIONS).unwrap();
        db.run_migrations(audit_logs::MIGRATIONS).unwrap();

        let org_repo = OrganizationRepository::new(db.clone());
        let company = org_repo.create_company(&Company::new("Acme Trading Co", "USD")).unwrap();

        (db, company.id.unwrap())
    }

    fn date(y: i32, m: u32, d: u32) -> chrono::NaiveDate {
        chrono::NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    #[test]
    fn creates_and_fetches_an_account_by_code() {
        let (repo, company_id) = repo_with_company();
        repo.create_account(&Account::new(company_id, "1000", "Cash", AccountType::Asset)).unwrap();

        let found = repo
            .get_account_by_code(company_id, "1000")
            .unwrap()
            .expect("account should exist");
        assert_eq!(found.name, "Cash");
        assert_eq!(found.account_type, AccountType::Asset);
    }

    #[test]
    fn account_codes_are_unique_per_company() {
        let (repo, company_id) = repo_with_company();
        repo.create_account(&Account::new(company_id, "1000", "Cash", AccountType::Asset)).unwrap();
        let result = repo.create_account(&Account::new(company_id, "1000", "Duplicate", AccountType::Asset));
        assert!(matches!(result, Err(AppError::Database(_))));
    }

    #[test]
    fn update_account_persists_changed_fields() {
        let (repo, company_id) = repo_with_company();
        let created = repo.create_account(&Account::new(company_id, "1000", "Cash", AccountType::Asset)).unwrap();

        repo.update_account(&Account { name: "Petty Cash".into(), ..created.clone() }).unwrap();

        let fetched = repo.get_account(created.id.unwrap()).unwrap();
        assert_eq!(fetched.name, "Petty Cash");
    }

    #[test]
    fn delete_account_removes_an_unreferenced_account() {
        let (repo, company_id) = repo_with_company();
        let created = repo.create_account(&Account::new(company_id, "1000", "Cash", AccountType::Asset)).unwrap();

        repo.delete_account(created.id.unwrap()).unwrap();

        assert!(repo.get_account(created.id.unwrap()).is_err());
    }

    #[test]
    fn delete_account_is_rejected_with_a_clear_message_when_still_referenced() {
        let (repo, company_id) = repo_with_company();
        let cash = repo.create_account(&Account::new(company_id, "1000", "Cash", AccountType::Asset)).unwrap();
        let capital = repo.create_account(&Account::new(company_id, "3000", "Owner's Capital", AccountType::Equity)).unwrap();
        repo.create_fiscal_period(&FiscalPeriod::new(company_id, "Open Period", date(2026, 1, 1), date(2030, 12, 31))).unwrap();
        repo.post_transaction(
            &Transaction::new(company_id, TransactionType::OpeningBalance, date(2026, 1, 1))
                .with_line(TransactionLine::debit(cash.id.unwrap(), rust_decimal_macros::dec!(100)))
                .with_line(TransactionLine::credit(capital.id.unwrap(), rust_decimal_macros::dec!(100))),
        )
        .unwrap();

        let result = repo.delete_account(cash.id.unwrap());

        assert!(matches!(result, Err(AppError::Validation(_))));
        assert!(result.unwrap_err().to_string().contains("still referenced"));
        // Confirms this is a real rejection, not a silent no-op that
        // happens to also leave the row in place.
        assert!(repo.get_account(cash.id.unwrap()).is_ok());
    }

    #[test]
    fn parent_child_accounts_round_trip() {
        let (repo, company_id) = repo_with_company();
        let parent = repo
            .create_account(&Account::new(company_id, "1000", "Current Assets", AccountType::Asset))
            .unwrap();

        let mut child = Account::new(company_id, "1010", "Cash", AccountType::Asset);
        child.parent_id = parent.id;
        let child = repo.create_account(&child).unwrap();

        let fetched = repo.get_account(child.id.unwrap()).unwrap();
        assert_eq!(fetched.parent_id, parent.id);
    }

    #[test]
    fn fiscal_period_for_date_finds_the_containing_period() {
        let (repo, company_id) = repo_with_company();
        repo.create_fiscal_period(&FiscalPeriod::new(company_id, "January 2026", date(2026, 1, 1), date(2026, 1, 31)))
            .unwrap();
        repo.create_fiscal_period(&FiscalPeriod::new(company_id, "February 2026", date(2026, 2, 1), date(2026, 2, 28)))
            .unwrap();

        let found = repo
            .fiscal_period_for_date(company_id, date(2026, 2, 15))
            .unwrap()
            .expect("February should contain this date");
        assert_eq!(found.name, "February 2026");

        assert!(repo.fiscal_period_for_date(company_id, date(2026, 3, 1)).unwrap().is_none());
    }

    #[test]
    fn valid_status_transition_persists() {
        let (repo, company_id) = repo_with_company();
        let period = repo
            .create_fiscal_period(&FiscalPeriod::new(company_id, "January 2026", date(2026, 1, 1), date(2026, 1, 31)))
            .unwrap();

        let closed = repo
            .transition_fiscal_period_status(period.id.unwrap(), FiscalPeriodStatus::Closed)
            .unwrap();
        assert_eq!(closed.status, FiscalPeriodStatus::Closed);

        let refetched = repo.get_fiscal_period(period.id.unwrap()).unwrap();
        assert_eq!(refetched.status, FiscalPeriodStatus::Closed);
    }

    #[test]
    fn invalid_status_transition_is_rejected_and_not_persisted() {
        let (repo, company_id) = repo_with_company();
        let period = repo
            .create_fiscal_period(&FiscalPeriod::new(company_id, "January 2026", date(2026, 1, 1), date(2026, 1, 31)))
            .unwrap();

        let result = repo.transition_fiscal_period_status(period.id.unwrap(), FiscalPeriodStatus::Locked);
        assert!(result.is_err());

        let refetched = repo.get_fiscal_period(period.id.unwrap()).unwrap();
        assert_eq!(refetched.status, FiscalPeriodStatus::Open);
    }

    fn repo_with_company_and_accounts() -> (AccountingRepository, i64, i64, i64) {
        let (repo, company_id) = repo_with_company();
        repo.create_fiscal_period(&FiscalPeriod::new(company_id, "January 2026", date(2026, 1, 1), date(2026, 1, 31)))
            .unwrap();
        let cash = repo
            .create_account(&Account::new(company_id, "1000", "Cash", AccountType::Asset))
            .unwrap();
        let capital = repo
            .create_account(&Account::new(company_id, "3000", "Owner's Capital", AccountType::Equity))
            .unwrap();
        (repo, company_id, cash.id.unwrap(), capital.id.unwrap())
    }

    #[test]
    fn posting_a_balanced_transaction_persists_header_lines_and_audit_entry() {
        let (repo, company_id, cash_id, capital_id) = repo_with_company_and_accounts();
        let transaction = Transaction::new(company_id, TransactionType::ManualJournal, date(2026, 1, 15))
            .with_line(TransactionLine::debit(cash_id, rust_decimal_macros::dec!(5000)))
            .with_line(TransactionLine::credit(capital_id, rust_decimal_macros::dec!(5000)));

        let posted = repo.post_transaction(&transaction).unwrap();
        assert!(posted.id.is_some());
        assert_eq!(posted.status, TransactionStatus::Posted);

        let fetched = repo.get_transaction(posted.id.unwrap()).unwrap();
        assert_eq!(fetched.lines.len(), 2);
        assert!(fetched.is_balanced());

        let audit = audit_logs::list_recent(&repo.db, 10).unwrap();
        assert_eq!(audit.len(), 1);
        assert_eq!(audit[0].entity_id.as_deref(), Some(posted.id.unwrap().to_string().as_str()));
    }

    #[test]
    fn posting_an_unbalanced_transaction_is_rejected_and_nothing_is_written() {
        let (repo, company_id, cash_id, capital_id) = repo_with_company_and_accounts();
        let transaction = Transaction::new(company_id, TransactionType::ManualJournal, date(2026, 1, 15))
            .with_line(TransactionLine::debit(cash_id, rust_decimal_macros::dec!(5000)))
            .with_line(TransactionLine::credit(capital_id, rust_decimal_macros::dec!(4000)));

        let result = repo.post_transaction(&transaction);
        assert!(result.is_err());
        assert!(repo.list_transactions_for_company(company_id).unwrap().is_empty());
    }

    #[test]
    fn posting_outside_the_fiscal_period_range_is_rejected() {
        let (repo, company_id, cash_id, capital_id) = repo_with_company_and_accounts();
        let transaction = Transaction::new(company_id, TransactionType::ManualJournal, date(2026, 3, 1))
            .with_line(TransactionLine::debit(cash_id, rust_decimal_macros::dec!(100)))
            .with_line(TransactionLine::credit(capital_id, rust_decimal_macros::dec!(100)));

        assert!(repo.post_transaction(&transaction).is_err());
    }

    #[test]
    fn posting_into_a_closed_period_is_rejected() {
        let (repo, company_id, cash_id, capital_id) = repo_with_company_and_accounts();
        let periods = repo.list_fiscal_periods_for_company(company_id).unwrap();
        repo.transition_fiscal_period_status(periods[0].id.unwrap(), FiscalPeriodStatus::Closed)
            .unwrap();

        let transaction = Transaction::new(company_id, TransactionType::ManualJournal, date(2026, 1, 15))
            .with_line(TransactionLine::debit(cash_id, rust_decimal_macros::dec!(100)))
            .with_line(TransactionLine::credit(capital_id, rust_decimal_macros::dec!(100)));

        assert!(repo.post_transaction(&transaction).is_err());
    }

    #[test]
    fn reversing_a_posted_transaction_balances_it_out_and_marks_the_original_reversed() {
        let (repo, company_id, cash_id, capital_id) = repo_with_company_and_accounts();
        let transaction = Transaction::new(company_id, TransactionType::ManualJournal, date(2026, 1, 15))
            .with_line(TransactionLine::debit(cash_id, rust_decimal_macros::dec!(5000)))
            .with_line(TransactionLine::credit(capital_id, rust_decimal_macros::dec!(5000)));
        let posted = repo.post_transaction(&transaction).unwrap();

        let reversal = repo.reverse_transaction(posted.id.unwrap(), date(2026, 1, 20)).unwrap();
        assert_eq!(reversal.transaction_type, TransactionType::Reversal);
        assert_eq!(reversal.reverses_transaction_id, posted.id);

        let original = repo.get_transaction(posted.id.unwrap()).unwrap();
        assert_eq!(original.status, TransactionStatus::Reversed);
        assert_eq!(original.reversed_by_transaction_id, reversal.id);
        // the original's financial content (accounts/amounts) is untouched,
        // even though it was re-fetched from the DB (and so now carries line ids)
        let content = |lines: &[TransactionLine]| -> Vec<(i64, rust_decimal::Decimal, rust_decimal::Decimal)> {
            lines.iter().map(|l| (l.account_id, l.debit, l.credit)).collect()
        };
        assert_eq!(content(&original.lines), content(&posted.lines));

        // net effect across both postings is zero on every account
        let net_cash: rust_decimal::Decimal = original
            .lines
            .iter()
            .chain(reversal.lines.iter())
            .filter(|l| l.account_id == cash_id)
            .map(|l| l.debit - l.credit)
            .sum();
        assert_eq!(net_cash, rust_decimal::Decimal::ZERO);
    }

    #[test]
    fn general_ledger_for_account_shows_running_balance_in_order() {
        let (repo, company_id, cash_id, capital_id) = repo_with_company_and_accounts();
        repo.post_transaction(
            &Transaction::new(company_id, TransactionType::ManualJournal, date(2026, 1, 5))
                .with_line(TransactionLine::debit(cash_id, rust_decimal_macros::dec!(1000)))
                .with_line(TransactionLine::credit(capital_id, rust_decimal_macros::dec!(1000))),
        )
        .unwrap();
        repo.post_transaction(
            &Transaction::new(company_id, TransactionType::ManualJournal, date(2026, 1, 20))
                .with_line(TransactionLine::credit(cash_id, rust_decimal_macros::dec!(300)))
                .with_line(TransactionLine::debit(capital_id, rust_decimal_macros::dec!(300))),
        )
        .unwrap();

        let ledger = repo
            .general_ledger_for_account(cash_id, rust_decimal::Decimal::ZERO, date(2026, 1, 31))
            .unwrap();
        assert_eq!(ledger.len(), 2);
        assert_eq!(ledger[0].running_balance, rust_decimal_macros::dec!(1000));
        assert_eq!(ledger[1].running_balance, rust_decimal_macros::dec!(700));
    }

    #[test]
    fn general_ledger_for_account_respects_the_opening_balance_and_cutoff_date() {
        let (repo, company_id, cash_id, capital_id) = repo_with_company_and_accounts();
        repo.post_transaction(
            &Transaction::new(company_id, TransactionType::ManualJournal, date(2026, 1, 5))
                .with_line(TransactionLine::debit(cash_id, rust_decimal_macros::dec!(100)))
                .with_line(TransactionLine::credit(capital_id, rust_decimal_macros::dec!(100))),
        )
        .unwrap();

        let ledger = repo
            .general_ledger_for_account(cash_id, rust_decimal_macros::dec!(500), date(2026, 1, 4))
            .unwrap();
        assert!(ledger.is_empty());

        let ledger = repo
            .general_ledger_for_account(cash_id, rust_decimal_macros::dec!(500), date(2026, 1, 31))
            .unwrap();
        assert_eq!(ledger.len(), 1);
        assert_eq!(ledger[0].running_balance, rust_decimal_macros::dec!(600));
    }

    #[test]
    fn trial_balance_as_of_always_balances_and_reflects_only_postings_up_to_the_date() {
        let (repo, company_id, cash_id, capital_id) = repo_with_company_and_accounts();
        repo.post_transaction(
            &Transaction::new(company_id, TransactionType::ManualJournal, date(2026, 1, 5))
                .with_line(TransactionLine::debit(cash_id, rust_decimal_macros::dec!(1000)))
                .with_line(TransactionLine::credit(capital_id, rust_decimal_macros::dec!(1000))),
        )
        .unwrap();
        repo.post_transaction(
            &Transaction::new(company_id, TransactionType::ManualJournal, date(2026, 1, 20))
                .with_line(TransactionLine::credit(cash_id, rust_decimal_macros::dec!(300)))
                .with_line(TransactionLine::debit(capital_id, rust_decimal_macros::dec!(300))),
        )
        .unwrap();

        let mid_month = repo.trial_balance_as_of(company_id, date(2026, 1, 10)).unwrap();
        assert!(mid_month.is_balanced());
        let cash_line = mid_month.lines.iter().find(|l| l.account_id == cash_id).unwrap();
        assert_eq!(cash_line.debit, rust_decimal_macros::dec!(1000));

        let end_month = repo.trial_balance_as_of(company_id, date(2026, 1, 31)).unwrap();
        assert!(end_month.is_balanced());
        let cash_line = end_month.lines.iter().find(|l| l.account_id == cash_id).unwrap();
        assert_eq!(cash_line.debit, rust_decimal_macros::dec!(700));
    }

    #[test]
    fn posting_opening_balances_seeds_the_trial_balance() {
        let (repo, company_id, cash_id, capital_id) = repo_with_company_and_accounts();
        let opening = crate::backend::accounting::opening_balances::OpeningBalances::new(company_id, date(2026, 1, 1))
            .with_balance(cash_id, rust_decimal_macros::dec!(10000))
            .with_balance(capital_id, rust_decimal_macros::dec!(-10000))
            .into_transaction();

        repo.post_transaction(&opening).unwrap();

        let trial_balance = repo.trial_balance_as_of(company_id, date(2026, 1, 1)).unwrap();
        assert!(trial_balance.is_balanced());
        let cash_line = trial_balance.lines.iter().find(|l| l.account_id == cash_id).unwrap();
        assert_eq!(cash_line.debit, rust_decimal_macros::dec!(10000));
    }

    #[test]
    fn closing_a_period_out_of_order_is_rejected() {
        let (repo, company_id) = repo_with_company();
        repo.create_fiscal_period(&FiscalPeriod::new(company_id, "December 2025", date(2025, 12, 1), date(2025, 12, 31)))
            .unwrap();
        let january = repo
            .create_fiscal_period(&FiscalPeriod::new(company_id, "January 2026", date(2026, 1, 1), date(2026, 1, 31)))
            .unwrap();

        let result = repo.close_fiscal_period(january.id.unwrap());
        assert!(result.is_err());
        assert_eq!(repo.get_fiscal_period(january.id.unwrap()).unwrap().status, FiscalPeriodStatus::Open);
    }

    #[test]
    fn closing_the_earliest_open_period_succeeds() {
        let (repo, company_id) = repo_with_company();
        let december = repo
            .create_fiscal_period(&FiscalPeriod::new(company_id, "December 2025", date(2025, 12, 1), date(2025, 12, 31)))
            .unwrap();
        repo.create_fiscal_period(&FiscalPeriod::new(company_id, "January 2026", date(2026, 1, 1), date(2026, 1, 31)))
            .unwrap();

        let closed = repo.close_fiscal_period(december.id.unwrap()).unwrap();
        assert_eq!(closed.status, FiscalPeriodStatus::Closed);
    }

    #[test]
    fn revenue_and_expense_balances_as_of_excludes_balance_sheet_accounts() {
        let (repo, company_id, cash_id, capital_id) = repo_with_company_and_accounts();
        let sales = repo
            .create_account(&Account::new(company_id, "4000", "Sales Revenue", AccountType::Revenue))
            .unwrap();

        repo.post_transaction(
            &Transaction::new(company_id, TransactionType::ManualJournal, date(2026, 1, 15))
                .with_line(TransactionLine::debit(cash_id, rust_decimal_macros::dec!(2000)))
                .with_line(TransactionLine::credit(sales.id.unwrap(), rust_decimal_macros::dec!(2000))),
        )
        .unwrap();
        // A balance-sheet-only entry that should not show up as revenue/expense.
        repo.post_transaction(
            &Transaction::new(company_id, TransactionType::ManualJournal, date(2026, 1, 16))
                .with_line(TransactionLine::debit(cash_id, rust_decimal_macros::dec!(500)))
                .with_line(TransactionLine::credit(capital_id, rust_decimal_macros::dec!(500))),
        )
        .unwrap();

        let balances = repo.revenue_and_expense_balances_as_of(company_id, date(2026, 1, 31)).unwrap();
        assert_eq!(balances.len(), 1);
        assert_eq!(balances[0].account_id, sales.id.unwrap());
        assert_eq!(balances[0].net_balance, rust_decimal_macros::dec!(-2000));
    }

    #[test]
    fn end_to_end_closing_entry_posts_and_zeroes_revenue_and_expense() {
        let (repo, company_id, cash_id, _capital_id) = repo_with_company_and_accounts();
        let sales = repo
            .create_account(&Account::new(company_id, "4000", "Sales Revenue", AccountType::Revenue))
            .unwrap();
        let rent = repo
            .create_account(&Account::new(company_id, "5000", "Rent Expense", AccountType::Expense))
            .unwrap();
        let retained_earnings = repo
            .create_account(&Account::new(company_id, "3900", "Retained Earnings", AccountType::Equity))
            .unwrap();

        repo.post_transaction(
            &Transaction::new(company_id, TransactionType::ManualJournal, date(2026, 1, 10))
                .with_line(TransactionLine::debit(cash_id, rust_decimal_macros::dec!(5000)))
                .with_line(TransactionLine::credit(sales.id.unwrap(), rust_decimal_macros::dec!(5000))),
        )
        .unwrap();
        repo.post_transaction(
            &Transaction::new(company_id, TransactionType::ManualJournal, date(2026, 1, 12))
                .with_line(TransactionLine::debit(rent.id.unwrap(), rust_decimal_macros::dec!(1200)))
                .with_line(TransactionLine::credit(cash_id, rust_decimal_macros::dec!(1200))),
        )
        .unwrap();

        let balances = repo.revenue_and_expense_balances_as_of(company_id, date(2026, 1, 31)).unwrap();
        let closing_transaction =
            period_closing::build_closing_transaction(company_id, date(2026, 1, 31), retained_earnings.id.unwrap(), &balances)
                .expect("there should be something to close");
        repo.post_transaction(&closing_transaction).unwrap();

        let post_closing_balances = repo.revenue_and_expense_balances_as_of(company_id, date(2026, 1, 31)).unwrap();
        assert!(post_closing_balances.iter().all(|b| b.net_balance == Decimal::ZERO));

        let trial_balance = repo.trial_balance_as_of(company_id, date(2026, 1, 31)).unwrap();
        assert!(trial_balance.is_balanced());
        let retained_earnings_line = trial_balance.lines.iter().find(|l| l.account_id == retained_earnings.id.unwrap()).unwrap();
        assert_eq!(retained_earnings_line.credit, rust_decimal_macros::dec!(3800));
    }

    #[test]
    fn receivable_lifecycle_create_list_open_then_settle() {
        let (db, company_id) = db_with_company();
        let repo = AccountingRepository::new(db.clone());
        let party_repo = crate::backend::repositories::party_repository::PartyRepository::new(db);
        let customer = party_repo
            .create_customer(&crate::backend::parties::customers::Customer::new(company_id, "Acme Retail"))
            .unwrap();

        let receivable = repo
            .create_receivable(&Receivable::new(
                company_id,
                customer.id.unwrap(),
                date(2026, 1, 1),
                date(2026, 1, 31),
                rust_decimal_macros::dec!(1000),
            ))
            .unwrap();

        assert_eq!(repo.list_open_receivables_for_company(company_id).unwrap().len(), 1);

        repo.apply_receivable_payments(&[(receivable.id.unwrap(), rust_decimal_macros::dec!(1000))])
            .unwrap();
        assert!(repo.list_open_receivables_for_company(company_id).unwrap().is_empty());
    }

    #[test]
    fn payable_lifecycle_create_list_open_then_partially_settle() {
        let (db, company_id) = db_with_company();
        let repo = AccountingRepository::new(db.clone());
        let party_repo = crate::backend::repositories::party_repository::PartyRepository::new(db);
        let vendor = party_repo
            .create_vendor(&crate::backend::parties::vendors::Vendor::new(company_id, "Acme Supplies Ltd"))
            .unwrap();

        let payable = repo
            .create_payable(&Payable::new(
                company_id,
                vendor.id.unwrap(),
                date(2026, 1, 1),
                date(2026, 1, 31),
                rust_decimal_macros::dec!(1000),
            ))
            .unwrap();

        repo.apply_payable_payments(&[(payable.id.unwrap(), rust_decimal_macros::dec!(400))])
            .unwrap();

        let open = repo.list_open_payables_for_company(company_id).unwrap();
        assert_eq!(open.len(), 1);
        assert_eq!(open[0].balance(), rust_decimal_macros::dec!(600));
    }

    #[test]
    fn creating_a_receivable_for_a_nonexistent_customer_is_rejected() {
        let (db, company_id) = db_with_company();
        let repo = AccountingRepository::new(db);
        let result = repo.create_receivable(&Receivable::new(
            company_id,
            999,
            date(2026, 1, 1),
            date(2026, 1, 31),
            rust_decimal_macros::dec!(1000),
        ));
        assert!(matches!(result, Err(AppError::Database(_))));
    }
}
