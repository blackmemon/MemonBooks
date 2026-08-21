//! Persistence for fixed asset categories, assets, depreciation, and
//! disposals.

use rust_decimal::Decimal;
use rusqlite::{params, OptionalExtension, Row};

use crate::backend::core::errors::{AppError, AppResult};
use crate::backend::core::validation::Validate;
use crate::backend::fixed_assets::asset_categories::AssetCategory;
use crate::backend::fixed_assets::assets::Asset;
use crate::backend::fixed_assets::depreciation::{self, DepreciationMethod};
use crate::backend::fixed_assets::disposals::AssetDisposal;
use crate::backend::fixed_assets::transfers::AssetTransfer;
use crate::backend::repositories::accounting_repository::AccountingRepository;
use crate::backend::transactions::transaction::Transaction;
use crate::backend::transactions::transaction_lines::TransactionLine;
use crate::backend::transactions::transaction_types::TransactionType;
use crate::database::database::{Database, Migration};

pub const MIGRATIONS: &[Migration] = &[Migration {
    version: 1301,
    name: "fixed_assets_schema",
    sql: "
        CREATE TABLE asset_categories (
            id                                      INTEGER PRIMARY KEY AUTOINCREMENT,
            company_id                              INTEGER NOT NULL REFERENCES companies(id),
            name                                    TEXT NOT NULL,
            asset_gl_account_id                     INTEGER NOT NULL REFERENCES accounts(id),
            accumulated_depreciation_gl_account_id  INTEGER NOT NULL REFERENCES accounts(id),
            depreciation_expense_gl_account_id      INTEGER NOT NULL REFERENCES accounts(id),
            default_useful_life_years               INTEGER NOT NULL,
            default_method                          TEXT NOT NULL,
            default_method_rate                     TEXT
        );

        CREATE TABLE assets (
            id                          INTEGER PRIMARY KEY AUTOINCREMENT,
            company_id                  INTEGER NOT NULL REFERENCES companies(id),
            category_id                 INTEGER NOT NULL REFERENCES asset_categories(id),
            name                        TEXT NOT NULL,
            acquisition_date            TEXT NOT NULL,
            acquisition_cost            TEXT NOT NULL,
            salvage_value               TEXT NOT NULL,
            useful_life_years           INTEGER NOT NULL,
            method                      TEXT NOT NULL,
            method_rate                 TEXT,
            accumulated_depreciation    TEXT NOT NULL DEFAULT '0',
            is_disposed                 INTEGER NOT NULL DEFAULT 0,
            transaction_id              INTEGER REFERENCES transactions(id)
        );

        CREATE TABLE asset_transfers (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            asset_id        INTEGER NOT NULL REFERENCES assets(id),
            transfer_date   TEXT NOT NULL,
            from_branch_id  INTEGER REFERENCES branches(id),
            to_branch_id    INTEGER REFERENCES branches(id),
            notes           TEXT
        );
    ",
}];

#[derive(Clone)]
pub struct FixedAssetRepository {
    db: Database,
    accounting: AccountingRepository,
}

impl FixedAssetRepository {
    pub fn new(db: Database) -> Self {
        let accounting = AccountingRepository::new(db.clone());
        Self { db, accounting }
    }

    // -- categories -------------------------------------------------------

    pub fn create_category(&self, category: &AssetCategory) -> AppResult<AssetCategory> {
        category.validate()?;
        let (method, rate) = encode_method(category.default_method);
        self.db.with_connection(|conn| {
            conn.execute(
                "INSERT INTO asset_categories
                 (company_id, name, asset_gl_account_id, accumulated_depreciation_gl_account_id, depreciation_expense_gl_account_id, default_useful_life_years, default_method, default_method_rate)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    category.company_id,
                    category.name,
                    category.asset_gl_account_id,
                    category.accumulated_depreciation_gl_account_id,
                    category.depreciation_expense_gl_account_id,
                    category.default_useful_life_years,
                    method,
                    rate,
                ],
            )?;
            Ok(AssetCategory { id: Some(conn.last_insert_rowid()), ..category.clone() })
        })
    }

    pub fn get_category(&self, id: i64) -> AppResult<AssetCategory> {
        self.db.with_connection(|conn| {
            conn.query_row(
                "SELECT id, company_id, name, asset_gl_account_id, accumulated_depreciation_gl_account_id, depreciation_expense_gl_account_id, default_useful_life_years, default_method, default_method_rate
                 FROM asset_categories WHERE id = ?1",
                params![id],
                map_category,
            )
            .optional()?
            .ok_or_else(|| AppError::NotFound(format!("asset category {id} not found")))
        })
    }

    // -- assets -----------------------------------------------------------

    /// Registers the asset and posts its acquisition (debit the asset
    /// account, credit `payment_account_id`) atomically.
    pub fn acquire_asset(&self, asset: &Asset, payment_account_id: i64) -> AppResult<(Asset, Transaction)> {
        asset.validate()?;
        let category = self.get_category(asset.category_id)?;

        let mut transaction = Transaction::new(asset.company_id, TransactionType::ManualJournal, asset.acquisition_date);
        transaction.description = Some(format!("Acquisition: {}", asset.name));
        transaction.lines.push(TransactionLine::debit(category.asset_gl_account_id, asset.acquisition_cost));
        transaction.lines.push(TransactionLine::credit(payment_account_id, asset.acquisition_cost));
        let posted = self.accounting.post_transaction(&transaction)?;

        let (method, rate) = encode_method(asset.method);
        let asset_id = self.db.with_connection(|conn| {
            conn.execute(
                "INSERT INTO assets
                 (company_id, category_id, name, acquisition_date, acquisition_cost, salvage_value, useful_life_years, method, method_rate, accumulated_depreciation, is_disposed, transaction_id)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                params![
                    asset.company_id,
                    asset.category_id,
                    asset.name,
                    asset.acquisition_date,
                    asset.acquisition_cost.to_string(),
                    asset.salvage_value.to_string(),
                    asset.useful_life_years,
                    method,
                    rate,
                    asset.accumulated_depreciation.to_string(),
                    asset.is_disposed,
                    posted.id,
                ],
            )?;
            Ok(conn.last_insert_rowid())
        })?;

        Ok((Asset { id: Some(asset_id), transaction_id: posted.id, ..asset.clone() }, posted))
    }

    pub fn get_asset(&self, id: i64) -> AppResult<Asset> {
        self.db.with_connection(|conn| {
            conn.query_row(
                "SELECT id, company_id, category_id, name, acquisition_date, acquisition_cost, salvage_value, useful_life_years, method, method_rate, accumulated_depreciation, is_disposed, transaction_id
                 FROM assets WHERE id = ?1",
                params![id],
                map_asset,
            )
            .optional()?
            .ok_or_else(|| AppError::NotFound(format!("asset {id} not found")))
        })
    }

    /// Computes this year's depreciation from the asset's current book
    /// value, posts the expense/accumulated-depreciation entry, and
    /// updates the asset's accumulated_depreciation -- atomically.
    pub fn record_depreciation(&self, asset_id: i64, depreciation_date: chrono::NaiveDate) -> AppResult<Option<Transaction>> {
        let asset = self.get_asset(asset_id)?;
        if asset.is_disposed {
            return Err(AppError::Validation("cannot depreciate a disposed asset".into()));
        }
        let category = self.get_category(asset.category_id)?;

        let amount = depreciation::annual_depreciation(asset.method, asset.acquisition_cost, asset.salvage_value, asset.useful_life_years, asset.book_value());
        if amount <= Decimal::ZERO {
            return Ok(None);
        }

        let transaction = depreciation::build_depreciation_transaction(
            asset.company_id,
            depreciation_date,
            category.depreciation_expense_gl_account_id,
            category.accumulated_depreciation_gl_account_id,
            amount,
            &asset.name,
        );
        let posted = self.accounting.post_transaction(&transaction)?;

        self.db.with_connection(|conn| {
            conn.execute(
                "UPDATE assets SET accumulated_depreciation = accumulated_depreciation + ?1 WHERE id = ?2",
                params![amount.to_string(), asset_id],
            )?;
            Ok(())
        })?;

        Ok(Some(posted))
    }

    /// Posts the disposal entry and marks the asset disposed, atomically.
    pub fn dispose_asset(&self, asset_id: i64, disposal_date: chrono::NaiveDate, proceeds: Decimal, proceeds_account_id: i64, gain_loss_account_id: i64) -> AppResult<Transaction> {
        let asset = self.get_asset(asset_id)?;
        if asset.is_disposed {
            return Err(AppError::Validation("asset has already been disposed".into()));
        }
        let category = self.get_category(asset.category_id)?;

        let disposal = AssetDisposal {
            company_id: asset.company_id,
            asset_name: asset.name.clone(),
            disposal_date,
            acquisition_cost: asset.acquisition_cost,
            accumulated_depreciation: asset.accumulated_depreciation,
            proceeds,
        };
        let transaction = disposal.build_transaction(category.asset_gl_account_id, category.accumulated_depreciation_gl_account_id, proceeds_account_id, gain_loss_account_id);
        let posted = self.accounting.post_transaction(&transaction)?;

        self.db.with_connection(|conn| {
            conn.execute("UPDATE assets SET is_disposed = 1 WHERE id = ?1", params![asset_id])?;
            Ok(())
        })?;

        Ok(posted)
    }

    // -- transfers (custody/location, no GL impact) --------------------

    pub fn create_transfer(&self, transfer: &AssetTransfer) -> AppResult<AssetTransfer> {
        transfer.validate()?;
        self.db.with_connection(|conn| {
            conn.execute(
                "INSERT INTO asset_transfers (asset_id, transfer_date, from_branch_id, to_branch_id, notes) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![transfer.asset_id, transfer.transfer_date, transfer.from_branch_id, transfer.to_branch_id, transfer.notes],
            )?;
            Ok(AssetTransfer { id: Some(conn.last_insert_rowid()), ..transfer.clone() })
        })
    }
}

fn encode_method(method: DepreciationMethod) -> (&'static str, Option<String>) {
    match method {
        DepreciationMethod::StraightLine => ("straight_line", None),
        DepreciationMethod::DecliningBalance { rate } => ("declining_balance", Some(rate.to_string())),
    }
}

fn decode_method(method: &str, rate: Option<String>) -> rusqlite::Result<DepreciationMethod> {
    match method {
        "straight_line" => Ok(DepreciationMethod::StraightLine),
        "declining_balance" => {
            let rate: Decimal = rate
                .and_then(|r| r.parse().ok())
                .ok_or_else(|| rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, "missing declining balance rate".into()))?;
            Ok(DepreciationMethod::DecliningBalance { rate })
        }
        _ => Err(rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, "invalid depreciation method".into())),
    }
}

fn map_category(row: &Row) -> rusqlite::Result<AssetCategory> {
    let method_text: String = row.get(7)?;
    let rate: Option<String> = row.get(8)?;
    Ok(AssetCategory {
        id: Some(row.get(0)?),
        company_id: row.get(1)?,
        name: row.get(2)?,
        asset_gl_account_id: row.get(3)?,
        accumulated_depreciation_gl_account_id: row.get(4)?,
        depreciation_expense_gl_account_id: row.get(5)?,
        default_useful_life_years: row.get(6)?,
        default_method: decode_method(&method_text, rate)?,
    })
}

fn map_asset(row: &Row) -> rusqlite::Result<Asset> {
    let cost_text: String = row.get(5)?;
    let salvage_text: String = row.get(6)?;
    let method_text: String = row.get(8)?;
    let rate: Option<String> = row.get(9)?;
    let accumulated_text: String = row.get(10)?;
    Ok(Asset {
        id: Some(row.get(0)?),
        company_id: row.get(1)?,
        category_id: row.get(2)?,
        name: row.get(3)?,
        acquisition_date: row.get(4)?,
        acquisition_cost: cost_text.parse().unwrap_or(Decimal::ZERO),
        salvage_value: salvage_text.parse().unwrap_or(Decimal::ZERO),
        useful_life_years: row.get(7)?,
        method: decode_method(&method_text, rate)?,
        accumulated_depreciation: accumulated_text.parse().unwrap_or(Decimal::ZERO),
        is_disposed: row.get(11)?,
        transaction_id: row.get(12)?,
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
        fixed_assets: FixedAssetRepository,
        company_id: i64,
        category_id: i64,
        cash_account_id: i64,
        gain_loss_account_id: i64,
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
            .create_fiscal_period(&FiscalPeriod::new(company_id, "Fiscal Year 2026", date(2026, 1, 1), date(2026, 12, 31)))
            .unwrap();
        let asset_account = acc_repo.create_account(&Account::new(company_id, "1500", "Computer Equipment", AccountType::Asset)).unwrap();
        let accumulated_depreciation_account = acc_repo.create_account(&Account::new(company_id, "1510", "Accumulated Depreciation", AccountType::Asset)).unwrap();
        let depreciation_expense_account = acc_repo.create_account(&Account::new(company_id, "6500", "Depreciation Expense", AccountType::Expense)).unwrap();
        let cash_account = acc_repo.create_account(&Account::new(company_id, "1000", "Cash", AccountType::Asset)).unwrap();
        let gain_loss_account = acc_repo.create_account(&Account::new(company_id, "7000", "Gain/Loss on Disposal", AccountType::Revenue)).unwrap();

        let fixed_assets = FixedAssetRepository::new(db);
        let category = fixed_assets
            .create_category(&AssetCategory::new(
                company_id,
                "Computer Equipment",
                asset_account.id.unwrap(),
                accumulated_depreciation_account.id.unwrap(),
                depreciation_expense_account.id.unwrap(),
                5,
            ))
            .unwrap();

        TestFixture {
            fixed_assets,
            company_id,
            category_id: category.id.unwrap(),
            cash_account_id: cash_account.id.unwrap(),
            gain_loss_account_id: gain_loss_account.id.unwrap(),
        }
    }

    #[test]
    fn acquiring_an_asset_posts_and_links_the_transaction() {
        let f = fixture();
        let asset = Asset::new(f.company_id, f.category_id, "Office Laptop", date(2026, 1, 1), dec!(2000), dec!(200), 5);
        let (acquired, transaction) = f.fixed_assets.acquire_asset(&asset, f.cash_account_id).unwrap();
        assert!(transaction.is_balanced());
        assert_eq!(acquired.transaction_id, transaction.id);
    }

    #[test]
    fn recording_depreciation_updates_accumulated_depreciation() {
        let f = fixture();
        let asset = Asset::new(f.company_id, f.category_id, "Office Laptop", date(2026, 1, 1), dec!(2000), dec!(200), 5);
        let (acquired, _) = f.fixed_assets.acquire_asset(&asset, f.cash_account_id).unwrap();

        let transaction = f.fixed_assets.record_depreciation(acquired.id.unwrap(), date(2026, 12, 31)).unwrap().unwrap();
        assert!(transaction.is_balanced());
        assert_eq!(transaction.total_debits(), dec!(360)); // (2000-200)/5

        let refetched = f.fixed_assets.get_asset(acquired.id.unwrap()).unwrap();
        assert_eq!(refetched.accumulated_depreciation, dec!(360));
        assert_eq!(refetched.book_value(), dec!(1640));
    }

    #[test]
    fn disposing_an_asset_marks_it_disposed_and_posts_gain_or_loss() {
        let f = fixture();
        let asset = Asset::new(f.company_id, f.category_id, "Office Laptop", date(2026, 1, 1), dec!(2000), dec!(200), 5);
        let (acquired, _) = f.fixed_assets.acquire_asset(&asset, f.cash_account_id).unwrap();
        f.fixed_assets.record_depreciation(acquired.id.unwrap(), date(2026, 6, 30)).unwrap();

        let transaction = f
            .fixed_assets
            .dispose_asset(acquired.id.unwrap(), date(2026, 7, 1), dec!(1000), f.cash_account_id, f.gain_loss_account_id)
            .unwrap();
        assert!(transaction.is_balanced());

        let result = f.fixed_assets.dispose_asset(acquired.id.unwrap(), date(2026, 7, 2), dec!(0), f.cash_account_id, f.gain_loss_account_id);
        assert!(result.is_err());
    }

    #[test]
    fn a_fully_depreciated_asset_has_nothing_left_to_depreciate() {
        let f = fixture();
        let asset = Asset::new(f.company_id, f.category_id, "Cheap Gadget", date(2026, 1, 1), dec!(100), dec!(100), 5);
        let (acquired, _) = f.fixed_assets.acquire_asset(&asset, f.cash_account_id).unwrap();

        let result = f.fixed_assets.record_depreciation(acquired.id.unwrap(), date(2026, 12, 31)).unwrap();
        assert!(result.is_none());
    }
}
