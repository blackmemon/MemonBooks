//! Integration test for the purchasing flow (Development Order steps
//! 9-10): vendor -> bill -> post -> payable -> payment -> settled.
//! Mirrors tests/sales/main.rs.

use chrono::NaiveDate;
use rust_decimal_macros::dec;

use memonbooks::backend::accounting::accounting_rules::AccountType;
use memonbooks::backend::accounting::chart_of_accounts::Account;
use memonbooks::backend::accounting::fiscal_periods::FiscalPeriod;
use memonbooks::backend::organization::companies::Company;
use memonbooks::backend::parties::vendors::Vendor;
use memonbooks::backend::purchases::bills::{Bill, BillLine, BillStatus};
use memonbooks::backend::purchases::payments::VendorPayment;
use memonbooks::backend::repositories::accounting_repository::{self, AccountingRepository};
use memonbooks::backend::repositories::organization_repository::{self, OrganizationRepository};
use memonbooks::backend::repositories::party_repository::{self, PartyRepository};
use memonbooks::backend::repositories::purchase_repository::{self, PurchaseRepository};
use memonbooks::database::database::Database;

fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
}

#[test]
fn bill_to_payment_settles_the_payable_and_keeps_the_ledger_balanced() {
    let db = Database::open_in_memory().unwrap();
    db.run_migrations(organization_repository::MIGRATIONS)
        .unwrap();
    db.run_migrations(party_repository::MIGRATIONS).unwrap();
    db.run_migrations(accounting_repository::MIGRATIONS)
        .unwrap();
    db.run_migrations(purchase_repository::MIGRATIONS).unwrap();
    db.run_migrations(memonbooks::backend::audit::audit_logs::MIGRATIONS)
        .unwrap();

    let org_repo = OrganizationRepository::new(db.clone());
    let company_id = org_repo
        .create_company(&Company::new("Acme Trading Co", "USD"))
        .unwrap()
        .id
        .unwrap();

    let party_repo = PartyRepository::new(db.clone());
    let vendor_id = party_repo
        .create_vendor(&Vendor::new(company_id, "Acme Supplies Ltd"))
        .unwrap()
        .id
        .unwrap();

    let accounting_repo = AccountingRepository::new(db.clone());
    accounting_repo
        .create_fiscal_period(&FiscalPeriod::new(
            company_id,
            "January 2026",
            date(2026, 1, 1),
            date(2026, 1, 31),
        ))
        .unwrap();
    let ap = accounting_repo
        .create_account(&Account::new(
            company_id,
            "2000",
            "Accounts Payable",
            AccountType::Liability,
        ))
        .unwrap();
    let expense = accounting_repo
        .create_account(&Account::new(
            company_id,
            "6000",
            "Office Expense",
            AccountType::Expense,
        ))
        .unwrap();
    let cash = accounting_repo
        .create_account(&Account::new(
            company_id,
            "1000",
            "Cash",
            AccountType::Asset,
        ))
        .unwrap();

    let purchase_repo = PurchaseRepository::new(db);
    let mut bill = Bill::new(company_id, vendor_id, date(2026, 1, 10), date(2026, 2, 9));
    bill.lines.push(BillLine::new(
        expense.id.unwrap(),
        "Office supplies",
        dec!(1),
        dec!(450),
    ));
    let bill = purchase_repo.create_bill(&bill).unwrap();

    let (posted_bill, transaction, payable) = purchase_repo
        .post_bill(bill.id.unwrap(), ap.id.unwrap())
        .unwrap();
    assert_eq!(posted_bill.status, BillStatus::Posted);
    assert!(transaction.is_balanced());
    assert_eq!(payable.amount, dec!(450));

    let payment = VendorPayment::new(company_id, vendor_id, date(2026, 1, 20), dec!(450));
    purchase_repo
        .record_vendor_payment(&payment, ap.id.unwrap(), cash.id.unwrap())
        .unwrap();

    assert!(accounting_repo
        .list_open_payables_for_company(company_id)
        .unwrap()
        .is_empty());
    let trial_balance = accounting_repo
        .trial_balance_as_of(company_id, date(2026, 1, 31))
        .unwrap();
    assert!(trial_balance.is_balanced());
}
