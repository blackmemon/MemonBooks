//! Integration test for the sales flow (Development Order steps 9-10):
//! customer -> invoice -> post -> receivable -> payment -> settled.

use chrono::NaiveDate;
use rust_decimal_macros::dec;

use memonbooks::backend::accounting::accounting_rules::AccountType;
use memonbooks::backend::accounting::chart_of_accounts::Account;
use memonbooks::backend::accounting::fiscal_periods::FiscalPeriod;
use memonbooks::backend::organization::companies::Company;
use memonbooks::backend::parties::customers::Customer;
use memonbooks::backend::repositories::accounting_repository::{self, AccountingRepository};
use memonbooks::backend::repositories::organization_repository::{self, OrganizationRepository};
use memonbooks::backend::repositories::party_repository::{self, PartyRepository};
use memonbooks::backend::repositories::sales_repository::{self, SalesRepository};
use memonbooks::backend::sales::invoices::{Invoice, InvoiceLine, InvoiceStatus};
use memonbooks::backend::sales::payments::CustomerPayment;
use memonbooks::database::database::Database;

fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
}

#[test]
fn invoice_to_payment_settles_the_receivable_and_keeps_the_ledger_balanced() {
    let db = Database::open_in_memory().unwrap();
    db.run_migrations(organization_repository::MIGRATIONS).unwrap();
    db.run_migrations(party_repository::MIGRATIONS).unwrap();
    db.run_migrations(accounting_repository::MIGRATIONS).unwrap();
    db.run_migrations(sales_repository::MIGRATIONS).unwrap();
    db.run_migrations(memonbooks::backend::audit::audit_logs::MIGRATIONS).unwrap();

    let org_repo = OrganizationRepository::new(db.clone());
    let company_id = org_repo.create_company(&Company::new("Acme Trading Co", "USD")).unwrap().id.unwrap();

    let party_repo = PartyRepository::new(db.clone());
    let customer_id = party_repo.create_customer(&Customer::new(company_id, "Acme Retail")).unwrap().id.unwrap();

    let accounting_repo = AccountingRepository::new(db.clone());
    accounting_repo
        .create_fiscal_period(&FiscalPeriod::new(company_id, "January 2026", date(2026, 1, 1), date(2026, 1, 31)))
        .unwrap();
    let ar = accounting_repo.create_account(&Account::new(company_id, "1100", "Accounts Receivable", AccountType::Asset)).unwrap();
    let revenue = accounting_repo.create_account(&Account::new(company_id, "4000", "Sales Revenue", AccountType::Revenue)).unwrap();
    let cash = accounting_repo.create_account(&Account::new(company_id, "1000", "Cash", AccountType::Asset)).unwrap();

    let sales_repo = SalesRepository::new(db);
    let mut invoice = Invoice::new(company_id, customer_id, date(2026, 1, 10), date(2026, 2, 9));
    invoice.lines.push(InvoiceLine::new(revenue.id.unwrap(), "Consulting", dec!(10), dec!(150)));
    let invoice = sales_repo.create_invoice(&invoice).unwrap();

    let (posted_invoice, transaction, receivable) = sales_repo.post_invoice(invoice.id.unwrap(), ar.id.unwrap()).unwrap();
    assert_eq!(posted_invoice.status, InvoiceStatus::Posted);
    assert!(transaction.is_balanced());
    assert_eq!(receivable.amount, dec!(1500));

    let payment = CustomerPayment::new(company_id, customer_id, date(2026, 1, 20), dec!(1500));
    sales_repo.record_customer_payment(&payment, cash.id.unwrap(), ar.id.unwrap()).unwrap();

    assert!(accounting_repo.list_open_receivables_for_company(company_id).unwrap().is_empty());
    let trial_balance = accounting_repo.trial_balance_as_of(company_id, date(2026, 1, 31)).unwrap();
    assert!(trial_balance.is_balanced());

    let cash_line = trial_balance.lines.iter().find(|l| l.account_id == cash.id.unwrap()).unwrap();
    assert_eq!(cash_line.debit, dec!(1500));
}
