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
use crate::backend::accounting::fiscal_periods::FiscalPeriod;
use crate::backend::accounting::trial_balance::TrialBalance;
use crate::backend::core::errors::AppResult;
use crate::backend::repositories::accounting_repository::AccountingRepository;
use crate::backend::transactions::transaction::Transaction;
use crate::backend::transactions::transaction_types::TransactionType;
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

    pub fn update_account(&self, account: &Account) -> AppResult<()> {
        self.repo.update_account(account)
    }

    pub fn delete_account(&self, id: i64) -> AppResult<()> {
        self.repo.delete_account(id)
    }

    /// Every account's net balance as of `as_of` -- exposed here (not
    /// just through `ReportService`) so `chart_of_accounts_view` can
    /// show live debit/credit/net balance columns without a second
    /// service dependency.
    pub fn trial_balance_as_of(&self, company_id: i64, as_of: chrono::NaiveDate) -> AppResult<TrialBalance> {
        self.repo.trial_balance_as_of(company_id, as_of)
    }

    pub fn post_transaction(&self, transaction: &Transaction) -> AppResult<Transaction> {
        self.repo.post_transaction(transaction)
    }

    /// Every manual journal entry (not every transaction -- sales
    /// invoices, bill postings, etc. all go through `transactions` too,
    /// but this screen is scoped to the ones an accountant entered
    /// directly). A reversal of one is its own `Reversal`-typed
    /// transaction, not itself a `ManualJournal`, so it doesn't appear
    /// as a second row here -- the original's status just becomes
    /// Reversed, mirroring how a credit note doesn't add its own row to
    /// the Invoices list either.
    pub fn list_journal_entries(&self, company_id: i64) -> AppResult<Vec<Transaction>> {
        Ok(self.repo.list_transactions_for_company(company_id)?.into_iter().filter(|t| t.transaction_type == TransactionType::ManualJournal).collect())
    }

    /// Builds and posts the reversal of an already-posted journal entry
    /// -- see `AccountingRepository::reverse_transaction`. The
    /// immutability-correct way to fix a posted entry: never edited in
    /// place, only corrected via a new, dated, linked transaction.
    pub fn reverse_transaction(&self, original_id: i64, reversal_date: chrono::NaiveDate) -> AppResult<Transaction> {
        self.repo.reverse_transaction(original_id, reversal_date)
    }

    pub fn create_fiscal_period(&self, period: &FiscalPeriod) -> AppResult<FiscalPeriod> {
        self.repo.create_fiscal_period(period)
    }
}
