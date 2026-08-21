//! Persistence for tax rates and tax transactions.

use chrono::NaiveDate;
use rust_decimal::Decimal;
use rusqlite::{params, OptionalExtension, Row};

use crate::backend::core::errors::{AppError, AppResult};
use crate::backend::core::validation::Validate;
use crate::backend::repositories::accounting_repository::AccountingRepository;
use crate::backend::tax::tax_rates::{TaxDirection, TaxRate};
use crate::backend::tax::tax_reports::{self, TaxSummary};
use crate::backend::tax::tax_transactions::TaxTransaction;
use crate::backend::transactions::transaction::Transaction;
use crate::database::database::{Database, Migration};

pub const MIGRATIONS: &[Migration] = &[Migration {
    version: 1400,
    name: "tax_schema",
    sql: "
        CREATE TABLE tax_rates (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            company_id      INTEGER NOT NULL REFERENCES companies(id),
            name            TEXT NOT NULL,
            rate_percent    TEXT NOT NULL,
            direction       TEXT NOT NULL,
            gl_account_id   INTEGER NOT NULL REFERENCES accounts(id),
            is_active       INTEGER NOT NULL DEFAULT 1
        );

        CREATE TABLE tax_transactions (
            id                  INTEGER PRIMARY KEY AUTOINCREMENT,
            company_id          INTEGER NOT NULL REFERENCES companies(id),
            tax_rate_id         INTEGER NOT NULL REFERENCES tax_rates(id),
            source_reference    TEXT,
            tax_date            TEXT NOT NULL,
            taxable_amount      TEXT NOT NULL,
            tax_amount          TEXT NOT NULL,
            direction           TEXT NOT NULL,
            transaction_id      INTEGER REFERENCES transactions(id)
        );
    ",
}];

#[derive(Clone)]
pub struct TaxRepository {
    db: Database,
    accounting: AccountingRepository,
}

impl TaxRepository {
    pub fn new(db: Database) -> Self {
        let accounting = AccountingRepository::new(db.clone());
        Self { db, accounting }
    }

    // -- tax rates --------------------------------------------------------

    pub fn create_tax_rate(&self, rate: &TaxRate) -> AppResult<TaxRate> {
        rate.validate()?;
        self.db.with_connection(|conn| {
            conn.execute(
                "INSERT INTO tax_rates (company_id, name, rate_percent, direction, gl_account_id, is_active)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![rate.company_id, rate.name, rate.rate_percent.to_string(), rate.direction.as_str(), rate.gl_account_id, rate.is_active],
            )?;
            Ok(TaxRate { id: Some(conn.last_insert_rowid()), ..rate.clone() })
        })
    }

    pub fn get_tax_rate(&self, id: i64) -> AppResult<TaxRate> {
        self.db.with_connection(|conn| {
            conn.query_row(
                "SELECT id, company_id, name, rate_percent, direction, gl_account_id, is_active FROM tax_rates WHERE id = ?1",
                params![id],
                map_tax_rate,
            )
            .optional()?
            .ok_or_else(|| AppError::NotFound(format!("tax rate {id} not found")))
        })
    }

    pub fn list_tax_rates_for_company(&self, company_id: i64) -> AppResult<Vec<TaxRate>> {
        self.db.with_connection(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, company_id, name, rate_percent, direction, gl_account_id, is_active FROM tax_rates WHERE company_id = ?1 ORDER BY name",
            )?;
            let rows = stmt.query_map(params![company_id], map_tax_rate)?;
            rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
        })
    }

    // -- tax transactions ---------------------------------------------

    /// Posts the tax's GL transaction and stores it linked, atomically.
    pub fn post_tax(&self, tax: &TaxTransaction, offset_account_id: i64) -> AppResult<(TaxTransaction, Transaction)> {
        tax.validate()?;
        let rate = self.get_tax_rate(tax.tax_rate_id)?;
        let transaction = tax.build_transaction(rate.gl_account_id, offset_account_id);
        let posted = self.accounting.post_transaction(&transaction)?;

        let tax_id = self.db.with_connection(|conn| {
            conn.execute(
                "INSERT INTO tax_transactions (company_id, tax_rate_id, source_reference, tax_date, taxable_amount, tax_amount, direction, transaction_id)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    tax.company_id,
                    tax.tax_rate_id,
                    tax.source_reference,
                    tax.tax_date,
                    tax.taxable_amount.to_string(),
                    tax.tax_amount.to_string(),
                    tax.direction.as_str(),
                    posted.id,
                ],
            )?;
            Ok(conn.last_insert_rowid())
        })?;

        Ok((TaxTransaction { id: Some(tax_id), transaction_id: posted.id, ..tax.clone() }, posted))
    }

    pub fn list_tax_transactions_for_company(&self, company_id: i64) -> AppResult<Vec<TaxTransaction>> {
        self.db.with_connection(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, company_id, tax_rate_id, source_reference, tax_date, taxable_amount, tax_amount, direction, transaction_id
                 FROM tax_transactions WHERE company_id = ?1 ORDER BY tax_date",
            )?;
            let rows = stmt.query_map(params![company_id], map_tax_transaction)?;
            rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
        })
    }

    pub fn tax_summary(&self, company_id: i64, period_start: NaiveDate, period_end: NaiveDate) -> AppResult<TaxSummary> {
        let transactions = self.list_tax_transactions_for_company(company_id)?;
        Ok(tax_reports::summarize(&transactions, period_start, period_end))
    }
}

fn map_tax_rate(row: &Row) -> rusqlite::Result<TaxRate> {
    let rate_text: String = row.get(3)?;
    let direction_text: String = row.get(4)?;
    let direction = TaxDirection::parse(&direction_text)
        .ok_or_else(|| rusqlite::Error::FromSqlConversionFailure(4, rusqlite::types::Type::Text, "invalid tax direction".into()))?;
    Ok(TaxRate {
        id: Some(row.get(0)?),
        company_id: row.get(1)?,
        name: row.get(2)?,
        rate_percent: rate_text.parse().unwrap_or(Decimal::ZERO),
        direction,
        gl_account_id: row.get(5)?,
        is_active: row.get(6)?,
    })
}

fn map_tax_transaction(row: &Row) -> rusqlite::Result<TaxTransaction> {
    let taxable_text: String = row.get(5)?;
    let tax_text: String = row.get(6)?;
    let direction_text: String = row.get(7)?;
    let direction = TaxDirection::parse(&direction_text)
        .ok_or_else(|| rusqlite::Error::FromSqlConversionFailure(7, rusqlite::types::Type::Text, "invalid tax direction".into()))?;
    Ok(TaxTransaction {
        id: Some(row.get(0)?),
        company_id: row.get(1)?,
        tax_rate_id: row.get(2)?,
        source_reference: row.get(3)?,
        tax_date: row.get(4)?,
        taxable_amount: taxable_text.parse().unwrap_or(Decimal::ZERO),
        tax_amount: tax_text.parse().unwrap_or(Decimal::ZERO),
        direction,
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
    use rust_decimal_macros::dec;

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    struct TestFixture {
        tax: TaxRepository,
        company_id: i64,
        output_rate_id: i64,
        ar_account_id: i64,
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
        let tax_payable_account = acc_repo.create_account(&Account::new(company_id, "2100", "VAT Payable", AccountType::Liability)).unwrap();
        let ar_account = acc_repo.create_account(&Account::new(company_id, "1100", "Accounts Receivable", AccountType::Asset)).unwrap();

        let tax = TaxRepository::new(db);
        let output_rate = tax
            .create_tax_rate(&TaxRate::new(company_id, "Standard VAT 15%", dec!(15), TaxDirection::Output, tax_payable_account.id.unwrap()))
            .unwrap();

        TestFixture {
            tax,
            company_id,
            output_rate_id: output_rate.id.unwrap(),
            ar_account_id: ar_account.id.unwrap(),
        }
    }

    #[test]
    fn posting_a_tax_transaction_links_it_to_the_gl_transaction() {
        let f = fixture();
        let tax_transaction = TaxTransaction::new(f.company_id, f.output_rate_id, date(2026, 1, 10), dec!(1000), dec!(150), TaxDirection::Output);
        let (recorded, transaction) = f.tax.post_tax(&tax_transaction, f.ar_account_id).unwrap();
        assert!(transaction.is_balanced());
        assert_eq!(recorded.transaction_id, transaction.id);
    }

    #[test]
    fn tax_summary_aggregates_posted_transactions_for_the_period() {
        let f = fixture();
        f.tax
            .post_tax(&TaxTransaction::new(f.company_id, f.output_rate_id, date(2026, 1, 5), dec!(1000), dec!(150), TaxDirection::Output), f.ar_account_id)
            .unwrap();
        f.tax
            .post_tax(&TaxTransaction::new(f.company_id, f.output_rate_id, date(2026, 1, 20), dec!(500), dec!(75), TaxDirection::Output), f.ar_account_id)
            .unwrap();

        let summary = f.tax.tax_summary(f.company_id, date(2026, 1, 1), date(2026, 1, 31)).unwrap();
        assert_eq!(summary.total_output_tax, dec!(225));
        assert_eq!(summary.net_tax_payable(), dec!(225));
    }
}
