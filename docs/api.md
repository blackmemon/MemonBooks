# MemonBooks – Internal API (service and repository layer)

**MemonBooks has no HTTP/REST API.** It is an offline desktop application: there is no server, no listening socket, no endpoint, no authentication token over a network, and no HTTP, gRPC or WebSocket crate in `Cargo.toml`. The files `backend/services/api.rs` and `backend/services/commands.rs` are one-line placeholders with no code. This document therefore describes the **in-process API** that the UI uses – the service and repository layer – and the **file formats** the application reads and writes.

Related documents: [README](../README.md) · [Architecture](architecture.md) · [Database](database.md) · [Security](security.md) · [Feature development](feature-development.md)

## Table of contents

1. [Calling convention](#1-calling-convention)
2. [Error type](#2-error-type)
3. [Services](#3-services)
4. [Repositories](#4-repositories)
5. [Domain helpers used by callers](#5-domain-helpers-used-by-callers)
6. [Import and export formats](#6-import-and-export-formats)
7. [Files written by the application](#7-files-written-by-the-application)
8. [Open questions](#8-open-questions)

---

## 1. Calling convention

```rust
use memonbooks::backend::services::accounting_service::AccountingService;
use memonbooks::database::database::Database;

let db = Database::open(&path)?;                         // or Database::open_in_memory()
db.run_migrations(&memonbooks::all_migrations())?;
let service = AccountingService::new(db.clone());        // services are cheap to clone
let accounts = service.list_accounts(company_id)?;       // AppResult<Vec<Account>>
```

* Every service is `Service::new(db: Database)`; every method is synchronous and returns `AppResult<T>`.
* Services are scoped per company by passing `company_id: i64`. Creating methods return the stored entity with its new `id`.
* Money and quantities are `rust_decimal::Decimal`; dates are `chrono::NaiveDate`.
* Only `AuthService` checks permissions (`users.manage`); other services trust the caller. See [security.md](security.md).
* The integration tests in `tests/` call this layer directly (see [testing-qa.md](testing-qa.md)).

## 2. Error type

`backend/core/errors.rs`:

| Variant | Meaning | Typical source |
|---------|---------|----------------|
| `Validation(String)` | input or business rule rejected (also used for permission denial and bad credentials) | `Validate::validate`, `transaction_engine`, `auth_service` |
| `NotFound(String)` | entity does not exist | `get_*` |
| `Database(rusqlite::Error)` | SQLite failure; `is_constraint_violation()` is true for FK/unique violations | repositories |
| `Migration(String)` | duplicate version, version/name clash, failing SQL | `run_migrations` |
| `Io { path, source }` | filesystem failure | open DB, create dirs, import |
| `Config(String)` | no data directory could be determined | `AppConfig::load` |
| `Internal(String)` | unexpected state (poisoned mutex, hash failure, malformed stored hash) | various |

Views display the `Display` text of the error inline.

## 3. Services

All under `backend/services/`. `list_*`/`get_*` read; the rest write. `→` shows the success type inside `AppResult`.

### 3.1 `AuthService` (`auth_service.rs`)

| Method | Input | Output | Errors / notes |
|--------|-------|--------|----------------|
| `login_with_credentials` | `username: &str, password: &str` | `AuthOutcome::Authenticated(User)` or `TwoFactorRequired(User)` | `Validation("invalid username or password")` for unknown, inactive or wrong – deliberately identical |
| `login_with_two_factor_code` | `user: &User, code: &str` | `User` | `Validation("invalid two-factor code")`; `Internal` if 2FA is on without a secret |
| `change_password` | `user: &User, current: &str, new: &str` | updated `User` (`must_change_password = false`) | `Validation` if current is wrong or new is shorter than 10 characters |
| `has_permission` | `user: &User, permission: &str` | `bool` | looks up the user's role |
| `list_users` / `list_roles` / `list_other_active_users(exclude_user_id)` | – | `Vec<User>` / `Vec<Role>` | |
| `create_user` | `acting_user, username, display_name, role_id` | `(User, String)` – the one-time temporary password | requires `users.manage`; new user has `must_change_password` |
| `update_user` | `acting_user, &User` | `()` | requires `users.manage` |
| `reset_password` | `acting_user, target_user_id` | temporary password `String` | requires `users.manage` |
| `request_password_reset` | `username: &str` | `()` | sets `is_reset_requested`; silent for unknown users |

### 3.2 `AccountingService`

`list_accounts(company_id)`, `create_account(&Account)`, `update_account(&Account)`, `delete_account(id)`, `trial_balance_as_of(company_id, NaiveDate) → TrialBalance`, `post_transaction(&Transaction) → Transaction`, `list_journal_entries(company_id) → Vec<Transaction>` (manual journals only), `reverse_transaction(original_id, reversal_date) → Transaction`, `create_fiscal_period(&FiscalPeriod)`.

`post_transaction` errors (`Validation`): unbalanced; not a draft; fiscal period of another company; date outside the period; period not open. Atomic: transaction, lines and audit row commit together.

### 3.3 `PartyService`

`list_customers`, `create_customer`, `update_customer`, `delete_customer`, `list_vendors`, `create_vendor`, `update_vendor`, `delete_vendor` – each takes a company id or the entity and returns the entity / `()`. Vendors carry the 905 fields (website, addresses, payment method, description), custom fields and attachment records.

### 3.4 `SalesService`

`list_customers`, `list_gl_accounts`, `list_invoices`, `create_invoice`, `get_invoice`, `update_invoice`, `delete_invoice`, `post_invoice(invoice_id, ar_account_id) → (Invoice, Transaction, Receivable)`, `record_customer_payment(&CustomerPayment, cash_account_id, ar_account_id)`, `record_payment_for_invoice(invoice_id, date, amount, cash_account_id, ar_account_id)`, `invoice_remaining_balance(invoice_id) → Decimal`, `void_invoice`, `create_and_post_credit_note_for_invoice(invoice_id, date, revenue_account_id, description, amount, ar_account_id)`, and POS: `list_terminals`, `create_terminal`, `list_products`, `open_session`, `post_pos_sale(&PosSale, &[(PaymentMethod, account_id)]) → (PosSale, Transaction)`.

Payments larger than the party's open balance are rejected before anything posts (CHANGELOG, *Fixed*).

### 3.5 `PurchaseService`

Mirror of sales: `list_vendors`, `vendor_ledger(company_id, vendor_id) → VendorLedger`, `list_gl_accounts`, `list_bills`, `create_bill`, `get_bill`, `update_bill`, `delete_bill`, `void_bill`, `post_bill(bill_id, ap_account_id) → (Bill, Transaction, Payable)`, `create_and_post_debit_note_for_bill(...)`, `record_vendor_payment(&VendorPayment, ap_account_id, cash_account_id)`, `record_payment_for_bill(...)`, `bill_remaining_balance(bill_id) → Decimal`.

### 3.6 Other services

| Service | Methods |
|---------|---------|
| `BankingService` | `list_bank_accounts`, `create/update/delete_bank_account`, `remove_bank_account(id) → Removal::{Deleted, Archived}` (archives when the account has history), `book_balance(id, as_of) → Decimal`, `bank_ledger`, `bank_ledger_summary`, `list_gl_accounts`, `company_currency` |
| `InventoryService` | `list/create/update/delete` for products and warehouses |
| `WarehouseService` (`warehouse_layout/service.rs`) | `list_companies`, `list_products`, `list_overviews`, `save_warehouse(&WarehouseDraft) → i64`, `remove_warehouse(company_id, id) → Removal`, `place_stock(bin_id, product_id, quantity)` |
| `ExpenseService` | `create_category`, `record_expense → (Expense, Transaction)`, `create_claim`, `approve_claim`, `reject_claim`, `reimburse_claim(id, payment_account_id, date)` |
| `FixedAssetService` | `list_assets`, `get_asset`, `record_depreciation(asset_id, date) → Option<Transaction>` |
| `TaxService` | `list/create/update/delete_tax_rate`, `list_gl_accounts` |
| `OrganizationService` | companies (`get/update/list/create_company` – creating also adds an open fiscal period for the current year), employees (`list/create/update/delete`), `create_salary_structure`, `current_salary_structure(employee_id, as_of)`, `post_payroll_run(&PayrollRun, salary_expense_account_id, deductions_payable_account_id, net_pay_account_id) → (PayrollRun, Transaction)` |
| `ReportService` | `general_ledger(company, account, as_of)`, `party_open_items(company, party_type, party, as_of)`, `trial_balance_as_of`, `revenue_and_expense_balances_as_of`, `list_open_receivables`, `list_open_payables`, `list_accounts` |
| `DocumentService` | `list(company, Kind)`, `save(&Document)`, `delete(company, &[id]) → usize`, `save_template`, `templates`, `template`, `import(company, Kind, rows) → (imported, skipped)`, `stock_report`, `account_tree`, `payroll_for`, `generate_payroll(company, period, employee_ids) → (created, skipped)` |

`transaction_service.rs`, `api.rs`, `commands.rs` are empty.

## 4. Repositories

`backend/repositories/` holds the SQL: `AccountingRepository`, `BankingRepository`, `DocumentRepository`, `ExpenseRepository`, `FixedAssetRepository`, `InventoryRepository`, `OrganizationRepository`, `PartyRepository`, `PurchaseRepository`, `SalesRepository`, `TaxRepository`, `UserRepository` (plus `master_profiles.rs`, and `WarehouseLayoutRepository` in `backend/warehouse_layout/`). Each:

* is constructed with `Repo::new(db: Database)` and is `Clone`;
* owns the `MIGRATIONS` of its tables ([database.md](database.md#4-migration-list));
* uses `rusqlite` **parameterised** statements (`?1`, `params![]`);
* performs multi-table writes in a single `rusqlite::Transaction`;
* maps FK violations to `Validation` messages (for example deleting a referenced record).

Views call repositories directly in a few places (bank accounts, tax rates, products, companies, login) – see [architecture.md](architecture.md#4-layers-and-dependency-rule).

## 5. Domain helpers used by callers

| Item | Location | Contract |
|------|----------|----------|
| `Validate` trait | `core/validation.rs` | `fn validate(&self) -> AppResult<()>`; helpers `require_non_empty`, `require_max_length`, `require_non_negative`, `require_positive` |
| `Transaction::new(..).with_line(..)`, `total_debits()`, `total_credits()`, `is_balanced()`, `build_reversal(date)` | `transactions/transaction.rs` | balanced check on `Decimal` |
| `transaction_engine::validate_for_posting(&Transaction, &FiscalPeriod)` | `transactions/transaction_engine.rs` | see 3.2 |
| `TransactionStatus` | `Draft`, `Posted`, `Reversed` (`"draft"`, `"posted"`, `"reversed"`) | |
| `FiscalPeriodStatus` | `Open`, `Closed`, `Locked`; allowed transitions Open⇄Closed, Closed→Locked | |
| `hash_password` / `verify_password` | `core/auth/passwords.rs` | Argon2id (crate default), random salt, minimum 10 characters |
| `Session::new(user_id)` | `core/auth/sessions.rs` | 32 random bytes hex token, valid 12 hours |
| `two_factor::generate_secret` / `verify_code` | `core/auth/two_factor.rs` | TOTP SHA-1, 6 digits, 30 s, ±1 step |
| `sign_license_key` / `verify_license_key`, `trial_status`, `licensed_status` | `core/auth/activation.rs`, `licensing.rs` | HMAC-SHA256 signed key; 30-day trial; **not called by the app** |
| `create_backup(db, dest)` / `restore_from_backup(backup, target)` / `is_backup_due` | `backend/backup/` | `VACUUM INTO`; see [database.md](database.md#8-backup-and-restore) |

## 6. Import and export formats

### 6.1 CSV export (implemented, used by screens)

`database::exports::csv::export_csv(headers: &[&str], rows: &[Vec<String>]) -> String`: RFC 4180, CRLF line endings, fields quoted only when they contain a comma, quote or newline, quotes doubled. Written by the list and report screens through `Ctx::export_path(file)` to `<data dir>/exports/`. Files seen in the code: `vendors_export.csv`, `balance_sheet.csv`, `profit_and_loss.csv`, plus bank account, customer, warehouse, tax rate, trial balance, employee, bill, invoice and product exports. Each report module has `to_rows() -> (Vec<&'static str>, Vec<Vec<String>>)` producing the headers and rows.

### 6.2 XLSX export (library only)

`export_xlsx(sheet_name: &str, headers: &[&str], rows: &[Vec<String>], path: &Path) -> AppResult<()>` via `rust_xlsxwriter`: one worksheet, bold header row. No screen calls it yet.

### 6.3 PDF export (library only)

`export_pdf(title: &str, headers: &[&str], rows: &[Vec<String>], path: &Path) -> AppResult<()>` via `printpdf`: built-in Helvetica / Helvetica-Bold at 10 pt, bold title and header row, plain table. No screen calls it yet. Because built-in PDF fonts are Latin-1 only, Urdu/Arabic text cannot be exported.

### 6.4 CSV / XLSX import (library only)

`database::exports::import::read(path: &Path) -> AppResult<Vec<Vec<String>>>`:

* extension `.csv` → UTF-8 (BOM allowed), quotes supported, otherwise treated as XLSX;
* XLSX: reads `xl/sharedStrings.xml` and `xl/worksheets/sheet1.xml` (first sheet only); no formulas are evaluated;
* limits: file ≤ 20 MB and each XML part ≤ 20 MB (`Validation` otherwise); only predefined XML entities accepted.

`DocumentService::import(company, kind, rows)` matches header cells case-insensitively and in any order, ignores unknown columns, skips rows whose first column is blank, validates everything first, and saves all or fails: returns `(imported, skipped)`. Only master kinds are importable. Expected headers:

| Kind | Headers |
|------|---------|
| Vendor | Vendor Name, Company Name, Phone, Email, Website, Mailing Address, Shipping Address, Payment Terms, Payment Method, Notes/Memo |
| Customer | Customer Name, Company Name, Phone, Email, NTN, STRN, Mailing Address, Shipping Address, Description |
| Item | Item Name, Item Type, Category, Sub-Category, Unit of Measure, Purchase Price, Sale Price, Description |
| Location | Location Name, Location Type, Contact Person, Phone, Email, Capacity, Mailing Address, Shipping Address, Description |
| Account | Account Name, Account Type, Parent Account, Description |

No screen calls `import` in the current views (`tests/workflows.rs` covers it).

### 6.5 Database backup

A SQLite database file produced by `VACUUM INTO` (see [database.md](database.md#8-backup-and-restore)).

## 7. Files written by the application

| Path (under the data directory) | Written by | Content |
|---------------------------------|-----------|---------|
| `memonbooks.sqlite3` | `Database` | all data |
| `logs/memonbooks.log` | `logging::init` | log lines; the **first-run admin password is logged here** |
| `exports/*.csv` | list/report screens | CSV |
| `uploads/<nanos>-<n>-<original name>` | vendor attachment button (`std::fs::copy`) | user files |
| `theme_preference.txt`, `font_scale_preference.txt` | settings / sidebar | plain text |

## 8. Open questions

1. Should the service layer enforce permissions for all write methods, not only user management?
2. Should the placeholder files `api.rs` and `commands.rs` be removed?
3. Is a scripting or CLI interface (beyond `seed_demo_data` / `clear_demo_data`) required?
4. Which screens should expose XLSX/PDF export and CSV/XLSX import?
5. The exact CSV column order of each exported list is defined per view (`*_HEADERS` constants); a consolidated reference has not been written.
