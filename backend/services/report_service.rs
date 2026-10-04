//! The reporting service layer: every read-only report/dashboard view
//! (`accounting/trial_balance_view.rs`, `dashboard/dashboard_view.rs`,
//! `reports/reports_view.rs`) routes here rather than through
//! `accounting_service.rs`, even though the data underneath is the
//! accounting ledger -- reports are their own concern (aggregation and
//! presentation of already-posted data), not chart-of-accounts/
//! transaction mutation, so they get their own service. Internally this
//! wraps `AccountingRepository`, since that's where the report-shaped
//! queries (trial balance, open receivables/payables, P&L balances)
//! already live.

use chrono::NaiveDate;

use crate::backend::accounting::accounts_payable::payables::Payable;
use crate::backend::accounting::accounts_receivable::receivables::Receivable;
use crate::backend::accounting::chart_of_accounts::Account;
use crate::backend::accounting::period_closing::ClosingBalance;
use crate::backend::accounting::trial_balance::TrialBalance;
use crate::backend::core::errors::AppResult;
use crate::backend::repositories::accounting_repository::AccountingRepository;
use crate::database::database::Database;

#[derive(Clone)]
pub struct ReportService {
    repo: AccountingRepository,
}

impl ReportService {
    pub fn general_ledger(
        &self,
        company: i64,
        account: i64,
        as_of: NaiveDate,
    ) -> AppResult<Vec<crate::backend::accounting::general_ledger::GeneralLedgerEntry>> {
        if self.repo.get_account(account)?.company_id != company {
            return Err(crate::backend::core::errors::AppError::Validation(
                "Account belongs to another company".into(),
            ));
        }
        self.repo
            .general_ledger_for_account(account, rust_decimal::Decimal::ZERO, as_of)
    }

    pub fn party_open_items(
        &self,
        company: i64,
        party_type: &str,
        party: i64,
        as_of: NaiveDate,
    ) -> AppResult<
        Vec<(
            NaiveDate,
            String,
            rust_decimal::Decimal,
            rust_decimal::Decimal,
        )>,
    > {
        if party_type == "Customer" {
            Ok(self
                .repo
                .list_open_receivables_for_company(company)?
                .into_iter()
                .filter(|r| r.customer_id == party && r.due_date <= as_of)
                .map(|r| {
                    (
                        r.due_date,
                        r.reference.clone().unwrap_or_default(),
                        r.amount,
                        r.balance(),
                    )
                })
                .collect())
        } else {
            Ok(self
                .repo
                .list_open_payables_for_company(company)?
                .into_iter()
                .filter(|r| r.vendor_id == party && r.due_date <= as_of)
                .map(|r| {
                    (
                        r.due_date,
                        r.reference.clone().unwrap_or_default(),
                        r.amount,
                        r.balance(),
                    )
                })
                .collect())
        }
    }
    pub fn new(db: Database) -> Self {
        Self {
            repo: AccountingRepository::new(db),
        }
    }

    pub fn list_accounts(&self, company_id: i64) -> AppResult<Vec<Account>> {
        self.repo.list_accounts_for_company(company_id)
    }

    pub fn trial_balance_as_of(
        &self,
        company_id: i64,
        as_of: NaiveDate,
    ) -> AppResult<TrialBalance> {
        self.repo.trial_balance_as_of(company_id, as_of)
    }

    pub fn revenue_and_expense_balances_as_of(
        &self,
        company_id: i64,
        as_of: NaiveDate,
    ) -> AppResult<Vec<ClosingBalance>> {
        self.repo
            .revenue_and_expense_balances_as_of(company_id, as_of)
    }

    pub fn list_open_receivables(&self, company_id: i64) -> AppResult<Vec<Receivable>> {
        self.repo.list_open_receivables_for_company(company_id)
    }

    pub fn list_open_payables(&self, company_id: i64) -> AppResult<Vec<Payable>> {
        self.repo.list_open_payables_for_company(company_id)
    }
}
