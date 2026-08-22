//! The accounting domain's service layer: the only thing views are
//! allowed to talk to for accounting data (Architecture Rules, Section
//! 3 -- "Iced View -> Service -> Repository -> Database"). Right now
//! this is a thin delegate to `AccountingRepository`, since the
//! repository already owns validation and atomicity; as cross-cutting
//! concerns (permission checks tied to `core::permissions`, for
//! instance) come up, they belong here, not back in the view.
//!
//! Grown incrementally as each view is migrated onto it -- covers what
//! `chart_of_accounts_view.rs` and `journal_entry_view.rs` need.

use crate::backend::accounting::chart_of_accounts::Account;
use crate::backend::core::errors::AppResult;
use crate::backend::repositories::accounting_repository::AccountingRepository;
use crate::backend::transactions::transaction::Transaction;
use crate::database::database::Database;

#[derive(Clone)]
pub struct AccountingService {
    repo: AccountingRepository,
}

impl AccountingService {
    pub fn new(db: Database) -> Self {
        Self { repo: AccountingRepository::new(db) }
    }

    pub fn list_accounts(&self, company_id: i64) -> AppResult<Vec<Account>> {
        self.repo.list_accounts_for_company(company_id)
    }

    pub fn create_account(&self, account: &Account) -> AppResult<Account> {
        self.repo.create_account(account)
    }

    pub fn post_transaction(&self, transaction: &Transaction) -> AppResult<Transaction> {
        self.repo.post_transaction(transaction)
    }
}
