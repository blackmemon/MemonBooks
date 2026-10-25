//! The banking domain's service layer, for
//! `accounting/maintain/bank_of_accounts_view.rs`. The view mutates data
//! (creating, editing and archiving bank accounts), so it routes through
//! banking_service.rs rather than report_service.rs, which is reserved
//! for read-only presentation. Also wraps `AccountingRepository` for the
//! GL account picker and `OrganizationRepository` for the company's
//! currency.

use chrono::NaiveDate;
use rust_decimal::Decimal;

use crate::backend::accounting::chart_of_accounts::Account;
use crate::backend::banking::bank_accounts::BankAccount;
use crate::backend::banking::bank_ledger::BankLedger;
use crate::backend::core::errors::AppResult;
use crate::backend::organization::companies::Company;
use crate::backend::repositories::accounting_repository::AccountingRepository;
use crate::backend::repositories::banking_repository::{BankingRepository, Removal};
use crate::backend::repositories::organization_repository::OrganizationRepository;
use crate::database::database::Database;

#[derive(Clone)]
pub struct BankingService {
    banking_repo: BankingRepository,
    accounting_repo: AccountingRepository,
    organization_repo: OrganizationRepository,
}

impl BankingService {
    pub fn new(db: Database) -> Self {
        Self {
            banking_repo: BankingRepository::new(db.clone()),
            accounting_repo: AccountingRepository::new(db.clone()),
            organization_repo: OrganizationRepository::new(db),
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

    /// Deletes the account, or archives it when it has history.
    pub fn remove_bank_account(&self, id: i64) -> AppResult<Removal> {
        self.banking_repo.remove_bank_account(id)
    }

    pub fn bank_ledger(&self, bank_account_id: i64) -> AppResult<BankLedger> {
        self.banking_repo.bank_ledger(bank_account_id)
    }

    /// The ledger without per-entry details, for balances only.
    pub fn bank_ledger_summary(&self, bank_account_id: i64) -> AppResult<BankLedger> {
        self.banking_repo.bank_ledger_summary(bank_account_id)
    }

    pub fn list_gl_accounts(&self, company_id: i64) -> AppResult<Vec<Account>> {
        self.accounting_repo.list_accounts_for_company(company_id)
    }

    /// Every company, for the company pickers.
    pub fn list_companies(&self) -> AppResult<Vec<Company>> {
        self.organization_repo.list_companies()
    }

    /// The company's base currency, which new bank accounts default to.
    pub fn company_currency(&self, company_id: i64) -> AppResult<String> {
        self.organization_repo
            .get_company(company_id)
            .map(|company| company.base_currency_code)
    }
}
