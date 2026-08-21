//! Persistence for the accounting domain. Starts with the chart of
//! accounts and fiscal periods (Development Order step 5); journal
//! entries, the general ledger, and trial balance land in the same
//! repository as later steps build them, since they all share this one
//! migration set and company-scoped schema.

use rusqlite::{params, OptionalExtension, Row};

use crate::backend::accounting::accounting_rules::AccountType;
use crate::backend::accounting::chart_of_accounts::Account;
use crate::backend::accounting::fiscal_periods::{FiscalPeriod, FiscalPeriodStatus};
use crate::backend::core::errors::{AppError, AppResult};
use crate::backend::core::validation::Validate;
use crate::database::database::{Database, Migration};

pub const MIGRATIONS: &[Migration] = &[Migration {
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
}];

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
                "INSERT INTO accounts (company_id, code, name, account_type, parent_id, is_active)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    account.company_id,
                    account.code,
                    account.name,
                    account.account_type.as_str(),
                    account.parent_id,
                    account.is_active,
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
                "SELECT id, company_id, code, name, account_type, parent_id, is_active FROM accounts WHERE id = ?1",
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
                "SELECT id, company_id, code, name, account_type, parent_id, is_active
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
                "SELECT id, company_id, code, name, account_type, parent_id, is_active
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::organization::companies::Company;
    use crate::backend::repositories::organization_repository::{self, OrganizationRepository};

    fn repo_with_company() -> (AccountingRepository, i64) {
        let db = Database::open_in_memory().unwrap();
        db.run_migrations(organization_repository::MIGRATIONS).unwrap();
        db.run_migrations(MIGRATIONS).unwrap();

        let org_repo = OrganizationRepository::new(db.clone());
        let company = org_repo.create_company(&Company::new("Acme Trading Co", "USD")).unwrap();

        (AccountingRepository::new(db), company.id.unwrap())
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
}
