//! The parties (customers/vendors) domain's service layer, for
//! `sales/customers_view.rs` and `inventory/vendors_view.rs`.

use crate::backend::core::errors::AppResult;
use crate::backend::parties::customers::Customer;
use crate::backend::parties::vendors::Vendor;
use crate::backend::repositories::party_repository::PartyRepository;
use crate::database::database::Database;

#[derive(Clone)]
pub struct PartyService {
    repo: PartyRepository,
}

impl PartyService {
    pub fn new(db: Database) -> Self {
        Self { repo: PartyRepository::new(db) }
    }

    pub fn list_customers(&self, company_id: i64) -> AppResult<Vec<Customer>> {
        self.repo.list_customers_for_company(company_id)
    }

    pub fn create_customer(&self, customer: &Customer) -> AppResult<Customer> {
        self.repo.create_customer(customer)
    }

    pub fn update_customer(&self, customer: &Customer) -> AppResult<()> {
        self.repo.update_customer(customer)
    }

    pub fn delete_customer(&self, id: i64) -> AppResult<()> {
        self.repo.delete_customer(id)
    }

    pub fn list_vendors(&self, company_id: i64) -> AppResult<Vec<Vendor>> {
        self.repo.list_vendors_for_company(company_id)
    }

    pub fn create_vendor(&self, vendor: &Vendor) -> AppResult<Vendor> {
        self.repo.create_vendor(vendor)
    }

    pub fn update_vendor(&self, vendor: &Vendor) -> AppResult<()> {
        self.repo.update_vendor(vendor)
    }

    pub fn delete_vendor(&self, id: i64) -> AppResult<()> {
        self.repo.delete_vendor(id)
    }
}
