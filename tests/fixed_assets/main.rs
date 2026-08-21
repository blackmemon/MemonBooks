//! Integration test for fixed assets (Development Order step 13):
//! acquire -> depreciate -> dispose, verified end to end against the
//! ledger rather than just the domain-level math each already has unit
//! tests for.

use chrono::NaiveDate;
use rust_decimal_macros::dec;

use memonbooks::backend::accounting::accounting_rules::AccountType;
use memonbooks::backend::accounting::chart_of_accounts::Account;
use memonbooks::backend::accounting::fiscal_periods::FiscalPeriod;
use memonbooks::backend::fixed_assets::asset_categories::AssetCategory;
use memonbooks::backend::fixed_assets::assets::Asset;
use memonbooks::backend::organization::companies::Company;
use memonbooks::backend::repositories::accounting_repository::{self, AccountingRepository};
use memonbooks::backend::repositories::fixed_asset_repository::{self, FixedAssetRepository};
use memonbooks::backend::repositories::organization_repository::{self, OrganizationRepository};
use memonbooks::database::database::Database;

fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
}

#[test]
fn acquire_depreciate_dispose_keeps_the_ledger_balanced_throughout() {
    let db = Database::open_in_memory().unwrap();
    db.run_migrations(organization_repository::MIGRATIONS).unwrap();
    db.run_migrations(accounting_repository::MIGRATIONS).unwrap();
    db.run_migrations(memonbooks::backend::audit::audit_logs::MIGRATIONS).unwrap();
    db.run_migrations(fixed_asset_repository::MIGRATIONS).unwrap();

    let org_repo = OrganizationRepository::new(db.clone());
    let company_id = org_repo.create_company(&Company::new("Acme Trading Co", "USD")).unwrap().id.unwrap();

    let accounting_repo = AccountingRepository::new(db.clone());
    accounting_repo
        .create_fiscal_period(&FiscalPeriod::new(company_id, "Fiscal Year 2026", date(2026, 1, 1), date(2026, 12, 31)))
        .unwrap();
    let asset_account = accounting_repo.create_account(&Account::new(company_id, "1500", "Computer Equipment", AccountType::Asset)).unwrap();
    let accumulated_depreciation = accounting_repo.create_account(&Account::new(company_id, "1510", "Accumulated Depreciation", AccountType::Asset)).unwrap();
    let depreciation_expense = accounting_repo.create_account(&Account::new(company_id, "6500", "Depreciation Expense", AccountType::Expense)).unwrap();
    let cash = accounting_repo.create_account(&Account::new(company_id, "1000", "Cash", AccountType::Asset)).unwrap();
    let gain_loss = accounting_repo.create_account(&Account::new(company_id, "7000", "Gain/Loss on Disposal", AccountType::Revenue)).unwrap();

    let fixed_asset_repo = FixedAssetRepository::new(db);
    let category = fixed_asset_repo
        .create_category(&AssetCategory::new(company_id, "Computer Equipment", asset_account.id.unwrap(), accumulated_depreciation.id.unwrap(), depreciation_expense.id.unwrap(), 5))
        .unwrap();

    let (acquired, acquisition_transaction) = fixed_asset_repo
        .acquire_asset(&Asset::new(company_id, category.id.unwrap(), "Office Laptop", date(2026, 1, 1), dec!(2000), dec!(200), 5), cash.id.unwrap())
        .unwrap();
    assert!(acquisition_transaction.is_balanced());

    let depreciation_transaction = fixed_asset_repo.record_depreciation(acquired.id.unwrap(), date(2026, 12, 31)).unwrap().unwrap();
    assert!(depreciation_transaction.is_balanced());
    assert_eq!(depreciation_transaction.total_debits(), dec!(360)); // (2000-200)/5

    let disposal_transaction = fixed_asset_repo
        .dispose_asset(acquired.id.unwrap(), date(2026, 12, 31), dec!(1500), cash.id.unwrap(), gain_loss.id.unwrap())
        .unwrap();
    assert!(disposal_transaction.is_balanced());

    let trial_balance = accounting_repo.trial_balance_as_of(company_id, date(2026, 12, 31)).unwrap();
    assert!(trial_balance.is_balanced());

    // The asset and accumulated depreciation accounts should now net to
    // zero -- everything was cleared off the books by the disposal.
    let net_asset_side: rust_decimal::Decimal = trial_balance
        .lines
        .iter()
        .filter(|l| l.account_id == asset_account.id.unwrap() || l.account_id == accumulated_depreciation.id.unwrap())
        .map(|l| l.debit - l.credit)
        .sum();
    assert_eq!(net_asset_side, rust_decimal::Decimal::ZERO);
}
