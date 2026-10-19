// Crate root. backend/, configs/, database/ and frontend/ live outside
// src/ per the frozen structure in PROJECT_SPEC.md Section 2, so they are
// wired in with explicit #[path] attributes instead of Cargo's default
// src/-relative module discovery.

#[path = "../backend/mod.rs"]
pub mod backend;

#[path = "../configs/mod.rs"]
pub mod configs;

#[path = "../database/mod.rs"]
pub mod database;

#[path = "../frontend/mod.rs"]
pub mod frontend;

pub mod demo_data;

/// Every module that owns schema contributes its `MIGRATIONS` here, in the
/// order its Development Order step was implemented. `Database::run_migrations`
/// re-sorts by version anyway, so this list only has to be complete, not
/// ordered. Shared by `src/main.rs` and the `src/bin/*.rs` admin tools
/// (`seed_demo_data`, `clear_demo_data`), which need the same schema
/// available before touching the database -- a single copy here means a
/// module added later can't be forgotten in one of the three.
pub fn all_migrations() -> Vec<database::database::Migration> {
    let mut migrations = Vec::new();
    migrations.extend_from_slice(backend::repositories::document_repository::MIGRATIONS);
    migrations.extend_from_slice(backend::repositories::organization_repository::MIGRATIONS);
    migrations.extend_from_slice(backend::repositories::user_repository::MIGRATIONS);
    migrations.extend_from_slice(backend::repositories::party_repository::MIGRATIONS);
    migrations.extend_from_slice(backend::repositories::accounting_repository::MIGRATIONS);
    migrations.extend_from_slice(backend::repositories::sales_repository::MIGRATIONS);
    migrations.extend_from_slice(backend::repositories::purchase_repository::MIGRATIONS);
    migrations.extend_from_slice(backend::repositories::inventory_repository::MIGRATIONS);
    migrations.extend_from_slice(backend::repositories::banking_repository::MIGRATIONS);
    migrations.extend_from_slice(backend::repositories::expense_repository::MIGRATIONS);
    migrations.extend_from_slice(backend::repositories::fixed_asset_repository::MIGRATIONS);
    migrations.extend_from_slice(backend::repositories::tax_repository::MIGRATIONS);
    migrations.extend_from_slice(backend::audit::audit_logs::MIGRATIONS);
    migrations.extend_from_slice(backend::warehouse_layout::repository::MIGRATIONS);
    migrations
}
