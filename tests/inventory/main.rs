//! Integration test for inventory (Development Order step 11): a
//! purchase movement, then a transfer between warehouses, verified via
//! on-hand quantity per warehouse.

use chrono::NaiveDate;
use rust_decimal_macros::dec;

use memonbooks::backend::inventory::products::Product;
use memonbooks::backend::inventory::stock_movements::{MovementType, StockMovement};
use memonbooks::backend::inventory::transfers::StockTransfer;
use memonbooks::backend::inventory::warehouses::Warehouse;
use memonbooks::backend::organization::companies::Company;
use memonbooks::backend::repositories::accounting_repository;
use memonbooks::backend::repositories::inventory_repository::{self, InventoryRepository};
use memonbooks::backend::repositories::organization_repository::{self, OrganizationRepository};
use memonbooks::database::database::Database;

fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
}

#[test]
fn purchase_then_transfer_moves_quantity_between_warehouses() {
    let db = Database::open_in_memory().unwrap();
    db.run_migrations(organization_repository::MIGRATIONS)
        .unwrap();
    db.run_migrations(accounting_repository::MIGRATIONS)
        .unwrap();
    db.run_migrations(inventory_repository::MIGRATIONS).unwrap();

    let org_repo = OrganizationRepository::new(db.clone());
    let company_id = org_repo
        .create_company(&Company::new("Acme Trading Co", "USD"))
        .unwrap()
        .id
        .unwrap();

    let repo = InventoryRepository::new(db);
    let product = repo
        .create_product(&Product::new(company_id, "WIDGET-1", "Widget"))
        .unwrap();
    let main = repo
        .create_warehouse(&Warehouse::new(company_id, "Main Warehouse", "MAIN"))
        .unwrap();
    let overflow = repo
        .create_warehouse(&Warehouse::new(
            company_id,
            "Overflow Warehouse",
            "OVERFLOW",
        ))
        .unwrap();

    repo.record_movement(&StockMovement::new(
        company_id,
        product.id.unwrap(),
        main.id.unwrap(),
        date(2026, 1, 1),
        MovementType::Purchase,
        dec!(200),
    ))
    .unwrap();

    let transfer = StockTransfer::new(
        company_id,
        product.id.unwrap(),
        main.id.unwrap(),
        overflow.id.unwrap(),
        date(2026, 1, 10),
        dec!(75),
    );
    repo.post_transfer(&transfer).unwrap();

    assert_eq!(
        repo.on_hand_quantity(company_id, product.id.unwrap(), main.id.unwrap())
            .unwrap(),
        dec!(125)
    );
    assert_eq!(
        repo.on_hand_quantity(company_id, product.id.unwrap(), overflow.id.unwrap())
            .unwrap(),
        dec!(75)
    );

    // A sale reduces the selling warehouse only.
    repo.record_movement(&StockMovement::new(
        company_id,
        product.id.unwrap(),
        overflow.id.unwrap(),
        date(2026, 1, 15),
        MovementType::Sale,
        dec!(-20),
    ))
    .unwrap();
    assert_eq!(
        repo.on_hand_quantity(company_id, product.id.unwrap(), overflow.id.unwrap())
            .unwrap(),
        dec!(55)
    );
    assert_eq!(
        repo.on_hand_quantity(company_id, product.id.unwrap(), main.id.unwrap())
            .unwrap(),
        dec!(125)
    );
}
