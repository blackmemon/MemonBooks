//! Integration test for tax (Development Order step 14): output tax on
//! a sale and input tax on a purchase, netted into a tax summary.

use chrono::NaiveDate;
use rust_decimal_macros::dec;

use memonbooks::backend::accounting::accounting_rules::AccountType;
use memonbooks::backend::accounting::chart_of_accounts::Account;
use memonbooks::backend::accounting::fiscal_periods::FiscalPeriod;
use memonbooks::backend::organization::companies::Company;
use memonbooks::backend::repositories::accounting_repository::{self, AccountingRepository};
use memonbooks::backend::repositories::organization_repository::{self, OrganizationRepository};
use memonbooks::backend::repositories::tax_repository::{self, TaxRepository};
use memonbooks::backend::tax::tax_rates::{TaxDirection, TaxRate};
use memonbooks::backend::tax::tax_transactions::TaxTransaction;
use memonbooks::database::database::Database;

fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
}

#[test]
fn output_and_input_tax_net_correctly_and_keep_the_ledger_balanced() {
    let db = Database::open_in_memory().unwrap();
    db.run_migrations(organization_repository::MIGRATIONS).unwrap();
    db.run_migrations(accounting_repository::MIGRATIONS).unwrap();
    db.run_migrations(memonbooks::backend::audit::audit_logs::MIGRATIONS).unwrap();
    db.run_migrations(tax_repository::MIGRATIONS).unwrap();

    let org_repo = OrganizationRepository::new(db.clone());
    let company_id = org_repo.create_company(&Company::new("Acme Trading Co", "USD")).unwrap().id.unwrap();

    let accounting_repo = AccountingRepository::new(db.clone());
    accounting_repo
        .create_fiscal_period(&FiscalPeriod::new(company_id, "January 2026", date(2026, 1, 1), date(2026, 1, 31)))
        .unwrap();
    let vat_payable = accounting_repo.create_account(&Account::new(company_id, "2100", "VAT Payable", AccountType::Liability)).unwrap();
    let vat_receivable = accounting_repo.create_account(&Account::new(company_id, "1300", "VAT Receivable", AccountType::Asset)).unwrap();
    let ar = accounting_repo.create_account(&Account::new(company_id, "1100", "Accounts Receivable", AccountType::Asset)).unwrap();
    let ap = accounting_repo.create_account(&Account::new(company_id, "2000", "Accounts Payable", AccountType::Liability)).unwrap();

    let tax_repo = TaxRepository::new(db);
    let output_rate = tax_repo.create_tax_rate(&TaxRate::new(company_id, "Standard VAT 15% (Output)", dec!(15), TaxDirection::Output, vat_payable.id.unwrap())).unwrap();
    let input_rate = tax_repo.create_tax_rate(&TaxRate::new(company_id, "Standard VAT 15% (Input)", dec!(15), TaxDirection::Input, vat_receivable.id.unwrap())).unwrap();

    tax_repo
        .post_tax(&TaxTransaction::new(company_id, output_rate.id.unwrap(), date(2026, 1, 10), dec!(1000), dec!(150), TaxDirection::Output), ar.id.unwrap())
        .unwrap();
    tax_repo
        .post_tax(&TaxTransaction::new(company_id, input_rate.id.unwrap(), date(2026, 1, 15), dec!(400), dec!(60), TaxDirection::Input), ap.id.unwrap())
        .unwrap();

    let summary = tax_repo.tax_summary(company_id, date(2026, 1, 1), date(2026, 1, 31)).unwrap();
    assert_eq!(summary.total_output_tax, dec!(150));
    assert_eq!(summary.total_input_tax, dec!(60));
    assert_eq!(summary.net_tax_payable(), dec!(90));

    let trial_balance = accounting_repo.trial_balance_as_of(company_id, date(2026, 1, 31)).unwrap();
    assert!(trial_balance.is_balanced());
}
