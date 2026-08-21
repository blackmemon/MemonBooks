//! Persistence for expense categories, direct expenses, and employee
//! expense claims.

use rusqlite::{params, OptionalExtension, Row};

use crate::backend::core::errors::{AppError, AppResult};
use crate::backend::core::validation::Validate;
use crate::backend::expenses::expense_categories::ExpenseCategory;
use crate::backend::expenses::expense_claims::{ExpenseClaim, ExpenseClaimStatus};
use crate::backend::expenses::expenses::Expense;
use crate::backend::repositories::accounting_repository::AccountingRepository;
use crate::backend::transactions::transaction::Transaction;
use crate::database::database::{Database, Migration};

pub const MIGRATIONS: &[Migration] = &[Migration {
    version: 1300,
    name: "expenses_schema",
    sql: "
        CREATE TABLE expense_categories (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            company_id      INTEGER NOT NULL REFERENCES companies(id),
            name            TEXT NOT NULL,
            gl_account_id   INTEGER NOT NULL REFERENCES accounts(id)
        );

        CREATE TABLE expenses (
            id                  INTEGER PRIMARY KEY AUTOINCREMENT,
            company_id          INTEGER NOT NULL REFERENCES companies(id),
            category_id         INTEGER NOT NULL REFERENCES expense_categories(id),
            expense_date        TEXT NOT NULL,
            amount              TEXT NOT NULL,
            description         TEXT NOT NULL,
            payment_account_id  INTEGER NOT NULL REFERENCES accounts(id),
            transaction_id      INTEGER REFERENCES transactions(id)
        );

        CREATE TABLE expense_claims (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            company_id      INTEGER NOT NULL REFERENCES companies(id),
            claimant_name   TEXT NOT NULL,
            category_id     INTEGER NOT NULL REFERENCES expense_categories(id),
            claim_date      TEXT NOT NULL,
            amount          TEXT NOT NULL,
            description     TEXT NOT NULL,
            status          TEXT NOT NULL,
            transaction_id  INTEGER REFERENCES transactions(id)
        );
    ",
}];

#[derive(Clone)]
pub struct ExpenseRepository {
    db: Database,
    accounting: AccountingRepository,
}

impl ExpenseRepository {
    pub fn new(db: Database) -> Self {
        let accounting = AccountingRepository::new(db.clone());
        Self { db, accounting }
    }

    // -- categories -------------------------------------------------------

    pub fn create_category(&self, category: &ExpenseCategory) -> AppResult<ExpenseCategory> {
        category.validate()?;
        self.db.with_connection(|conn| {
            conn.execute(
                "INSERT INTO expense_categories (company_id, name, gl_account_id) VALUES (?1, ?2, ?3)",
                params![category.company_id, category.name, category.gl_account_id],
            )?;
            Ok(ExpenseCategory { id: Some(conn.last_insert_rowid()), ..category.clone() })
        })
    }

    pub fn get_category(&self, id: i64) -> AppResult<ExpenseCategory> {
        self.db.with_connection(|conn| {
            conn.query_row(
                "SELECT id, company_id, name, gl_account_id FROM expense_categories WHERE id = ?1",
                params![id],
                map_category,
            )
            .optional()?
            .ok_or_else(|| AppError::NotFound(format!("expense category {id} not found")))
        })
    }

    // -- direct expenses --------------------------------------------------

    /// Builds and posts the expense's transaction, then stores it
    /// already-linked -- there's no separate draft state for a direct
    /// expense (unlike an invoice/bill), so create and post happen together.
    pub fn record_expense(&self, expense: &Expense) -> AppResult<(Expense, Transaction)> {
        expense.validate()?;
        let category = self.get_category(expense.category_id)?;
        let transaction = expense.build_transaction(category.gl_account_id);
        let posted = self.accounting.post_transaction(&transaction)?;

        let expense_id = self.db.with_connection(|conn| {
            conn.execute(
                "INSERT INTO expenses (company_id, category_id, expense_date, amount, description, payment_account_id, transaction_id)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![expense.company_id, expense.category_id, expense.expense_date, expense.amount.to_string(), expense.description, expense.payment_account_id, posted.id],
            )?;
            Ok(conn.last_insert_rowid())
        })?;

        Ok((Expense { id: Some(expense_id), transaction_id: posted.id, ..expense.clone() }, posted))
    }

    // -- expense claims -----------------------------------------------

    pub fn create_claim(&self, claim: &ExpenseClaim) -> AppResult<ExpenseClaim> {
        claim.validate()?;
        self.db.with_connection(|conn| {
            conn.execute(
                "INSERT INTO expense_claims (company_id, claimant_name, category_id, claim_date, amount, description, status, transaction_id)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    claim.company_id,
                    claim.claimant_name,
                    claim.category_id,
                    claim.claim_date,
                    claim.amount.to_string(),
                    claim.description,
                    claim.status.as_str(),
                    claim.transaction_id,
                ],
            )?;
            Ok(ExpenseClaim { id: Some(conn.last_insert_rowid()), ..claim.clone() })
        })
    }

    pub fn get_claim(&self, id: i64) -> AppResult<ExpenseClaim> {
        self.db.with_connection(|conn| {
            conn.query_row(
                "SELECT id, company_id, claimant_name, category_id, claim_date, amount, description, status, transaction_id
                 FROM expense_claims WHERE id = ?1",
                params![id],
                map_claim,
            )
            .optional()?
            .ok_or_else(|| AppError::NotFound(format!("expense claim {id} not found")))
        })
    }

    fn transition_claim(&self, id: i64, to: ExpenseClaimStatus) -> AppResult<ExpenseClaim> {
        let claim = self.get_claim(id)?;
        if !claim.status.can_transition_to(to) {
            return Err(AppError::Validation(format!(
                "cannot transition expense claim from {} to {}",
                claim.status.as_str(),
                to.as_str()
            )));
        }
        self.db.with_connection(|conn| {
            conn.execute("UPDATE expense_claims SET status = ?1 WHERE id = ?2", params![to.as_str(), id])?;
            Ok(())
        })?;
        Ok(ExpenseClaim { status: to, ..claim })
    }

    pub fn approve_claim(&self, id: i64) -> AppResult<ExpenseClaim> {
        self.transition_claim(id, ExpenseClaimStatus::Approved)
    }

    pub fn reject_claim(&self, id: i64) -> AppResult<ExpenseClaim> {
        self.transition_claim(id, ExpenseClaimStatus::Rejected)
    }

    /// Posts the reimbursement transaction and marks the claim
    /// Reimbursed, atomically linked.
    pub fn reimburse_claim(&self, id: i64, payment_account_id: i64, reimbursement_date: chrono::NaiveDate) -> AppResult<(ExpenseClaim, Transaction)> {
        let claim = self.get_claim(id)?;
        if claim.status != ExpenseClaimStatus::Approved {
            return Err(AppError::Validation("only an approved claim can be reimbursed".into()));
        }
        let category = self.get_category(claim.category_id)?;
        let transaction = claim.build_reimbursement_transaction(category.gl_account_id, payment_account_id, reimbursement_date);
        let posted = self.accounting.post_transaction(&transaction)?;

        self.db.with_connection(|conn| {
            conn.execute(
                "UPDATE expense_claims SET status = ?1, transaction_id = ?2 WHERE id = ?3",
                params![ExpenseClaimStatus::Reimbursed.as_str(), posted.id, id],
            )?;
            Ok(())
        })?;

        Ok((ExpenseClaim { status: ExpenseClaimStatus::Reimbursed, transaction_id: posted.id, ..claim }, posted))
    }
}

fn map_category(row: &Row) -> rusqlite::Result<ExpenseCategory> {
    Ok(ExpenseCategory {
        id: Some(row.get(0)?),
        company_id: row.get(1)?,
        name: row.get(2)?,
        gl_account_id: row.get(3)?,
    })
}

fn map_claim(row: &Row) -> rusqlite::Result<ExpenseClaim> {
    let amount_text: String = row.get(5)?;
    let status_text: String = row.get(7)?;
    let status = ExpenseClaimStatus::parse(&status_text)
        .ok_or_else(|| rusqlite::Error::FromSqlConversionFailure(7, rusqlite::types::Type::Text, "invalid expense claim status".into()))?;
    Ok(ExpenseClaim {
        id: Some(row.get(0)?),
        company_id: row.get(1)?,
        claimant_name: row.get(2)?,
        category_id: row.get(3)?,
        claim_date: row.get(4)?,
        amount: amount_text.parse().unwrap_or(rust_decimal::Decimal::ZERO),
        description: row.get(6)?,
        status,
        transaction_id: row.get(8)?,
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
        expenses: ExpenseRepository,
        company_id: i64,
        category_id: i64,
        cash_account_id: i64,
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
        let expense_account = acc_repo.create_account(&Account::new(company_id, "6100", "Travel Expense", AccountType::Expense)).unwrap();
        let cash_account = acc_repo.create_account(&Account::new(company_id, "1000", "Cash", AccountType::Asset)).unwrap();

        let expenses = ExpenseRepository::new(db);
        let category = expenses.create_category(&ExpenseCategory::new(company_id, "Travel", expense_account.id.unwrap())).unwrap();

        TestFixture {
            expenses,
            company_id,
            category_id: category.id.unwrap(),
            cash_account_id: cash_account.id.unwrap(),
        }
    }

    #[test]
    fn recording_an_expense_posts_it_and_links_the_transaction() {
        let f = fixture();
        let expense = Expense::new(f.company_id, f.category_id, date(2026, 1, 10), dec!(45), "Taxi fare", f.cash_account_id);
        let (recorded, transaction) = f.expenses.record_expense(&expense).unwrap();
        assert!(transaction.is_balanced());
        assert_eq!(recorded.transaction_id, transaction.id);
    }

    #[test]
    fn expense_claim_lifecycle_submit_approve_reimburse() {
        let f = fixture();
        let claim = f
            .expenses
            .create_claim(&ExpenseClaim::new(f.company_id, "Jane Doe", f.category_id, date(2026, 1, 5), dec!(80), "Client dinner"))
            .unwrap();
        assert_eq!(claim.status, ExpenseClaimStatus::Submitted);

        let approved = f.expenses.approve_claim(claim.id.unwrap()).unwrap();
        assert_eq!(approved.status, ExpenseClaimStatus::Approved);

        let (reimbursed, transaction) = f.expenses.reimburse_claim(claim.id.unwrap(), f.cash_account_id, date(2026, 1, 10)).unwrap();
        assert_eq!(reimbursed.status, ExpenseClaimStatus::Reimbursed);
        assert!(transaction.is_balanced());
    }

    #[test]
    fn reimbursing_a_claim_that_was_never_approved_is_rejected() {
        let f = fixture();
        let claim = f
            .expenses
            .create_claim(&ExpenseClaim::new(f.company_id, "Jane Doe", f.category_id, date(2026, 1, 5), dec!(80), "Client dinner"))
            .unwrap();

        let result = f.expenses.reimburse_claim(claim.id.unwrap(), f.cash_account_id, date(2026, 1, 10));
        assert!(result.is_err());
    }
}
