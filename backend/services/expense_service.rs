//! The expenses domain's service layer, for `hr/expenses_view.rs`. Also
//! wraps `AccountingRepository` internally for the GL account pickers
//! the view needs -- a service is free to reach across domains
//! internally; the Architecture Rule only forbids the view doing so.

use chrono::NaiveDate;

use crate::backend::accounting::chart_of_accounts::Account;
use crate::backend::core::errors::AppResult;
use crate::backend::expenses::expense_categories::ExpenseCategory;
use crate::backend::expenses::expense_claims::ExpenseClaim;
use crate::backend::expenses::expenses::Expense;
use crate::backend::repositories::accounting_repository::AccountingRepository;
use crate::backend::repositories::expense_repository::ExpenseRepository;
use crate::backend::transactions::transaction::Transaction;
use crate::database::database::Database;

#[derive(Clone)]
pub struct ExpenseService {
    expense_repo: ExpenseRepository,
    accounting_repo: AccountingRepository,
}

impl ExpenseService {
    pub fn new(db: Database) -> Self {
        Self { expense_repo: ExpenseRepository::new(db.clone()), accounting_repo: AccountingRepository::new(db) }
    }

    pub fn list_gl_accounts(&self, company_id: i64) -> AppResult<Vec<Account>> {
        self.accounting_repo.list_accounts_for_company(company_id)
    }

    pub fn create_category(&self, category: &ExpenseCategory) -> AppResult<ExpenseCategory> {
        self.expense_repo.create_category(category)
    }

    pub fn record_expense(&self, expense: &Expense) -> AppResult<(Expense, Transaction)> {
        self.expense_repo.record_expense(expense)
    }

    pub fn create_claim(&self, claim: &ExpenseClaim) -> AppResult<ExpenseClaim> {
        self.expense_repo.create_claim(claim)
    }

    pub fn approve_claim(&self, id: i64) -> AppResult<ExpenseClaim> {
        self.expense_repo.approve_claim(id)
    }

    pub fn reject_claim(&self, id: i64) -> AppResult<ExpenseClaim> {
        self.expense_repo.reject_claim(id)
    }

    pub fn reimburse_claim(&self, id: i64, payment_account_id: i64, reimbursement_date: NaiveDate) -> AppResult<(ExpenseClaim, Transaction)> {
        self.expense_repo.reimburse_claim(id, payment_account_id, reimbursement_date)
    }
}
