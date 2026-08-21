//! Persistence for warehouses, categories, products, and stock movements.

use rust_decimal::Decimal;
use rusqlite::{params, OptionalExtension, Row};

use crate::backend::core::errors::{AppError, AppResult};
use crate::backend::core::validation::Validate;
use crate::backend::inventory::categories::Category;
use crate::backend::inventory::products::Product;
use crate::backend::inventory::stock;
use crate::backend::inventory::stock_movements::{MovementType, StockMovement};
use crate::backend::inventory::transfers::StockTransfer;
use crate::backend::inventory::warehouses::Warehouse;
use crate::database::database::{Database, Migration};

pub const MIGRATIONS: &[Migration] = &[Migration {
    version: 1100,
    name: "inventory_schema",
    sql: "
        CREATE TABLE warehouses (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            company_id  INTEGER NOT NULL REFERENCES companies(id),
            name        TEXT NOT NULL,
            code        TEXT NOT NULL,
            address     TEXT,
            is_active   INTEGER NOT NULL DEFAULT 1,
            UNIQUE (company_id, code)
        );

        CREATE TABLE categories (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            company_id  INTEGER NOT NULL REFERENCES companies(id),
            name        TEXT NOT NULL,
            parent_id   INTEGER REFERENCES categories(id)
        );

        CREATE TABLE products (
            id                  INTEGER PRIMARY KEY AUTOINCREMENT,
            company_id          INTEGER NOT NULL REFERENCES companies(id),
            sku                 TEXT NOT NULL,
            name                TEXT NOT NULL,
            category_id         INTEGER REFERENCES categories(id),
            unit_of_measure     TEXT NOT NULL,
            cost_price          TEXT NOT NULL,
            sale_price          TEXT NOT NULL,
            track_inventory     INTEGER NOT NULL DEFAULT 1,
            is_active           INTEGER NOT NULL DEFAULT 1,
            UNIQUE (company_id, sku)
        );

        CREATE TABLE stock_movements (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            company_id      INTEGER NOT NULL REFERENCES companies(id),
            product_id      INTEGER NOT NULL REFERENCES products(id),
            warehouse_id    INTEGER NOT NULL REFERENCES warehouses(id),
            movement_date   TEXT NOT NULL,
            movement_type   TEXT NOT NULL,
            quantity_delta  TEXT NOT NULL,
            reference       TEXT,
            transaction_id  INTEGER REFERENCES transactions(id)
        );
    ",
}, Migration {
    // Additive: products already has rows by the time POS (step 16)
    // needs barcode lookup, so this extends the existing table rather
    // than editing migration 1100's already-applied SQL.
    version: 1600,
    name: "products_barcode_column",
    sql: "ALTER TABLE products ADD COLUMN barcode TEXT;",
}];

#[derive(Clone)]
pub struct InventoryRepository {
    db: Database,
}

impl InventoryRepository {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    // -- warehouses -----------------------------------------------------

    pub fn create_warehouse(&self, warehouse: &Warehouse) -> AppResult<Warehouse> {
        warehouse.validate()?;
        self.db.with_connection(|conn| {
            conn.execute(
                "INSERT INTO warehouses (company_id, name, code, address, is_active) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![warehouse.company_id, warehouse.name, warehouse.code, warehouse.address, warehouse.is_active],
            )?;
            Ok(Warehouse { id: Some(conn.last_insert_rowid()), ..warehouse.clone() })
        })
    }

    pub fn list_warehouses_for_company(&self, company_id: i64) -> AppResult<Vec<Warehouse>> {
        self.db.with_connection(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, company_id, name, code, address, is_active FROM warehouses WHERE company_id = ?1 ORDER BY name",
            )?;
            let rows = stmt.query_map(params![company_id], map_warehouse)?;
            rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
        })
    }

    // -- categories -----------------------------------------------------

    pub fn create_category(&self, category: &Category) -> AppResult<Category> {
        category.validate()?;
        self.db.with_connection(|conn| {
            conn.execute(
                "INSERT INTO categories (company_id, name, parent_id) VALUES (?1, ?2, ?3)",
                params![category.company_id, category.name, category.parent_id],
            )?;
            Ok(Category { id: Some(conn.last_insert_rowid()), ..category.clone() })
        })
    }

    pub fn list_categories_for_company(&self, company_id: i64) -> AppResult<Vec<Category>> {
        self.db.with_connection(|conn| {
            let mut stmt = conn.prepare("SELECT id, company_id, name, parent_id FROM categories WHERE company_id = ?1 ORDER BY name")?;
            let rows = stmt.query_map(params![company_id], map_category)?;
            rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
        })
    }

    // -- products -----------------------------------------------------

    pub fn create_product(&self, product: &Product) -> AppResult<Product> {
        product.validate()?;
        self.db.with_connection(|conn| {
            conn.execute(
                "INSERT INTO products (company_id, sku, name, barcode, category_id, unit_of_measure, cost_price, sale_price, track_inventory, is_active)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    product.company_id,
                    product.sku,
                    product.name,
                    product.barcode,
                    product.category_id,
                    product.unit_of_measure,
                    product.cost_price.to_string(),
                    product.sale_price.to_string(),
                    product.track_inventory,
                    product.is_active,
                ],
            )?;
            Ok(Product { id: Some(conn.last_insert_rowid()), ..product.clone() })
        })
    }

    pub fn get_product(&self, id: i64) -> AppResult<Product> {
        self.db.with_connection(|conn| {
            conn.query_row(
                "SELECT id, company_id, sku, name, barcode, category_id, unit_of_measure, cost_price, sale_price, track_inventory, is_active
                 FROM products WHERE id = ?1",
                params![id],
                map_product,
            )
            .optional()?
            .ok_or_else(|| AppError::NotFound(format!("product {id} not found")))
        })
    }

    pub fn get_product_by_barcode(&self, company_id: i64, barcode: &str) -> AppResult<Option<Product>> {
        self.db.with_connection(|conn| {
            conn.query_row(
                "SELECT id, company_id, sku, name, barcode, category_id, unit_of_measure, cost_price, sale_price, track_inventory, is_active
                 FROM products WHERE company_id = ?1 AND barcode = ?2",
                params![company_id, barcode],
                map_product,
            )
            .optional()
            .map_err(AppError::from)
        })
    }

    pub fn list_products_for_company(&self, company_id: i64) -> AppResult<Vec<Product>> {
        self.db.with_connection(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, company_id, sku, name, barcode, category_id, unit_of_measure, cost_price, sale_price, track_inventory, is_active
                 FROM products WHERE company_id = ?1 ORDER BY sku",
            )?;
            let rows = stmt.query_map(params![company_id], map_product)?;
            rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
        })
    }

    // -- stock movements --------------------------------------------------

    pub fn record_movement(&self, movement: &StockMovement) -> AppResult<StockMovement> {
        movement.validate()?;
        self.db.with_connection(|conn| {
            conn.execute(
                "INSERT INTO stock_movements (company_id, product_id, warehouse_id, movement_date, movement_type, quantity_delta, reference, transaction_id)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    movement.company_id,
                    movement.product_id,
                    movement.warehouse_id,
                    movement.movement_date,
                    movement.movement_type.as_str(),
                    movement.quantity_delta.to_string(),
                    movement.reference,
                    movement.transaction_id,
                ],
            )?;
            Ok(StockMovement { id: Some(conn.last_insert_rowid()), ..movement.clone() })
        })
    }

    /// Records both legs of a transfer atomically -- stock must never
    /// appear to leave the source warehouse without also arriving at the
    /// destination.
    pub fn post_transfer(&self, transfer: &StockTransfer) -> AppResult<[StockMovement; 2]> {
        transfer.validate()?;
        let [out, inbound] = transfer.build_movements();
        out.validate()?;
        inbound.validate()?;

        let (out_id, in_id) = self.db.with_transaction(|tx| {
            let insert = |m: &StockMovement| -> AppResult<i64> {
                tx.execute(
                    "INSERT INTO stock_movements (company_id, product_id, warehouse_id, movement_date, movement_type, quantity_delta, reference, transaction_id)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    params![m.company_id, m.product_id, m.warehouse_id, m.movement_date, m.movement_type.as_str(), m.quantity_delta.to_string(), m.reference, m.transaction_id],
                )?;
                Ok(tx.last_insert_rowid())
            };
            let out_id = insert(&out)?;
            let in_id = insert(&inbound)?;
            Ok((out_id, in_id))
        })?;

        Ok([StockMovement { id: Some(out_id), ..out }, StockMovement { id: Some(in_id), ..inbound }])
    }

    pub fn list_movements_for_product(&self, company_id: i64, product_id: i64) -> AppResult<Vec<StockMovement>> {
        self.db.with_connection(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, company_id, product_id, warehouse_id, movement_date, movement_type, quantity_delta, reference, transaction_id
                 FROM stock_movements WHERE company_id = ?1 AND product_id = ?2 ORDER BY movement_date, id",
            )?;
            let rows = stmt.query_map(params![company_id, product_id], map_movement)?;
            rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
        })
    }

    pub fn on_hand_quantity(&self, company_id: i64, product_id: i64, warehouse_id: i64) -> AppResult<Decimal> {
        let movements = self.list_movements_for_product(company_id, product_id)?;
        Ok(stock::on_hand_quantity(&movements, product_id, warehouse_id))
    }
}

fn map_warehouse(row: &Row) -> rusqlite::Result<Warehouse> {
    Ok(Warehouse {
        id: Some(row.get(0)?),
        company_id: row.get(1)?,
        name: row.get(2)?,
        code: row.get(3)?,
        address: row.get(4)?,
        is_active: row.get(5)?,
    })
}

fn map_category(row: &Row) -> rusqlite::Result<Category> {
    Ok(Category {
        id: Some(row.get(0)?),
        company_id: row.get(1)?,
        name: row.get(2)?,
        parent_id: row.get(3)?,
    })
}

fn map_product(row: &Row) -> rusqlite::Result<Product> {
    let cost_price_text: String = row.get(7)?;
    let sale_price_text: String = row.get(8)?;
    Ok(Product {
        id: Some(row.get(0)?),
        company_id: row.get(1)?,
        sku: row.get(2)?,
        name: row.get(3)?,
        barcode: row.get(4)?,
        category_id: row.get(5)?,
        unit_of_measure: row.get(6)?,
        cost_price: cost_price_text.parse().unwrap_or(Decimal::ZERO),
        sale_price: sale_price_text.parse().unwrap_or(Decimal::ZERO),
        track_inventory: row.get(9)?,
        is_active: row.get(10)?,
    })
}

fn map_movement(row: &Row) -> rusqlite::Result<StockMovement> {
    let movement_type_text: String = row.get(5)?;
    let movement_type = MovementType::parse(&movement_type_text).ok_or_else(|| {
        rusqlite::Error::FromSqlConversionFailure(5, rusqlite::types::Type::Text, "invalid movement_type".into())
    })?;
    let quantity_text: String = row.get(6)?;
    Ok(StockMovement {
        id: Some(row.get(0)?),
        company_id: row.get(1)?,
        product_id: row.get(2)?,
        warehouse_id: row.get(3)?,
        movement_date: row.get(4)?,
        movement_type,
        quantity_delta: quantity_text.parse().unwrap_or(Decimal::ZERO),
        reference: row.get(7)?,
        transaction_id: row.get(8)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::organization::companies::Company;
    use crate::backend::repositories::organization_repository::{self, OrganizationRepository};
    use chrono::NaiveDate;
    use rust_decimal_macros::dec;

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    fn repo_with_company() -> (InventoryRepository, i64) {
        let db = Database::open_in_memory().unwrap();
        db.run_migrations(organization_repository::MIGRATIONS).unwrap();
        db.run_migrations(crate::backend::repositories::accounting_repository::MIGRATIONS).unwrap();
        db.run_migrations(MIGRATIONS).unwrap();

        let org_repo = OrganizationRepository::new(db.clone());
        let company = org_repo.create_company(&Company::new("Acme Trading Co", "USD")).unwrap();

        (InventoryRepository::new(db), company.id.unwrap())
    }

    #[test]
    fn product_round_trips_by_id() {
        let (repo, company_id) = repo_with_company();
        let created = repo.create_product(&Product::new(company_id, "WIDGET-1", "Widget")).unwrap();
        let fetched = repo.get_product(created.id.unwrap()).unwrap();
        assert_eq!(fetched.sku, "WIDGET-1");
    }

    #[test]
    fn recording_movements_and_querying_on_hand_quantity() {
        let (repo, company_id) = repo_with_company();
        let product = repo.create_product(&Product::new(company_id, "WIDGET-1", "Widget")).unwrap();
        let warehouse = repo.create_warehouse(&Warehouse::new(company_id, "Main Warehouse", "MAIN")).unwrap();

        repo.record_movement(&StockMovement::new(
            company_id,
            product.id.unwrap(),
            warehouse.id.unwrap(),
            date(2026, 1, 1),
            MovementType::Purchase,
            dec!(100),
        ))
        .unwrap();
        repo.record_movement(&StockMovement::new(
            company_id,
            product.id.unwrap(),
            warehouse.id.unwrap(),
            date(2026, 1, 5),
            MovementType::Sale,
            dec!(-40),
        ))
        .unwrap();

        let on_hand = repo.on_hand_quantity(company_id, product.id.unwrap(), warehouse.id.unwrap()).unwrap();
        assert_eq!(on_hand, dec!(60));
    }

    #[test]
    fn post_transfer_moves_quantity_between_warehouses_atomically() {
        let (repo, company_id) = repo_with_company();
        let product = repo.create_product(&Product::new(company_id, "WIDGET-1", "Widget")).unwrap();
        let main = repo.create_warehouse(&Warehouse::new(company_id, "Main Warehouse", "MAIN")).unwrap();
        let overflow = repo.create_warehouse(&Warehouse::new(company_id, "Overflow Warehouse", "OVERFLOW")).unwrap();

        repo.record_movement(&StockMovement::new(company_id, product.id.unwrap(), main.id.unwrap(), date(2026, 1, 1), MovementType::Purchase, dec!(100)))
            .unwrap();

        let transfer = StockTransfer::new(company_id, product.id.unwrap(), main.id.unwrap(), overflow.id.unwrap(), date(2026, 1, 10), dec!(30));
        repo.post_transfer(&transfer).unwrap();

        assert_eq!(repo.on_hand_quantity(company_id, product.id.unwrap(), main.id.unwrap()).unwrap(), dec!(70));
        assert_eq!(repo.on_hand_quantity(company_id, product.id.unwrap(), overflow.id.unwrap()).unwrap(), dec!(30));
    }

    #[test]
    fn an_invalid_transfer_posts_neither_leg() {
        let (repo, company_id) = repo_with_company();
        let product = repo.create_product(&Product::new(company_id, "WIDGET-1", "Widget")).unwrap();
        let warehouse = repo.create_warehouse(&Warehouse::new(company_id, "Main Warehouse", "MAIN")).unwrap();

        let transfer = StockTransfer::new(company_id, product.id.unwrap(), warehouse.id.unwrap(), warehouse.id.unwrap(), date(2026, 1, 10), dec!(30));
        assert!(repo.post_transfer(&transfer).is_err());
        assert!(repo.list_movements_for_product(company_id, product.id.unwrap()).unwrap().is_empty());
    }
}
