//! The organization domain's service layer, for
//! `settings/settings_view.rs` (company info and the multi-company
//! create/switch list) and `hr/employees_view.rs` (HR has no dedicated
//! repository or service of its own -- it was folded into
//! `OrganizationRepository` back in the repository layer, so it stays
//! folded into `organization_service.rs` here too).

use chrono::{Datelike, NaiveDate, Utc};

use crate::backend::accounting::fiscal_periods::FiscalPeriod;
use crate::backend::core::errors::AppResult;
use crate::backend::hr::employees::Employee;
use crate::backend::hr::payroll::PayrollRun;
use crate::backend::hr::salary::SalaryStructure;
use crate::backend::organization::companies::Company;
use crate::backend::repositories::accounting_repository::AccountingRepository;
use crate::backend::repositories::organization_repository::OrganizationRepository;
use crate::backend::transactions::transaction::Transaction;
use crate::database::database::Database;

#[derive(Clone)]
pub struct OrganizationService {
    repo: OrganizationRepository,
    accounting_repo: AccountingRepository,
}

impl OrganizationService {
    pub fn new(db: Database) -> Self {
        Self { repo: OrganizationRepository::new(db.clone()), accounting_repo: AccountingRepository::new(db) }
    }

    pub fn get_company(&self, company_id: i64) -> AppResult<Company> {
        self.repo.get_company(company_id)
    }

    pub fn update_company(&self, company: &Company) -> AppResult<()> {
        self.repo.update_company(company)
    }

    pub fn list_companies(&self) -> AppResult<Vec<Company>> {
        self.repo.list_companies()
    }

    /// Creates a company *and* an open fiscal period covering the
    /// current calendar year, so it has somewhere to post to
    /// immediately -- the same thing `src/main.rs`'s
    /// `ensure_default_company` does for the bootstrap company, just
    /// reachable from the UI for every company after the first.
    pub fn create_company(&self, name: impl Into<String>, base_currency_code: impl Into<String>) -> AppResult<Company> {
        let company = self.repo.create_company(&Company::new(name, base_currency_code))?;
        let company_id = company.id.expect("just-created company must have an id");

        let today = Utc::now().date_naive();
        let year_start = chrono::NaiveDate::from_ymd_opt(today.year(), 1, 1).unwrap_or(today);
        let year_end = chrono::NaiveDate::from_ymd_opt(today.year(), 12, 31).unwrap_or(today);
        self.accounting_repo
            .create_fiscal_period(&FiscalPeriod::new(company_id, format!("FY{}", today.year()), year_start, year_end))?;

        Ok(company)
    }

    pub fn list_employees(&self, company_id: i64) -> AppResult<Vec<Employee>> {
        self.repo.list_employees_for_company(company_id)
    }

    pub fn create_employee(&self, employee: &Employee) -> AppResult<Employee> {
        self.repo.create_employee(employee)
    }

    pub fn create_salary_structure(&self, structure: &SalaryStructure) -> AppResult<SalaryStructure> {
        self.repo.create_salary_structure(structure)
    }

    pub fn current_salary_structure(&self, employee_id: i64, as_of: NaiveDate) -> AppResult<Option<SalaryStructure>> {
        self.repo.current_salary_structure(employee_id, as_of)
    }

    pub fn post_payroll_run(
        &self,
        run: &PayrollRun,
        salary_expense_account_id: i64,
        deductions_payable_account_id: i64,
        net_pay_account_id: i64,
    ) -> AppResult<(PayrollRun, Transaction)> {
        self.repo.post_payroll_run(run, salary_expense_account_id, deductions_payable_account_id, net_pay_account_id)
    }
}
