"""
setup_db.py — Initialize master.db and user.db with all tables.
Creates the default admin user, seeds base data (COA roots, system config).
Run ONCE after generate_keys.py. Safe to re-run (IF NOT EXISTS throughout).
"""

import os
import sys
import uuid
import getpass
from datetime import datetime, timezone
from pathlib import Path

# Allow imports from project root
sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from dotenv import load_dotenv
load_dotenv()

from backend.config import settings
from backend.security.device_fingerprint import get_device_hash
from backend.security.encryption import derive_master_db_key, derive_user_db_key
from backend.security.auth import hash_password

try:
    from sqlcipher3 import dbapi2 as sqlcipher
    _CIPHER = True
except ImportError:
    import sqlite3 as sqlcipher  # type: ignore
    _CIPHER = False


def _open_db(path: Path, hex_key: str) -> sqlcipher.Connection:
    conn = sqlcipher.connect(str(path))
    if _CIPHER:
        conn.execute(f"PRAGMA key = \"x'{hex_key}'\";")
    conn.execute("PRAGMA journal_mode=WAL;")
    conn.execute("PRAGMA foreign_keys=ON;")
    conn.row_factory = sqlcipher.Row
    return conn


def _now() -> str:
    return datetime.now(timezone.utc).isoformat()


def _uuid() -> str:
    return str(uuid.uuid4())


# ──────────────────────────────────────────────────────────────────────────────
# MASTER DB SCHEMA
# ──────────────────────────────────────────────────────────────────────────────

MASTER_DDL = """

-- System Configuration
CREATE TABLE IF NOT EXISTS system_config (
    key         TEXT PRIMARY KEY,
    value       TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);

-- Users
CREATE TABLE IF NOT EXISTS users (
    id                  TEXT PRIMARY KEY,
    username            TEXT NOT NULL UNIQUE,
    email               TEXT,
    full_name           TEXT NOT NULL,
    password_hash       TEXT NOT NULL,
    role                TEXT NOT NULL CHECK(role IN ('admin','manager','user','viewer')),
    device_hash         TEXT,
    is_active           INTEGER NOT NULL DEFAULT 1,
    is_deleted          INTEGER NOT NULL DEFAULT 0,
    failed_attempts     INTEGER NOT NULL DEFAULT 0,
    locked_until        TEXT,
    last_login          TEXT,
    password_changed_at TEXT,
    created_by          TEXT,
    created_at          TEXT NOT NULL,
    updated_at          TEXT NOT NULL
);

-- Per-user permission overrides (grant or revoke specific module+action)
CREATE TABLE IF NOT EXISTS user_permissions (
    id          TEXT PRIMARY KEY,
    user_id     TEXT NOT NULL REFERENCES users(id),
    module      TEXT NOT NULL,
    action      TEXT NOT NULL,
    is_granted  INTEGER NOT NULL DEFAULT 1,
    created_by  TEXT,
    created_at  TEXT NOT NULL,
    UNIQUE(user_id, module, action)
);

-- Approved mobile devices (IMEI + MAC whitelist)
CREATE TABLE IF NOT EXISTS approved_devices (
    id              TEXT PRIMARY KEY,
    user_id         TEXT REFERENCES users(id),
    device_name     TEXT,
    platform        TEXT NOT NULL CHECK(platform IN ('android','ios','windows','linux','macos')),
    device_hash     TEXT NOT NULL UNIQUE,
    imei_1          TEXT,
    imei_2          TEXT,
    mac_address     TEXT,
    is_active       INTEGER NOT NULL DEFAULT 1,
    approved_by     TEXT,
    approved_at     TEXT NOT NULL,
    last_seen_at    TEXT,
    created_at      TEXT NOT NULL
);

-- Audit log
CREATE TABLE IF NOT EXISTS audit_log (
    id          TEXT PRIMARY KEY,
    user_id     TEXT,
    username    TEXT,
    action      TEXT NOT NULL,
    module      TEXT NOT NULL,
    record_id   TEXT,
    table_name  TEXT,
    old_values  TEXT,
    new_values  TEXT,
    ip_address  TEXT,
    device_hash TEXT,
    created_at  TEXT NOT NULL
);

-- Fiscal Years
CREATE TABLE IF NOT EXISTS fiscal_years (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    start_date  TEXT NOT NULL,
    end_date    TEXT NOT NULL,
    is_active   INTEGER NOT NULL DEFAULT 1,
    is_closed   INTEGER NOT NULL DEFAULT 0,
    closed_by   TEXT,
    closed_at   TEXT,
    created_by  TEXT,
    created_at  TEXT NOT NULL
);

-- Currencies
CREATE TABLE IF NOT EXISTS currencies (
    id              TEXT PRIMARY KEY,
    code            TEXT NOT NULL UNIQUE,
    name            TEXT NOT NULL,
    symbol          TEXT NOT NULL,
    exchange_rate   REAL NOT NULL DEFAULT 1.0,
    is_base         INTEGER NOT NULL DEFAULT 0,
    is_active       INTEGER NOT NULL DEFAULT 1,
    updated_at      TEXT NOT NULL,
    created_at      TEXT NOT NULL
);

-- Chart of Accounts
CREATE TABLE IF NOT EXISTS chart_of_accounts (
    id              TEXT PRIMARY KEY,
    code            TEXT NOT NULL UNIQUE,
    name            TEXT NOT NULL,
    account_type    TEXT NOT NULL CHECK(account_type IN ('asset','liability','equity','income','expense')),
    parent_id       TEXT REFERENCES chart_of_accounts(id),
    level           INTEGER NOT NULL DEFAULT 1,
    is_control      INTEGER NOT NULL DEFAULT 0,
    is_active       INTEGER NOT NULL DEFAULT 1,
    currency_id     TEXT REFERENCES currencies(id),
    description     TEXT,
    created_by      TEXT,
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL
);

-- Tax Configuration
CREATE TABLE IF NOT EXISTS tax_config (
    id              TEXT PRIMARY KEY,
    name            TEXT NOT NULL,
    tax_type        TEXT NOT NULL CHECK(tax_type IN ('GST','WHT','FED','SED','custom')),
    rate            REAL NOT NULL,
    account_id      TEXT REFERENCES chart_of_accounts(id),
    applies_to      TEXT NOT NULL CHECK(applies_to IN ('purchases','sales','both')),
    is_active       INTEGER NOT NULL DEFAULT 1,
    created_by      TEXT,
    created_at      TEXT NOT NULL
);

-- Payment Terms
CREATE TABLE IF NOT EXISTS payment_terms (
    id              TEXT PRIMARY KEY,
    name            TEXT NOT NULL,
    due_days        INTEGER NOT NULL DEFAULT 0,
    discount_days   INTEGER,
    discount_pct    REAL,
    is_active       INTEGER NOT NULL DEFAULT 1,
    created_at      TEXT NOT NULL
);

-- Units of Measure
CREATE TABLE IF NOT EXISTS uom (
    id          TEXT PRIMARY KEY,
    code        TEXT NOT NULL UNIQUE,
    name        TEXT NOT NULL,
    is_active   INTEGER NOT NULL DEFAULT 1,
    created_at  TEXT NOT NULL
);

-- UOM Conversions
CREATE TABLE IF NOT EXISTS uom_conversions (
    id              TEXT PRIMARY KEY,
    from_uom_id     TEXT NOT NULL REFERENCES uom(id),
    to_uom_id       TEXT NOT NULL REFERENCES uom(id),
    factor          REAL NOT NULL,
    created_at      TEXT NOT NULL,
    UNIQUE(from_uom_id, to_uom_id)
);

-- Inventory Locations
CREATE TABLE IF NOT EXISTS inventory_locations (
    id          TEXT PRIMARY KEY,
    code        TEXT NOT NULL UNIQUE,
    name        TEXT NOT NULL,
    address     TEXT,
    is_active   INTEGER NOT NULL DEFAULT 1,
    created_by  TEXT,
    created_at  TEXT NOT NULL
);

-- Vendors
CREATE TABLE IF NOT EXISTS vendors (
    id              TEXT PRIMARY KEY,
    code            TEXT NOT NULL UNIQUE,
    name            TEXT NOT NULL,
    ntn             TEXT,
    strn            TEXT,
    cnic            TEXT,
    phone           TEXT,
    email           TEXT,
    address         TEXT,
    city            TEXT,
    bank_name       TEXT,
    bank_iban       TEXT,
    bank_account    TEXT,
    wht_rate        REAL DEFAULT 0,
    wht_account_id  TEXT REFERENCES chart_of_accounts(id),
    credit_limit    REAL DEFAULT 0,
    credit_days     INTEGER DEFAULT 30,
    payment_term_id TEXT REFERENCES payment_terms(id),
    currency_id     TEXT REFERENCES currencies(id),
    account_id      TEXT REFERENCES chart_of_accounts(id),
    is_active       INTEGER NOT NULL DEFAULT 1,
    is_deleted      INTEGER NOT NULL DEFAULT 0,
    created_by      TEXT,
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL
);

-- Customers
CREATE TABLE IF NOT EXISTS customers (
    id              TEXT PRIMARY KEY,
    code            TEXT NOT NULL UNIQUE,
    name            TEXT NOT NULL,
    ntn             TEXT,
    strn            TEXT,
    cnic            TEXT,
    phone           TEXT,
    email           TEXT,
    address         TEXT,
    city            TEXT,
    bank_iban       TEXT,
    credit_limit    REAL DEFAULT 0,
    credit_days     INTEGER DEFAULT 30,
    payment_term_id TEXT REFERENCES payment_terms(id),
    currency_id     TEXT REFERENCES currencies(id),
    account_id      TEXT REFERENCES chart_of_accounts(id),
    is_active       INTEGER NOT NULL DEFAULT 1,
    is_deleted      INTEGER NOT NULL DEFAULT 0,
    created_by      TEXT,
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL
);

-- Items (Products/Inventory)
CREATE TABLE IF NOT EXISTS items (
    id                  TEXT PRIMARY KEY,
    code                TEXT NOT NULL UNIQUE,
    name                TEXT NOT NULL,
    description         TEXT,
    item_type           TEXT NOT NULL CHECK(item_type IN ('product','service','raw_material')),
    uom_id              TEXT REFERENCES uom(id),
    purchase_uom_id     TEXT REFERENCES uom(id),
    sale_uom_id         TEXT REFERENCES uom(id),
    purchase_account_id TEXT REFERENCES chart_of_accounts(id),
    sale_account_id     TEXT REFERENCES chart_of_accounts(id),
    inventory_account_id TEXT REFERENCES chart_of_accounts(id),
    purchase_tax_id     TEXT REFERENCES tax_config(id),
    sale_tax_id         TEXT REFERENCES tax_config(id),
    standard_cost       REAL DEFAULT 0,
    standard_price      REAL DEFAULT 0,
    min_stock           REAL DEFAULT 0,
    max_stock           REAL DEFAULT 0,
    is_active           INTEGER NOT NULL DEFAULT 1,
    is_deleted          INTEGER NOT NULL DEFAULT 0,
    created_by          TEXT,
    created_at          TEXT NOT NULL,
    updated_at          TEXT NOT NULL
);

-- Item Price Lists
CREATE TABLE IF NOT EXISTS item_price_lists (
    id              TEXT PRIMARY KEY,
    item_id         TEXT NOT NULL REFERENCES items(id),
    customer_id     TEXT REFERENCES customers(id),
    min_qty         REAL DEFAULT 1,
    price           REAL NOT NULL,
    valid_from      TEXT,
    valid_to        TEXT,
    created_by      TEXT,
    created_at      TEXT NOT NULL
);

-- Stock Ledger
CREATE TABLE IF NOT EXISTS stock_ledger (
    id              TEXT PRIMARY KEY,
    item_id         TEXT NOT NULL REFERENCES items(id),
    location_id     TEXT REFERENCES inventory_locations(id),
    transaction_type TEXT NOT NULL,
    reference_type  TEXT,
    reference_id    TEXT,
    qty_in          REAL DEFAULT 0,
    qty_out         REAL DEFAULT 0,
    qty_balance     REAL NOT NULL,
    unit_cost       REAL DEFAULT 0,
    created_by      TEXT,
    created_at      TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_stock_ledger_item ON stock_ledger(item_id);
CREATE INDEX IF NOT EXISTS idx_stock_ledger_location ON stock_ledger(location_id);

-- Purchase Orders Header
CREATE TABLE IF NOT EXISTS purchase_orders (
    id              TEXT PRIMARY KEY,
    po_number       TEXT NOT NULL UNIQUE,
    vendor_id       TEXT NOT NULL REFERENCES vendors(id),
    location_id     TEXT REFERENCES inventory_locations(id),
    currency_id     TEXT REFERENCES currencies(id),
    exchange_rate   REAL DEFAULT 1,
    order_date      TEXT NOT NULL,
    delivery_date   TEXT,
    payment_term_id TEXT REFERENCES payment_terms(id),
    status          TEXT NOT NULL DEFAULT 'draft' CHECK(status IN ('draft','submitted','approved','posted','closed','cancelled')),
    subtotal        REAL DEFAULT 0,
    tax_amount      REAL DEFAULT 0,
    discount_amount REAL DEFAULT 0,
    total_amount    REAL DEFAULT 0,
    notes           TEXT,
    approved_by     TEXT,
    approved_at     TEXT,
    posted_by       TEXT,
    posted_at       TEXT,
    created_by      TEXT,
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL
);

-- Purchase Order Lines
CREATE TABLE IF NOT EXISTS purchase_order_lines (
    id              TEXT PRIMARY KEY,
    po_id           TEXT NOT NULL REFERENCES purchase_orders(id) ON DELETE CASCADE,
    item_id         TEXT NOT NULL REFERENCES items(id),
    description     TEXT,
    uom_id          TEXT REFERENCES uom(id),
    qty_ordered     REAL NOT NULL,
    qty_received    REAL DEFAULT 0,
    unit_price      REAL NOT NULL,
    discount_pct    REAL DEFAULT 0,
    tax_id          TEXT REFERENCES tax_config(id),
    tax_amount      REAL DEFAULT 0,
    line_total      REAL NOT NULL,
    sort_order      INTEGER DEFAULT 0
);

-- GRN Header
CREATE TABLE IF NOT EXISTS grns (
    id              TEXT PRIMARY KEY,
    grn_number      TEXT NOT NULL UNIQUE,
    po_id           TEXT REFERENCES purchase_orders(id),
    vendor_id       TEXT NOT NULL REFERENCES vendors(id),
    location_id     TEXT REFERENCES inventory_locations(id),
    grn_date        TEXT NOT NULL,
    status          TEXT NOT NULL DEFAULT 'draft' CHECK(status IN ('draft','submitted','accepted','partially_accepted','rejected','posted')),
    notes           TEXT,
    approved_by     TEXT,
    approved_at     TEXT,
    posted_by       TEXT,
    posted_at       TEXT,
    created_by      TEXT,
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL
);

-- GRN Lines
CREATE TABLE IF NOT EXISTS grn_lines (
    id              TEXT PRIMARY KEY,
    grn_id          TEXT NOT NULL REFERENCES grns(id) ON DELETE CASCADE,
    po_line_id      TEXT REFERENCES purchase_order_lines(id),
    item_id         TEXT NOT NULL REFERENCES items(id),
    uom_id          TEXT REFERENCES uom(id),
    qty_received    REAL NOT NULL,
    qty_accepted    REAL DEFAULT 0,
    qty_rejected    REAL DEFAULT 0,
    unit_cost       REAL NOT NULL,
    sort_order      INTEGER DEFAULT 0
);

-- Purchase Invoices Header
CREATE TABLE IF NOT EXISTS purchase_invoices (
    id              TEXT PRIMARY KEY,
    pi_number       TEXT NOT NULL UNIQUE,
    vendor_id       TEXT NOT NULL REFERENCES vendors(id),
    grn_id          TEXT REFERENCES grns(id),
    po_id           TEXT REFERENCES purchase_orders(id),
    invoice_date    TEXT NOT NULL,
    due_date        TEXT,
    currency_id     TEXT REFERENCES currencies(id),
    exchange_rate   REAL DEFAULT 1,
    status          TEXT NOT NULL DEFAULT 'draft' CHECK(status IN ('draft','submitted','approved','posted','paid','cancelled')),
    subtotal        REAL DEFAULT 0,
    tax_amount      REAL DEFAULT 0,
    wht_amount      REAL DEFAULT 0,
    discount_amount REAL DEFAULT 0,
    total_amount    REAL DEFAULT 0,
    paid_amount     REAL DEFAULT 0,
    balance_amount  REAL DEFAULT 0,
    jv_id           TEXT,
    notes           TEXT,
    approved_by     TEXT,
    approved_at     TEXT,
    posted_by       TEXT,
    posted_at       TEXT,
    created_by      TEXT,
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL
);

-- Purchase Invoice Lines
CREATE TABLE IF NOT EXISTS purchase_invoice_lines (
    id              TEXT PRIMARY KEY,
    pi_id           TEXT NOT NULL REFERENCES purchase_invoices(id) ON DELETE CASCADE,
    item_id         TEXT NOT NULL REFERENCES items(id),
    grn_line_id     TEXT REFERENCES grn_lines(id),
    description     TEXT,
    uom_id          TEXT REFERENCES uom(id),
    qty             REAL NOT NULL,
    unit_price      REAL NOT NULL,
    discount_pct    REAL DEFAULT 0,
    tax_id          TEXT REFERENCES tax_config(id),
    tax_amount      REAL DEFAULT 0,
    wht_rate        REAL DEFAULT 0,
    wht_amount      REAL DEFAULT 0,
    line_total      REAL NOT NULL,
    sort_order      INTEGER DEFAULT 0
);

-- Sale Orders Header
CREATE TABLE IF NOT EXISTS sale_orders (
    id              TEXT PRIMARY KEY,
    so_number       TEXT NOT NULL UNIQUE,
    customer_id     TEXT NOT NULL REFERENCES customers(id),
    location_id     TEXT REFERENCES inventory_locations(id),
    currency_id     TEXT REFERENCES currencies(id),
    exchange_rate   REAL DEFAULT 1,
    order_date      TEXT NOT NULL,
    delivery_date   TEXT,
    payment_term_id TEXT REFERENCES payment_terms(id),
    status          TEXT NOT NULL DEFAULT 'draft' CHECK(status IN ('draft','submitted','approved','posted','closed','cancelled')),
    subtotal        REAL DEFAULT 0,
    tax_amount      REAL DEFAULT 0,
    discount_amount REAL DEFAULT 0,
    total_amount    REAL DEFAULT 0,
    notes           TEXT,
    approved_by     TEXT,
    approved_at     TEXT,
    posted_by       TEXT,
    posted_at       TEXT,
    created_by      TEXT,
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL
);

-- Sale Order Lines
CREATE TABLE IF NOT EXISTS sale_order_lines (
    id              TEXT PRIMARY KEY,
    so_id           TEXT NOT NULL REFERENCES sale_orders(id) ON DELETE CASCADE,
    item_id         TEXT NOT NULL REFERENCES items(id),
    description     TEXT,
    uom_id          TEXT REFERENCES uom(id),
    qty_ordered     REAL NOT NULL,
    qty_delivered   REAL DEFAULT 0,
    unit_price      REAL NOT NULL,
    discount_pct    REAL DEFAULT 0,
    tax_id          TEXT REFERENCES tax_config(id),
    tax_amount      REAL DEFAULT 0,
    line_total      REAL NOT NULL,
    sort_order      INTEGER DEFAULT 0
);

-- Delivery Challans Header
CREATE TABLE IF NOT EXISTS delivery_challans (
    id              TEXT PRIMARY KEY,
    dc_number       TEXT NOT NULL UNIQUE,
    so_id           TEXT REFERENCES sale_orders(id),
    customer_id     TEXT NOT NULL REFERENCES customers(id),
    location_id     TEXT REFERENCES inventory_locations(id),
    dc_date         TEXT NOT NULL,
    status          TEXT NOT NULL DEFAULT 'draft' CHECK(status IN ('draft','submitted','delivered','returned','posted','cancelled')),
    notes           TEXT,
    posted_by       TEXT,
    posted_at       TEXT,
    created_by      TEXT,
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL
);

-- DC Lines
CREATE TABLE IF NOT EXISTS delivery_challan_lines (
    id              TEXT PRIMARY KEY,
    dc_id           TEXT NOT NULL REFERENCES delivery_challans(id) ON DELETE CASCADE,
    so_line_id      TEXT REFERENCES sale_order_lines(id),
    item_id         TEXT NOT NULL REFERENCES items(id),
    uom_id          TEXT REFERENCES uom(id),
    qty_delivered   REAL NOT NULL,
    unit_price      REAL NOT NULL,
    sort_order      INTEGER DEFAULT 0
);

-- Sale Invoices Header
CREATE TABLE IF NOT EXISTS sale_invoices (
    id              TEXT PRIMARY KEY,
    si_number       TEXT NOT NULL UNIQUE,
    customer_id     TEXT NOT NULL REFERENCES customers(id),
    so_id           TEXT REFERENCES sale_orders(id),
    dc_id           TEXT REFERENCES delivery_challans(id),
    invoice_date    TEXT NOT NULL,
    due_date        TEXT,
    currency_id     TEXT REFERENCES currencies(id),
    exchange_rate   REAL DEFAULT 1,
    status          TEXT NOT NULL DEFAULT 'draft' CHECK(status IN ('draft','submitted','approved','posted','paid','partially_paid','cancelled')),
    subtotal        REAL DEFAULT 0,
    tax_amount      REAL DEFAULT 0,
    discount_amount REAL DEFAULT 0,
    total_amount    REAL DEFAULT 0,
    paid_amount     REAL DEFAULT 0,
    balance_amount  REAL DEFAULT 0,
    jv_id           TEXT,
    notes           TEXT,
    approved_by     TEXT,
    approved_at     TEXT,
    posted_by       TEXT,
    posted_at       TEXT,
    created_by      TEXT,
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL
);

-- Sale Invoice Lines
CREATE TABLE IF NOT EXISTS sale_invoice_lines (
    id              TEXT PRIMARY KEY,
    si_id           TEXT NOT NULL REFERENCES sale_invoices(id) ON DELETE CASCADE,
    item_id         TEXT NOT NULL REFERENCES items(id),
    dc_line_id      TEXT REFERENCES delivery_challan_lines(id),
    description     TEXT,
    uom_id          TEXT REFERENCES uom(id),
    qty             REAL NOT NULL,
    unit_price      REAL NOT NULL,
    discount_pct    REAL DEFAULT 0,
    tax_id          TEXT REFERENCES tax_config(id),
    tax_amount      REAL DEFAULT 0,
    line_total      REAL NOT NULL,
    sort_order      INTEGER DEFAULT 0
);

-- Journal Vouchers Header
CREATE TABLE IF NOT EXISTS journal_vouchers (
    id              TEXT PRIMARY KEY,
    jv_number       TEXT NOT NULL UNIQUE,
    jv_date         TEXT NOT NULL,
    jv_type         TEXT NOT NULL CHECK(jv_type IN ('manual','auto_purchase','auto_sale','auto_payment','auto_payroll')),
    reference_type  TEXT,
    reference_id    TEXT,
    narration       TEXT,
    status          TEXT NOT NULL DEFAULT 'draft' CHECK(status IN ('draft','posted','reversed')),
    fiscal_year_id  TEXT REFERENCES fiscal_years(id),
    currency_id     TEXT REFERENCES currencies(id),
    exchange_rate   REAL DEFAULT 1,
    is_reversed     INTEGER NOT NULL DEFAULT 0,
    reversed_by     TEXT,
    reversed_at     TEXT,
    posted_by       TEXT,
    posted_at       TEXT,
    created_by      TEXT,
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL
);

-- Journal Voucher Lines
CREATE TABLE IF NOT EXISTS journal_voucher_lines (
    id              TEXT PRIMARY KEY,
    jv_id           TEXT NOT NULL REFERENCES journal_vouchers(id) ON DELETE CASCADE,
    account_id      TEXT NOT NULL REFERENCES chart_of_accounts(id),
    debit_amount    REAL DEFAULT 0,
    credit_amount   REAL DEFAULT 0,
    narration       TEXT,
    cost_center     TEXT,
    sort_order      INTEGER DEFAULT 0
);

-- Cash/Bank Receipts Header
CREATE TABLE IF NOT EXISTS receipts (
    id              TEXT PRIMARY KEY,
    receipt_number  TEXT NOT NULL UNIQUE,
    receipt_type    TEXT NOT NULL CHECK(receipt_type IN ('cash','bank')),
    receipt_date    TEXT NOT NULL,
    customer_id     TEXT REFERENCES customers(id),
    account_id      TEXT NOT NULL REFERENCES chart_of_accounts(id),
    amount          REAL NOT NULL,
    currency_id     TEXT REFERENCES currencies(id),
    exchange_rate   REAL DEFAULT 1,
    reference_no    TEXT,
    status          TEXT NOT NULL DEFAULT 'draft' CHECK(status IN ('draft','posted','reversed')),
    jv_id           TEXT,
    notes           TEXT,
    posted_by       TEXT,
    posted_at       TEXT,
    created_by      TEXT,
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL
);

-- Cash/Bank Payments Header
CREATE TABLE IF NOT EXISTS payments (
    id              TEXT PRIMARY KEY,
    payment_number  TEXT NOT NULL UNIQUE,
    payment_type    TEXT NOT NULL CHECK(payment_type IN ('cash','bank')),
    payment_date    TEXT NOT NULL,
    vendor_id       TEXT REFERENCES vendors(id),
    account_id      TEXT NOT NULL REFERENCES chart_of_accounts(id),
    amount          REAL NOT NULL,
    wht_amount      REAL DEFAULT 0,
    currency_id     TEXT REFERENCES currencies(id),
    exchange_rate   REAL DEFAULT 1,
    reference_no    TEXT,
    status          TEXT NOT NULL DEFAULT 'draft' CHECK(status IN ('draft','posted','reversed')),
    jv_id           TEXT,
    notes           TEXT,
    posted_by       TEXT,
    posted_at       TEXT,
    created_by      TEXT,
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL
);

-- Document Number Sequences
CREATE TABLE IF NOT EXISTS doc_sequences (
    id          TEXT PRIMARY KEY,
    doc_type    TEXT NOT NULL UNIQUE,
    prefix      TEXT NOT NULL,
    last_number INTEGER NOT NULL DEFAULT 0,
    year        INTEGER NOT NULL,
    updated_at  TEXT NOT NULL
);

-- HR: Employees
CREATE TABLE IF NOT EXISTS employees (
    id                  TEXT PRIMARY KEY,
    emp_code            TEXT NOT NULL UNIQUE,
    full_name           TEXT NOT NULL,
    father_name         TEXT,
    dob                 TEXT,
    gender              TEXT CHECK(gender IN ('male','female','other')),
    cnic                TEXT,
    passport_no         TEXT,
    nationality         TEXT,
    religion            TEXT,
    marital_status      TEXT,
    phone               TEXT,
    email               TEXT,
    address             TEXT,
    city                TEXT,
    department          TEXT,
    designation         TEXT,
    join_date           TEXT,
    confirm_date        TEXT,
    resign_date         TEXT,
    employment_type     TEXT CHECK(employment_type IN ('permanent','contract','daily_wage','intern')),
    basic_salary        REAL DEFAULT 0,
    bank_name           TEXT,
    bank_account        TEXT,
    bank_iban           TEXT,
    imei_1              TEXT,
    imei_2              TEXT,
    mac_address         TEXT,
    account_id          TEXT REFERENCES chart_of_accounts(id),
    is_active           INTEGER NOT NULL DEFAULT 1,
    is_deleted          INTEGER NOT NULL DEFAULT 0,
    created_by          TEXT,
    created_at          TEXT NOT NULL,
    updated_at          TEXT NOT NULL
);

-- HR: Attendance
CREATE TABLE IF NOT EXISTS attendance (
    id              TEXT PRIMARY KEY,
    employee_id     TEXT NOT NULL REFERENCES employees(id),
    att_date        TEXT NOT NULL,
    check_in        TEXT,
    check_out       TEXT,
    status          TEXT NOT NULL CHECK(status IN ('present','absent','half_day','leave','holiday')),
    overtime_hours  REAL DEFAULT 0,
    notes           TEXT,
    created_by      TEXT,
    created_at      TEXT NOT NULL,
    UNIQUE(employee_id, att_date)
);

-- HR: Leave Types
CREATE TABLE IF NOT EXISTS leave_types (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    days_allowed INTEGER NOT NULL,
    is_paid     INTEGER NOT NULL DEFAULT 1,
    is_active   INTEGER NOT NULL DEFAULT 1,
    created_at  TEXT NOT NULL
);

-- HR: Leave Applications
CREATE TABLE IF NOT EXISTS leave_applications (
    id              TEXT PRIMARY KEY,
    employee_id     TEXT NOT NULL REFERENCES employees(id),
    leave_type_id   TEXT NOT NULL REFERENCES leave_types(id),
    from_date       TEXT NOT NULL,
    to_date         TEXT NOT NULL,
    days            INTEGER NOT NULL,
    reason          TEXT,
    status          TEXT NOT NULL DEFAULT 'pending' CHECK(status IN ('pending','approved','rejected','cancelled')),
    approved_by     TEXT,
    approved_at     TEXT,
    created_by      TEXT,
    created_at      TEXT NOT NULL
);

-- HR: Salary Components
CREATE TABLE IF NOT EXISTS salary_components (
    id              TEXT PRIMARY KEY,
    code            TEXT NOT NULL UNIQUE,
    name            TEXT NOT NULL,
    component_type  TEXT NOT NULL CHECK(component_type IN ('earning','deduction','tax')),
    is_fixed        INTEGER NOT NULL DEFAULT 1,
    formula         TEXT,
    account_id      TEXT REFERENCES chart_of_accounts(id),
    is_active       INTEGER NOT NULL DEFAULT 1,
    created_at      TEXT NOT NULL
);

-- HR: Employee Salary Structure
CREATE TABLE IF NOT EXISTS employee_salary (
    id              TEXT PRIMARY KEY,
    employee_id     TEXT NOT NULL REFERENCES employees(id),
    component_id    TEXT NOT NULL REFERENCES salary_components(id),
    amount          REAL DEFAULT 0,
    percentage      REAL DEFAULT 0,
    effective_from  TEXT NOT NULL,
    effective_to    TEXT,
    created_by      TEXT,
    created_at      TEXT NOT NULL
);

-- HR: Payroll Runs
CREATE TABLE IF NOT EXISTS payroll_runs (
    id               TEXT PRIMARY KEY,
    run_number       TEXT NOT NULL UNIQUE,
    month            INTEGER NOT NULL,
    year             INTEGER NOT NULL,
    run_date         TEXT NOT NULL,
    status           TEXT NOT NULL DEFAULT 'draft' CHECK(status IN ('draft','processed','approved','posted','cancelled')),
    total_gross      REAL DEFAULT 0,
    total_deductions REAL DEFAULT 0,
    total_income_tax REAL DEFAULT 0,
    total_net        REAL DEFAULT 0,
    jv_id            TEXT,
    notes            TEXT,
    approved_by      TEXT,
    approved_at      TEXT,
    posted_by        TEXT,
    posted_at        TEXT,
    created_by       TEXT,
    created_at       TEXT NOT NULL,
    updated_at       TEXT,
    UNIQUE(month, year)
);

-- HR: Payroll Lines
CREATE TABLE IF NOT EXISTS payroll_lines (
    id              TEXT PRIMARY KEY,
    run_id          TEXT NOT NULL REFERENCES payroll_runs(id) ON DELETE CASCADE,
    employee_id     TEXT NOT NULL REFERENCES employees(id),
    working_days    INTEGER DEFAULT 0,
    present_days    INTEGER DEFAULT 0,
    absent_days     INTEGER DEFAULT 0,
    leave_days      INTEGER DEFAULT 0,
    overtime_hours  REAL DEFAULT 0,
    gross_salary    REAL DEFAULT 0,
    total_deductions REAL DEFAULT 0,
    income_tax      REAL DEFAULT 0,
    net_salary      REAL DEFAULT 0,
    created_at      TEXT NOT NULL
);

-- Sync Queue (for mobile offline sync)
CREATE TABLE IF NOT EXISTS sync_queue (
    id              TEXT PRIMARY KEY,
    device_id       TEXT NOT NULL,
    table_name      TEXT NOT NULL,
    record_id       TEXT NOT NULL,
    operation       TEXT NOT NULL CHECK(operation IN ('insert','update','delete')),
    payload         TEXT NOT NULL,
    status          TEXT NOT NULL DEFAULT 'pending' CHECK(status IN ('pending','synced','failed')),
    retry_count     INTEGER DEFAULT 0,
    synced_at       TEXT,
    created_at      TEXT NOT NULL
);
"""


# ──────────────────────────────────────────────────────────────────────────────
# USER DB SCHEMA (limited view — no sensitive admin tables)
# ──────────────────────────────────────────────────────────────────────────────

USER_DDL = """
CREATE TABLE IF NOT EXISTS user_profile (
    id          TEXT PRIMARY KEY,
    username    TEXT NOT NULL,
    full_name   TEXT NOT NULL,
    role        TEXT NOT NULL,
    synced_at   TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS user_permissions_cache (
    module      TEXT NOT NULL,
    action      TEXT NOT NULL,
    is_granted  INTEGER NOT NULL DEFAULT 1,
    PRIMARY KEY(module, action)
);

CREATE TABLE IF NOT EXISTS customers_cache (
    id          TEXT PRIMARY KEY,
    code        TEXT NOT NULL,
    name        TEXT NOT NULL,
    phone       TEXT,
    credit_limit REAL DEFAULT 0,
    synced_at   TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS vendors_cache (
    id          TEXT PRIMARY KEY,
    code        TEXT NOT NULL,
    name        TEXT NOT NULL,
    phone       TEXT,
    synced_at   TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS items_cache (
    id              TEXT PRIMARY KEY,
    code            TEXT NOT NULL,
    name            TEXT NOT NULL,
    standard_price  REAL DEFAULT 0,
    uom_code        TEXT,
    synced_at       TEXT NOT NULL
);
"""


# ──────────────────────────────────────────────────────────────────────────────
# Seed data
# ──────────────────────────────────────────────────────────────────────────────

def _seed_system_config(conn, device_hash: str) -> None:
    rows = [
        ("company_name", "My Company", _now()),
        ("company_ntn", "", _now()),
        ("company_strn", "", _now()),
        ("company_address", "", _now()),
        ("company_phone", "", _now()),
        ("company_email", "", _now()),
        ("company_logo", "", _now()),
        ("base_currency", "PKR", _now()),
        ("device_hash", device_hash, _now()),
        ("app_version", "1.0.0", _now()),
        ("setup_complete", "1", _now()),
    ]
    conn.executemany(
        "INSERT OR IGNORE INTO system_config(key, value, updated_at) VALUES(?,?,?)",
        rows,
    )


def _seed_currencies(conn) -> None:
    rows = [
        (_uuid(), "PKR", "Pakistani Rupee", "₨", 1.0, 1, 1, _now(), _now()),
        (_uuid(), "USD", "US Dollar", "$", 278.0, 0, 1, _now(), _now()),
        (_uuid(), "EUR", "Euro", "€", 295.0, 0, 1, _now(), _now()),
        (_uuid(), "AED", "UAE Dirham", "د.إ", 75.7, 0, 1, _now(), _now()),
        (_uuid(), "SAR", "Saudi Riyal", "﷼", 74.1, 0, 1, _now(), _now()),
        (_uuid(), "GBP", "British Pound", "£", 348.0, 0, 1, _now(), _now()),
    ]
    conn.executemany(
        """INSERT OR IGNORE INTO currencies(id,code,name,symbol,exchange_rate,is_base,is_active,updated_at,created_at)
           VALUES(?,?,?,?,?,?,?,?,?)""",
        rows,
    )


def _seed_uom(conn) -> None:
    rows = [
        (_uuid(), "PCS", "Pieces", 1, _now()),
        (_uuid(), "KG", "Kilogram", 1, _now()),
        (_uuid(), "GM", "Gram", 1, _now()),
        (_uuid(), "LTR", "Litre", 1, _now()),
        (_uuid(), "MTR", "Meter", 1, _now()),
        (_uuid(), "BOX", "Box", 1, _now()),
        (_uuid(), "CTN", "Carton", 1, _now()),
        (_uuid(), "SET", "Set", 1, _now()),
        (_uuid(), "PAIR", "Pair", 1, _now()),
        (_uuid(), "HR", "Hour", 1, _now()),
    ]
    conn.executemany(
        "INSERT OR IGNORE INTO uom(id,code,name,is_active,created_at) VALUES(?,?,?,?,?)",
        rows,
    )


def _seed_payment_terms(conn) -> None:
    rows = [
        (_uuid(), "Immediate", 0, None, None, 1, _now()),
        (_uuid(), "Net 15", 15, None, None, 1, _now()),
        (_uuid(), "Net 30", 30, None, None, 1, _now()),
        (_uuid(), "Net 45", 45, None, None, 1, _now()),
        (_uuid(), "Net 60", 60, None, None, 1, _now()),
        (_uuid(), "Net 90", 90, None, None, 1, _now()),
    ]
    conn.executemany(
        "INSERT OR IGNORE INTO payment_terms(id,name,due_days,discount_days,discount_pct,is_active,created_at) VALUES(?,?,?,?,?,?,?)",
        rows,
    )


def _seed_doc_sequences(conn) -> None:
    year = datetime.now(timezone.utc).year
    docs = ["PO", "GRN", "PI", "PR", "SO", "DC", "SI", "SR", "CR", "BR", "CP", "BP", "JV", "PAY", "EMP"]
    rows = [(_uuid(), doc, doc, 0, year, _now()) for doc in docs]
    conn.executemany(
        "INSERT OR IGNORE INTO doc_sequences(id,doc_type,prefix,last_number,year,updated_at) VALUES(?,?,?,?,?,?)",
        rows,
    )


def _seed_coa_roots(conn) -> None:
    """Seed root Chart of Accounts nodes."""
    now = _now()
    roots = [
        (_uuid(), "1000", "Assets",      "asset",     None, 1, 1, 1, now, now),
        (_uuid(), "2000", "Liabilities", "liability", None, 1, 1, 1, now, now),
        (_uuid(), "3000", "Equity",      "equity",    None, 1, 1, 1, now, now),
        (_uuid(), "4000", "Income",      "income",    None, 1, 1, 1, now, now),
        (_uuid(), "5000", "Expenses",    "expense",   None, 1, 1, 1, now, now),
    ]
    conn.executemany(
        """INSERT OR IGNORE INTO chart_of_accounts
           (id,code,name,account_type,parent_id,level,is_control,is_active,created_at,updated_at)
           VALUES(?,?,?,?,?,?,?,?,?,?)""",
        roots,
    )


def _seed_leave_types(conn) -> None:
    rows = [
        (_uuid(), "Annual Leave", 15, 1, 1, _now()),
        (_uuid(), "Sick Leave", 10, 1, 1, _now()),
        (_uuid(), "Casual Leave", 10, 1, 1, _now()),
        (_uuid(), "Unpaid Leave", 0, 0, 1, _now()),
    ]
    conn.executemany(
        "INSERT OR IGNORE INTO leave_types(id,name,days_allowed,is_paid,is_active,created_at) VALUES(?,?,?,?,?,?)",
        rows,
    )


def _seed_admin_user(conn, username: str, password: str) -> str:
    """Create the default admin user. Returns user_id."""
    user_id = _uuid()
    device_hash = get_device_hash()
    conn.execute(
        """INSERT OR IGNORE INTO users
           (id, username, full_name, password_hash, role, device_hash,
            is_active, is_deleted, failed_attempts, created_at, updated_at)
           VALUES (?,?,?,?,?,?,1,0,0,?,?)""",
        (
            user_id,
            username.lower().strip(),
            "Administrator",
            hash_password(password),
            "admin",
            device_hash,
            _now(),
            _now(),
        ),
    )
    return user_id


# ──────────────────────────────────────────────────────────────────────────────
# Main
# ──────────────────────────────────────────────────────────────────────────────

def main() -> None:
    db_dir = Path(settings.DATA_DIR) / "db"
    db_dir.mkdir(parents=True, exist_ok=True)

    master_path = db_dir / "master.db"
    user_path = db_dir / "user.db"

    device_hash = get_device_hash()

    master_key = derive_master_db_key(device_hash, settings.APP_SECRET, settings.DB_SALT)
    user_key_placeholder = derive_user_db_key(
        device_hash, settings.SECRET_TOKEN, "setup_placeholder", settings.DB_SALT
    )

    print(f"Device hash : {device_hash[:16]}...")
    print(f"DB directory: {db_dir}")

    # ── Master DB ──
    print("\nInitializing master.db...")
    mconn = _open_db(master_path, master_key)
    mconn.executescript(MASTER_DDL)

    _seed_system_config(mconn, device_hash)
    _seed_currencies(mconn)
    _seed_uom(mconn)
    _seed_payment_terms(mconn)
    _seed_doc_sequences(mconn)
    _seed_coa_roots(mconn)
    _seed_leave_types(mconn)

    # Admin user
    print("\nCreate admin account")
    username = input("  Username [admin]: ").strip() or "admin"
    password = getpass.getpass("  Password: ")
    confirm  = getpass.getpass("  Confirm password: ")

    if password != confirm:
        print("ERROR: Passwords do not match. Aborting.")
        mconn.close()
        raise SystemExit(1)

    if len(password) < 8:
        print("ERROR: Password must be at least 8 characters.")
        mconn.close()
        raise SystemExit(1)

    _seed_admin_user(mconn, username, password)
    mconn.commit()
    mconn.close()
    print(f"  master.db initialized at {master_path}")

    # ── User DB ──
    print("\nInitializing user.db...")
    uconn = _open_db(user_path, user_key_placeholder)
    uconn.executescript(USER_DDL)
    uconn.commit()
    uconn.close()
    print(f"  user.db initialized at {user_path}")

    print("\nSetup complete. Start the backend with:")
    print("  uvicorn backend.main:app --host 127.0.0.1 --port 8765 --reload")


if __name__ == "__main__":
    main()
