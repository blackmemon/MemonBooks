//! The organization domain's service layer, for
//! `settings/settings_view.rs` (company info) and `hr/employees_view.rs`
//! (HR has no dedicated repository or service of its own -- it was
//! folded into `OrganizationRepository` back in the repository layer,
//! so it stays folded into `organization_service.rs` here too).

use crate::backend::core::errors::AppResult;
use crate::backend::hr::employees::Employee;
use crate::backend::organization::companies::Company;
use crate::backend::repositories::organization_repository::OrganizationRepository;
use crate::database::database::Database;

#[derive(Clone)]
pub struct OrganizationService {
    repo: OrganizationRepository,
}

impl OrganizationService {
    pub fn new(db: Database) -> Self {
        Self { repo: OrganizationRepository::new(db) }
    }

    pub fn get_company(&self, company_id: i64) -> AppResult<Company> {
        self.repo.get_company(company_id)
    }

    pub fn update_company(&self, company: &Company) -> AppResult<()> {
        self.repo.update_company(company)
    }

    pub fn list_employees(&self, company_id: i64) -> AppResult<Vec<Employee>> {
        self.repo.list_employees_for_company(company_id)
    }

    pub fn create_employee(&self, employee: &Employee) -> AppResult<Employee> {
        self.repo.create_employee(employee)
    }
}
