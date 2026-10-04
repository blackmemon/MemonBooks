//! Shared infrastructure for the two admin binaries under `src/bin/`:
//! `seed_demo_data` (generates a realistic demo dataset for manual
//! testing) and `clear_demo_data` (wipes it back out). Not part of the
//! frozen `backend/`/`database/`/`frontend/`/`configs/` structure in
//! `PROJECT_SPEC.md` Section 2 -- this is tooling around the app, not
//! app domain logic, so it lives directly under `src/` alongside
//! `main.rs`.
//!
//! Every demo record lives under one company whose name starts with
//! [`DEMO_TAG`], plus one system user/role (needed as a POS session's
//! `cashier_user_id`, which isn't company-scoped) whose display
//! name/role name also start with it. That single company-scoping
//! choice is what makes `clear_demo_data` simple and safe: nearly
//! every table in the schema traces back to a `company_id` (directly
//! or through a chain of foreign keys), so "delete everything under
//! the tagged company" reaches almost the whole dataset without
//! needing to tag every individual row.

use rusqlite::Connection;

use crate::backend::core::errors::AppResult;
use crate::database::database::Database;

/// Every demo record's human-visible name/reference is prefixed with
/// this, so it's obvious at a glance in the UI which records are demo
/// data -- on top of (not instead of) the company-scoping above.
pub const DEMO_TAG: &str = "DEMO-";

/// The one company all demo data lives under. Re-running the seeder
/// looks for a company with this exact name to decide whether demo
/// data already exists.
pub const DEMO_COMPANY_NAME: &str = "DEMO- Acme Wholesale Co";

/// The one system user demo POS sessions run under (`users`/`roles`
/// aren't company-scoped, so they need their own tag to be found by
/// `clear_demo_data`).
pub const DEMO_CASHIER_USERNAME: &str = "demo.cashier";
pub const DEMO_CASHIER_DISPLAY_NAME: &str = "DEMO- Demo Cashier";
pub const DEMO_ROLE_NAME: &str = "DEMO- Cashier";

/// What got removed, for the two binaries to report back.
#[derive(Debug, Clone, Copy, Default)]
pub struct ClearSummary {
    pub companies_removed: i64,
    pub users_removed: i64,
    pub roles_removed: i64,
}

/// Deletes every row belonging to a `DEMO-`-tagged company, plus the
/// `DEMO-`-tagged system user/role, in one transaction.
///
/// Foreign key enforcement is toggled off for the duration (SQLite
/// only allows that outside an active transaction, so it happens
/// immediately before `BEGIN` and immediately after `COMMIT`/on the
/// way out) rather than hand-ordering ~35 `DELETE` statements around
/// self-referencing columns (`accounts.parent_id`,
/// `categories.parent_id`, `transactions.reverses_transaction_id`) and
/// polymorphic, non-FK-enforced tables (`addresses`/`contacts`, keyed
/// by `(party_type, party_id)` rather than a real foreign key). This
/// is a complete, self-consistent subgraph being removed as a whole,
/// not a partial edit that could leave something dangling -- the
/// pragma toggle is exactly the sanctioned way to do that safely in
/// SQLite, and it fails closed: if any statement inside the
/// transaction errors, the whole thing rolls back and FK enforcement
/// is still restored on the way out.
pub fn clear_demo_data(db: &Database) -> AppResult<ClearSummary> {
    db.with_connection(|conn| {
        conn.pragma_update(None, "foreign_keys", false)?;
        let result = run_clear(conn);
        // Always restore enforcement, whether the clear succeeded or not.
        conn.pragma_update(None, "foreign_keys", true)?;
        result
    })
}

fn run_clear(conn: &mut Connection) -> AppResult<ClearSummary> {
    let tx = conn.transaction()?;
    let mut summary = ClearSummary::default();

    let company_filter = "SELECT id FROM companies WHERE name LIKE 'DEMO-%'";

    // Company-scoped, direct child of companies.
    for table in [
        "branches",
        "departments",
        "employees",
        "payroll_runs",
        "customers",
        "vendors",
        "accounts",
        "fiscal_periods",
        "transactions",
        "receivables",
        "payables",
        "quotations",
        "invoices",
        "credit_notes",
        "customer_payments",
        "pos_terminals",
        "pos_sales",
        "purchase_orders",
        "bills",
        "debit_notes",
        "vendor_payments",
        "warehouses",
        "categories",
        "products",
        "stock_movements",
        "bank_accounts",
        "bank_transactions",
        "expense_categories",
        "expenses",
        "expense_claims",
        "asset_categories",
        "assets",
        "tax_rates",
        "tax_transactions",
    ] {
        tx.execute(
            &format!("DELETE FROM {table} WHERE company_id IN ({company_filter})"),
            [],
        )?;
    }

    // One level indirect (child of a direct company-scoped table).
    let indirect: &[(&str, &str, &str)] = &[
        ("salary_structures", "employee_id", "employees"),
        ("attendance_records", "employee_id", "employees"),
        ("leave_requests", "employee_id", "employees"),
        ("payroll_entries", "payroll_run_id", "payroll_runs"),
        ("transaction_lines", "transaction_id", "transactions"),
        ("quotation_lines", "quotation_id", "quotations"),
        ("invoice_lines", "invoice_id", "invoices"),
        ("credit_note_lines", "credit_note_id", "credit_notes"),
        ("pos_sessions", "terminal_id", "pos_terminals"),
        ("pos_sale_lines", "pos_sale_id", "pos_sales"),
        ("pos_sale_payments", "pos_sale_id", "pos_sales"),
        (
            "purchase_order_lines",
            "purchase_order_id",
            "purchase_orders",
        ),
        ("bill_lines", "bill_id", "bills"),
        ("debit_note_lines", "debit_note_id", "debit_notes"),
        ("asset_transfers", "asset_id", "assets"),
    ];
    for (table, fk_column, parent_table) in indirect {
        tx.execute(&format!("DELETE FROM {table} WHERE {fk_column} IN (SELECT id FROM {parent_table} WHERE company_id IN ({company_filter}))"), [])?;
    }

    // Two levels indirect.
    tx.execute(
        &format!(
            "DELETE FROM salary_components WHERE salary_structure_id IN (
                SELECT id FROM salary_structures WHERE employee_id IN (
                    SELECT id FROM employees WHERE company_id IN ({company_filter})
                )
            )"
        ),
        [],
    )?;
    tx.execute(
        &format!(
            "DELETE FROM cash_drawer_events WHERE session_id IN (
                SELECT id FROM pos_sessions WHERE terminal_id IN (
                    SELECT id FROM pos_terminals WHERE company_id IN ({company_filter})
                )
            )"
        ),
        [],
    )?;

    // Polymorphic, not a real foreign key: party_id matches either
    // customers.id or vendors.id depending on party_type.
    for table in ["addresses", "contacts"] {
        tx.execute(
            &format!(
                "DELETE FROM {table} WHERE
                    (party_type = 'customer' AND party_id IN (SELECT id FROM customers WHERE company_id IN ({company_filter})))
                 OR (party_type = 'vendor' AND party_id IN (SELECT id FROM vendors WHERE company_id IN ({company_filter})))"
            ),
            [],
        )?;
    }

    summary.companies_removed = tx.execute(
        &format!("DELETE FROM companies WHERE id IN ({company_filter})"),
        [],
    )? as i64;

    // Not company-scoped at all: the demo system user/role, found by tag.
    tx.execute(
        "DELETE FROM sessions WHERE user_id IN (SELECT id FROM users WHERE display_name LIKE 'DEMO-%')",
        [],
    )?;
    summary.users_removed =
        tx.execute("DELETE FROM users WHERE display_name LIKE 'DEMO-%'", [])? as i64;
    tx.execute(
        "DELETE FROM role_permissions WHERE role_id IN (SELECT id FROM roles WHERE name LIKE 'DEMO-%')",
        [],
    )?;
    summary.roles_removed = tx.execute("DELETE FROM roles WHERE name LIKE 'DEMO-%'", [])? as i64;

    tx.commit()?;
    Ok(summary)
}
