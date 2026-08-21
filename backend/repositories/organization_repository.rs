//! Persistence for the organization hierarchy (companies, branches,
//! departments) and the currency reference data that companies and
//! exchange rates depend on. Bundled into one repository because none of
//! these five entities is large enough to warrant its own file, and they
//! share a migration set that must apply together.

use chrono::NaiveDate;
use rusqlite::{params, OptionalExtension, Row};

use crate::backend::core::errors::{AppError, AppResult};
use crate::backend::core::validation::Validate;
use crate::backend::currency::currencies::Currency;
use crate::backend::currency::exchange_rates::ExchangeRate;
use crate::backend::organization::branches::Branch;
use crate::backend::organization::companies::Company;
use crate::backend::organization::departments::Department;
use crate::database::database::{Database, Migration};

// Migration versions are `<development order step> * 100 + <sequence
// within that step>`, so each step has headroom for more than one
// migration without renumbering anything that shipped earlier.
pub const MIGRATIONS: &[Migration] = &[Migration {
    version: 300,
    name: "organization_and_currency_schema",
    sql: "
        CREATE TABLE companies (
            id                  INTEGER PRIMARY KEY AUTOINCREMENT,
            name                TEXT NOT NULL,
            legal_name          TEXT,
            base_currency_code  TEXT NOT NULL,
            created_at          TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
        );

        CREATE TABLE branches (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            company_id  INTEGER NOT NULL REFERENCES companies(id),
            name        TEXT NOT NULL,
            code        TEXT NOT NULL,
            address     TEXT,
            created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
            UNIQUE (company_id, code)
        );

        CREATE TABLE departments (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            company_id  INTEGER NOT NULL REFERENCES companies(id),
            branch_id   INTEGER REFERENCES branches(id),
            name        TEXT NOT NULL,
            code        TEXT NOT NULL,
            created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
            UNIQUE (company_id, code)
        );

        CREATE TABLE currencies (
            code            TEXT PRIMARY KEY,
            name            TEXT NOT NULL,
            symbol          TEXT NOT NULL,
            decimal_places  INTEGER NOT NULL
        );

        CREATE TABLE exchange_rates (
            id             INTEGER PRIMARY KEY AUTOINCREMENT,
            from_currency  TEXT NOT NULL REFERENCES currencies(code),
            to_currency    TEXT NOT NULL REFERENCES currencies(code),
            rate           TEXT NOT NULL,
            as_of          TEXT NOT NULL,
            UNIQUE (from_currency, to_currency, as_of)
        );
    ",
}];

#[derive(Clone)]
pub struct OrganizationRepository {
    db: Database,
}

impl OrganizationRepository {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    // -- companies --------------------------------------------------

    pub fn create_company(&self, company: &Company) -> AppResult<Company> {
        company.validate()?;
        self.db.with_connection(|conn| {
            conn.execute(
                "INSERT INTO companies (name, legal_name, base_currency_code) VALUES (?1, ?2, ?3)",
                params![company.name, company.legal_name, company.base_currency_code],
            )?;
            Ok(Company {
                id: Some(conn.last_insert_rowid()),
                ..company.clone()
            })
        })
    }

    pub fn get_company(&self, id: i64) -> AppResult<Company> {
        self.db.with_connection(|conn| {
            conn.query_row(
                "SELECT id, name, legal_name, base_currency_code FROM companies WHERE id = ?1",
                params![id],
                map_company,
            )
            .optional()?
            .ok_or_else(|| AppError::NotFound(format!("company {id} not found")))
        })
    }

    pub fn list_companies(&self) -> AppResult<Vec<Company>> {
        self.db.with_connection(|conn| {
            let mut stmt = conn.prepare("SELECT id, name, legal_name, base_currency_code FROM companies ORDER BY name")?;
            let rows = stmt.query_map([], map_company)?;
            rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
        })
    }

    pub fn update_company(&self, company: &Company) -> AppResult<()> {
        company.validate()?;
        let id = company
            .id
            .ok_or_else(|| AppError::Validation("cannot update a company without an id".into()))?;
        self.db.with_connection(|conn| {
            let affected = conn.execute(
                "UPDATE companies SET name = ?1, legal_name = ?2, base_currency_code = ?3 WHERE id = ?4",
                params![company.name, company.legal_name, company.base_currency_code, id],
            )?;
            if affected == 0 {
                return Err(AppError::NotFound(format!("company {id} not found")));
            }
            Ok(())
        })
    }

    // -- branches -----------------------------------------------------

    pub fn create_branch(&self, branch: &Branch) -> AppResult<Branch> {
        branch.validate()?;
        self.db.with_connection(|conn| {
            conn.execute(
                "INSERT INTO branches (company_id, name, code, address) VALUES (?1, ?2, ?3, ?4)",
                params![branch.company_id, branch.name, branch.code, branch.address],
            )?;
            Ok(Branch {
                id: Some(conn.last_insert_rowid()),
                ..branch.clone()
            })
        })
    }

    pub fn get_branch(&self, id: i64) -> AppResult<Branch> {
        self.db.with_connection(|conn| {
            conn.query_row(
                "SELECT id, company_id, name, code, address FROM branches WHERE id = ?1",
                params![id],
                map_branch,
            )
            .optional()?
            .ok_or_else(|| AppError::NotFound(format!("branch {id} not found")))
        })
    }

    pub fn list_branches_for_company(&self, company_id: i64) -> AppResult<Vec<Branch>> {
        self.db.with_connection(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, company_id, name, code, address FROM branches WHERE company_id = ?1 ORDER BY name",
            )?;
            let rows = stmt.query_map(params![company_id], map_branch)?;
            rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
        })
    }

    pub fn update_branch(&self, branch: &Branch) -> AppResult<()> {
        branch.validate()?;
        let id = branch
            .id
            .ok_or_else(|| AppError::Validation("cannot update a branch without an id".into()))?;
        self.db.with_connection(|conn| {
            let affected = conn.execute(
                "UPDATE branches SET company_id = ?1, name = ?2, code = ?3, address = ?4 WHERE id = ?5",
                params![branch.company_id, branch.name, branch.code, branch.address, id],
            )?;
            if affected == 0 {
                return Err(AppError::NotFound(format!("branch {id} not found")));
            }
            Ok(())
        })
    }

    // -- departments --------------------------------------------------

    pub fn create_department(&self, department: &Department) -> AppResult<Department> {
        department.validate()?;
        self.db.with_connection(|conn| {
            conn.execute(
                "INSERT INTO departments (company_id, branch_id, name, code) VALUES (?1, ?2, ?3, ?4)",
                params![department.company_id, department.branch_id, department.name, department.code],
            )?;
            Ok(Department {
                id: Some(conn.last_insert_rowid()),
                ..department.clone()
            })
        })
    }

    pub fn get_department(&self, id: i64) -> AppResult<Department> {
        self.db.with_connection(|conn| {
            conn.query_row(
                "SELECT id, company_id, branch_id, name, code FROM departments WHERE id = ?1",
                params![id],
                map_department,
            )
            .optional()?
            .ok_or_else(|| AppError::NotFound(format!("department {id} not found")))
        })
    }

    pub fn list_departments_for_company(&self, company_id: i64) -> AppResult<Vec<Department>> {
        self.db.with_connection(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, company_id, branch_id, name, code FROM departments WHERE company_id = ?1 ORDER BY name",
            )?;
            let rows = stmt.query_map(params![company_id], map_department)?;
            rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
        })
    }

    pub fn update_department(&self, department: &Department) -> AppResult<()> {
        department.validate()?;
        let id = department
            .id
            .ok_or_else(|| AppError::Validation("cannot update a department without an id".into()))?;
        self.db.with_connection(|conn| {
            let affected = conn.execute(
                "UPDATE departments SET company_id = ?1, branch_id = ?2, name = ?3, code = ?4 WHERE id = ?5",
                params![department.company_id, department.branch_id, department.name, department.code, id],
            )?;
            if affected == 0 {
                return Err(AppError::NotFound(format!("department {id} not found")));
            }
            Ok(())
        })
    }

    // -- currencies -----------------------------------------------------

    /// Currencies are keyed by their code, so creating one that already
    /// exists overwrites its display fields instead of erroring.
    pub fn upsert_currency(&self, currency: &Currency) -> AppResult<()> {
        currency.validate()?;
        self.db.with_connection(|conn| {
            conn.execute(
                "INSERT INTO currencies (code, name, symbol, decimal_places) VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT (code) DO UPDATE SET name = excluded.name, symbol = excluded.symbol, decimal_places = excluded.decimal_places",
                params![currency.code, currency.name, currency.symbol, currency.decimal_places],
            )?;
            Ok(())
        })
    }

    pub fn get_currency(&self, code: &str) -> AppResult<Currency> {
        let code = code.to_ascii_uppercase();
        self.db.with_connection(|conn| {
            conn.query_row(
                "SELECT code, name, symbol, decimal_places FROM currencies WHERE code = ?1",
                params![code],
                map_currency,
            )
            .optional()?
            .ok_or_else(|| AppError::NotFound(format!("currency {code} not found")))
        })
    }

    pub fn list_currencies(&self) -> AppResult<Vec<Currency>> {
        self.db.with_connection(|conn| {
            let mut stmt = conn.prepare("SELECT code, name, symbol, decimal_places FROM currencies ORDER BY code")?;
            let rows = stmt.query_map([], map_currency)?;
            rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
        })
    }

    // -- exchange rates -----------------------------------------------

    pub fn record_exchange_rate(&self, rate: &ExchangeRate) -> AppResult<ExchangeRate> {
        rate.validate()?;
        self.db.with_connection(|conn| {
            conn.execute(
                "INSERT INTO exchange_rates (from_currency, to_currency, rate, as_of) VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT (from_currency, to_currency, as_of) DO UPDATE SET rate = excluded.rate",
                params![rate.from_currency, rate.to_currency, rate.rate.to_string(), rate.as_of],
            )?;
            let id: i64 = conn.query_row(
                "SELECT id FROM exchange_rates WHERE from_currency = ?1 AND to_currency = ?2 AND as_of = ?3",
                params![rate.from_currency, rate.to_currency, rate.as_of],
                |row| row.get(0),
            )?;
            Ok(ExchangeRate {
                id: Some(id),
                ..rate.clone()
            })
        })
    }

    /// The most recent rate on or before `as_of` for the given pair, if any.
    pub fn latest_exchange_rate(
        &self,
        from_currency: &str,
        to_currency: &str,
        as_of: NaiveDate,
    ) -> AppResult<Option<ExchangeRate>> {
        let from_currency = from_currency.to_ascii_uppercase();
        let to_currency = to_currency.to_ascii_uppercase();
        self.db.with_connection(|conn| {
            conn.query_row(
                "SELECT id, from_currency, to_currency, rate, as_of FROM exchange_rates
                 WHERE from_currency = ?1 AND to_currency = ?2 AND as_of <= ?3
                 ORDER BY as_of DESC LIMIT 1",
                params![from_currency, to_currency, as_of],
                map_exchange_rate,
            )
            .optional()
            .map_err(AppError::from)
        })
    }
}

fn map_company(row: &Row) -> rusqlite::Result<Company> {
    Ok(Company {
        id: Some(row.get(0)?),
        name: row.get(1)?,
        legal_name: row.get(2)?,
        base_currency_code: row.get(3)?,
    })
}

fn map_branch(row: &Row) -> rusqlite::Result<Branch> {
    Ok(Branch {
        id: Some(row.get(0)?),
        company_id: row.get(1)?,
        name: row.get(2)?,
        code: row.get(3)?,
        address: row.get(4)?,
    })
}

fn map_department(row: &Row) -> rusqlite::Result<Department> {
    Ok(Department {
        id: Some(row.get(0)?),
        company_id: row.get(1)?,
        branch_id: row.get(2)?,
        name: row.get(3)?,
        code: row.get(4)?,
    })
}

fn map_currency(row: &Row) -> rusqlite::Result<Currency> {
    let decimal_places: i64 = row.get(3)?;
    Ok(Currency {
        code: row.get(0)?,
        name: row.get(1)?,
        symbol: row.get(2)?,
        decimal_places: decimal_places as u32,
    })
}

fn map_exchange_rate(row: &Row) -> rusqlite::Result<ExchangeRate> {
    let rate_text: String = row.get(3)?;
    Ok(ExchangeRate {
        id: Some(row.get(0)?),
        from_currency: row.get(1)?,
        to_currency: row.get(2)?,
        rate: rate_text.parse().map_err(|_| {
            rusqlite::Error::FromSqlConversionFailure(3, rusqlite::types::Type::Text, "invalid decimal".into())
        })?,
        as_of: row.get(4)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn repo_with_schema() -> OrganizationRepository {
        let db = Database::open_in_memory().unwrap();
        db.run_migrations(MIGRATIONS).unwrap();
        OrganizationRepository::new(db)
    }

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    #[test]
    fn creates_and_fetches_a_company() {
        let repo = repo_with_schema();
        let created = repo.create_company(&Company::new("Acme Trading Co", "USD")).unwrap();
        assert!(created.id.is_some());

        let fetched = repo.get_company(created.id.unwrap()).unwrap();
        assert_eq!(fetched.name, "Acme Trading Co");
        assert_eq!(fetched.base_currency_code, "USD");
    }

    #[test]
    fn rejects_invalid_company_before_hitting_the_database() {
        let repo = repo_with_schema();
        let result = repo.create_company(&Company::new("", "USD"));
        assert!(matches!(result, Err(AppError::Validation(_))));
    }

    #[test]
    fn getting_a_missing_company_returns_not_found() {
        let repo = repo_with_schema();
        assert!(matches!(repo.get_company(999), Err(AppError::NotFound(_))));
    }

    #[test]
    fn branch_and_department_round_trip_under_a_company() {
        let repo = repo_with_schema();
        let company = repo.create_company(&Company::new("Acme Trading Co", "USD")).unwrap();
        let company_id = company.id.unwrap();

        let branch = repo
            .create_branch(&Branch::new(company_id, "Main Branch", "MAIN"))
            .unwrap();
        assert_eq!(repo.list_branches_for_company(company_id).unwrap().len(), 1);

        let mut department = Department::new(company_id, "Finance", "FIN");
        department.branch_id = branch.id;
        let department = repo.create_department(&department).unwrap();
        assert_eq!(department.branch_id, branch.id);
        assert_eq!(repo.list_departments_for_company(company_id).unwrap().len(), 1);
    }

    #[test]
    fn upserting_a_currency_twice_updates_rather_than_duplicates() {
        let repo = repo_with_schema();
        repo.upsert_currency(&Currency::new("USD", "US Dollar", "$", 2)).unwrap();
        repo.upsert_currency(&Currency::new("USD", "United States Dollar", "US$", 2))
            .unwrap();

        let currencies = repo.list_currencies().unwrap();
        assert_eq!(currencies.len(), 1);
        assert_eq!(currencies[0].name, "United States Dollar");
    }

    #[test]
    fn latest_exchange_rate_picks_the_most_recent_on_or_before_the_date() {
        let repo = repo_with_schema();
        repo.upsert_currency(&Currency::new("USD", "US Dollar", "$", 2)).unwrap();
        repo.upsert_currency(&Currency::new("PKR", "Pakistani Rupee", "Rs", 2)).unwrap();

        repo.record_exchange_rate(&ExchangeRate::new("USD", "PKR", dec!(278.0), date(2026, 1, 1)))
            .unwrap();
        repo.record_exchange_rate(&ExchangeRate::new("USD", "PKR", dec!(279.5), date(2026, 2, 1)))
            .unwrap();

        let rate = repo
            .latest_exchange_rate("USD", "PKR", date(2026, 3, 1))
            .unwrap()
            .expect("a rate should exist on or before 2026-03-01");
        assert_eq!(rate.rate, dec!(279.5));

        let rate = repo
            .latest_exchange_rate("USD", "PKR", date(2026, 1, 15))
            .unwrap()
            .expect("a rate should exist on or before 2026-01-15");
        assert_eq!(rate.rate, dec!(278.0));

        assert!(repo
            .latest_exchange_rate("USD", "PKR", date(2025, 12, 31))
            .unwrap()
            .is_none());
    }
}
