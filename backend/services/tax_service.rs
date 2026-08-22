//! The tax domain's service layer, for `accounting/tax_rates_view.rs`.
//! Also wraps `AccountingRepository` internally for the GL account
//! picker the view needs -- a service is free to reach across domains
//! internally; the Architecture Rule only forbids the view doing so.

use crate::backend::accounting::chart_of_accounts::Account;
use crate::backend::core::errors::AppResult;
use crate::backend::repositories::accounting_repository::AccountingRepository;
use crate::backend::repositories::tax_repository::TaxRepository;
use crate::backend::tax::tax_rates::TaxRate;
use crate::database::database::Database;

#[derive(Clone)]
pub struct TaxService {
    tax_repo: TaxRepository,
    accounting_repo: AccountingRepository,
}

impl TaxService {
    pub fn new(db: Database) -> Self {
        Self { tax_repo: TaxRepository::new(db.clone()), accounting_repo: AccountingRepository::new(db) }
    }

    pub fn list_tax_rates(&self, company_id: i64) -> AppResult<Vec<TaxRate>> {
        self.tax_repo.list_tax_rates_for_company(company_id)
    }

    pub fn create_tax_rate(&self, tax_rate: &TaxRate) -> AppResult<TaxRate> {
        self.tax_repo.create_tax_rate(tax_rate)
    }

    pub fn list_gl_accounts(&self, company_id: i64) -> AppResult<Vec<Account>> {
        self.accounting_repo.list_accounts_for_company(company_id)
    }
}
