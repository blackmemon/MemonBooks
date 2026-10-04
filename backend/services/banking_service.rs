//! The banking domain's service layer, for
//! `reports/bank_accounts_view.rs`. Grouped under `frontend/views/reports/`
//! for navigation (the frozen structure has no dedicated `banking/`
//! view directory), but the view mutates data (creating bank accounts),
//! so it routes through banking_service.rs rather than
//! report_service.rs, which is reserved for read-only presentation.
//! Also wraps `AccountingRepository` internally for the GL account
//! picker the view needs.

use chrono::NaiveDate;
use rust_decimal::Decimal;

use crate::backend::accounting::chart_of_accounts::Account;
use crate::backend::banking::bank_accounts::BankAccount;
use crate::backend::core::errors::AppResult;
use crate::backend::repositories::accounting_repository::AccountingRepository;
use crate::backend::repositories::banking_repository::BankingRepository;
use crate::database::database::Database;

#[derive(Clone)]
pub struct BankingService {
    banking_repo: BankingRepository,
    accounting_repo: AccountingRepository,
}

impl BankingService {
    pub fn new(db: Database) -> Self {
        Self {
            banking_repo: BankingRepository::new(db.clone()),
            accounting_repo: AccountingRepository::new(db),
        }
    }

    pub fn list_bank_accounts(&self, company_id: i64) -> AppResult<Vec<BankAccount>> {
        self.banking_repo.list_bank_accounts_for_company(company_id)
    }

    pub fn book_balance(&self, bank_account_id: i64, as_of: NaiveDate) -> AppResult<Decimal> {
        self.banking_repo.book_balance(bank_account_id, as_of)
    }

    pub fn create_bank_account(&self, bank_account: &BankAccount) -> AppResult<BankAccount> {
        self.banking_repo.create_bank_account(bank_account)
    }

    pub fn update_bank_account(&self, bank_account: &BankAccount) -> AppResult<()> {
        self.banking_repo.update_bank_account(bank_account)
    }

    pub fn delete_bank_account(&self, id: i64) -> AppResult<()> {
        self.banking_repo.delete_bank_account(id)
    }

    pub fn list_gl_accounts(&self, company_id: i64) -> AppResult<Vec<Account>> {
        self.accounting_repo.list_accounts_for_company(company_id)
    }
}
