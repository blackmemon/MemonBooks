pub mod expense_categories;
pub mod expense_claims;
// The frozen project structure intentionally nests expenses.rs under backend/expenses.
#[allow(clippy::module_inception)]
pub mod expenses;
