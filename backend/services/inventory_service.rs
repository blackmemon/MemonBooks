//! The inventory domain's service layer, for
//! `inventory/products_view.rs` and `inventory/warehouses_view.rs`.

use crate::backend::core::errors::AppResult;
use crate::backend::inventory::products::Product;
use crate::backend::inventory::warehouses::Warehouse;
use crate::backend::repositories::inventory_repository::InventoryRepository;
use crate::database::database::Database;

#[derive(Clone)]
pub struct InventoryService {
    repo: InventoryRepository,
}

impl InventoryService {
    pub fn new(db: Database) -> Self {
        Self { repo: InventoryRepository::new(db) }
    }

    pub fn list_products(&self, company_id: i64) -> AppResult<Vec<Product>> {
        self.repo.list_products_for_company(company_id)
    }

    pub fn create_product(&self, product: &Product) -> AppResult<Product> {
        self.repo.create_product(product)
    }

    pub fn update_product(&self, product: &Product) -> AppResult<()> {
        self.repo.update_product(product)
    }

    pub fn delete_product(&self, id: i64) -> AppResult<()> {
        self.repo.delete_product(id)
    }

    pub fn list_warehouses(&self, company_id: i64) -> AppResult<Vec<Warehouse>> {
        self.repo.list_warehouses_for_company(company_id)
    }

    pub fn create_warehouse(&self, warehouse: &Warehouse) -> AppResult<Warehouse> {
        self.repo.create_warehouse(warehouse)
    }

    pub fn update_warehouse(&self, warehouse: &Warehouse) -> AppResult<()> {
        self.repo.update_warehouse(warehouse)
    }

    pub fn delete_warehouse(&self, id: i64) -> AppResult<()> {
        self.repo.delete_warehouse(id)
    }
}
