//! Persistence for purchase orders, bills, debit notes, and vendor
//! payments. Mirrors `sales_repository.rs`.

use rusqlite::{params, OptionalExtension, Row};

use crate::backend::accounting::accounts_payable::payables::Payable;
use crate::backend::core::errors::{AppError, AppResult};
use crate::backend::core::validation::Validate;
use crate::backend::purchases::bills::{Bill, BillLine, BillStatus};
use crate::backend::purchases::debit_notes::DebitNote;
use crate::backend::purchases::payments::VendorPayment;
use crate::backend::purchases::purchase_orders::{PurchaseOrder, PurchaseOrderLine, PurchaseOrderStatus};
use crate::backend::repositories::accounting_repository::AccountingRepository;
use crate::backend::transactions::transaction::Transaction;
use crate::database::database::{Database, Migration};

pub const MIGRATIONS: &[Migration] = &[Migration {
    version: 1001,
    name: "purchases_schema",
    sql: "
        CREATE TABLE purchase_orders (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            company_id      INTEGER NOT NULL REFERENCES companies(id),
            vendor_id       INTEGER NOT NULL REFERENCES vendors(id),
            order_date      TEXT NOT NULL,
            expected_date   TEXT NOT NULL,
            status          TEXT NOT NULL
        );
        CREATE TABLE purchase_order_lines (
            id                  INTEGER PRIMARY KEY AUTOINCREMENT,
            purchase_order_id   INTEGER NOT NULL REFERENCES purchase_orders(id),
            description         TEXT NOT NULL,
            quantity            TEXT NOT NULL,
            unit_price          TEXT NOT NULL,
            line_number         INTEGER NOT NULL
        );

        CREATE TABLE bills (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            company_id      INTEGER NOT NULL REFERENCES companies(id),
            vendor_id       INTEGER NOT NULL REFERENCES vendors(id),
            bill_date       TEXT NOT NULL,
            due_date        TEXT NOT NULL,
            status          TEXT NOT NULL,
            transaction_id  INTEGER REFERENCES transactions(id)
        );
        CREATE TABLE bill_lines (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            bill_id         INTEGER NOT NULL REFERENCES bills(id),
            account_id      INTEGER NOT NULL REFERENCES accounts(id),
            description     TEXT NOT NULL,
            quantity        TEXT NOT NULL,
            unit_price      TEXT NOT NULL,
            line_number     INTEGER NOT NULL
        );

        CREATE TABLE debit_notes (
            id                  INTEGER PRIMARY KEY AUTOINCREMENT,
            company_id          INTEGER NOT NULL REFERENCES companies(id),
            vendor_id           INTEGER NOT NULL REFERENCES vendors(id),
            payable_id          INTEGER REFERENCES payables(id),
            debit_note_date     TEXT NOT NULL,
            transaction_id      INTEGER REFERENCES transactions(id)
        );
        CREATE TABLE debit_note_lines (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            debit_note_id   INTEGER NOT NULL REFERENCES debit_notes(id),
            account_id      INTEGER NOT NULL REFERENCES accounts(id),
            description     TEXT NOT NULL,
            amount          TEXT NOT NULL,
            line_number     INTEGER NOT NULL
        );

        CREATE TABLE vendor_payments (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            company_id      INTEGER NOT NULL REFERENCES companies(id),
            vendor_id       INTEGER NOT NULL REFERENCES vendors(id),
            payment_date    TEXT NOT NULL,
            amount          TEXT NOT NULL,
            transaction_id  INTEGER REFERENCES transactions(id)
        );
    ",
}];

#[derive(Clone)]
pub struct PurchaseRepository {
    db: Database,
    accounting: AccountingRepository,
}

impl PurchaseRepository {
    pub fn new(db: Database) -> Self {
        let accounting = AccountingRepository::new(db.clone());
        Self { db, accounting }
    }

    // -- purchase orders (no GL impact) ----------------------------------

    pub fn create_purchase_order(&self, po: &PurchaseOrder) -> AppResult<PurchaseOrder> {
        po.validate()?;
        self.db.with_transaction(|tx| {
            tx.execute(
                "INSERT INTO purchase_orders (company_id, vendor_id, order_date, expected_date, status)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![po.company_id, po.vendor_id, po.order_date, po.expected_date, po.status.as_str()],
            )?;
            let po_id = tx.last_insert_rowid();
            for (line_number, line) in po.lines.iter().enumerate() {
                tx.execute(
                    "INSERT INTO purchase_order_lines (purchase_order_id, description, quantity, unit_price, line_number)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![po_id, line.description, line.quantity.to_string(), line.unit_price.to_string(), line_number as i64],
                )?;
            }
            Ok(po_id)
        })
        .map(|id| PurchaseOrder { id: Some(id), ..po.clone() })
    }

    pub fn get_purchase_order(&self, id: i64) -> AppResult<PurchaseOrder> {
        self.db.with_connection(|conn| {
            let mut header = conn
                .query_row(
                    "SELECT id, company_id, vendor_id, order_date, expected_date, status FROM purchase_orders WHERE id = ?1",
                    params![id],
                    map_purchase_order,
                )
                .optional()?
                .ok_or_else(|| AppError::NotFound(format!("purchase order {id} not found")))?;

            let mut stmt = conn.prepare(
                "SELECT description, quantity, unit_price FROM purchase_order_lines WHERE purchase_order_id = ?1 ORDER BY line_number",
            )?;
            header.lines = stmt
                .query_map(params![id], map_purchase_order_line)?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(header)
        })
    }

    // -- bills (the GL-posting document) -------------------------------

    pub fn create_bill(&self, bill: &Bill) -> AppResult<Bill> {
        bill.validate()?;
        self.db.with_transaction(|tx| {
            tx.execute(
                "INSERT INTO bills (company_id, vendor_id, bill_date, due_date, status, transaction_id)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![bill.company_id, bill.vendor_id, bill.bill_date, bill.due_date, bill.status.as_str(), bill.transaction_id],
            )?;
            let bill_id = tx.last_insert_rowid();
            for (line_number, line) in bill.lines.iter().enumerate() {
                tx.execute(
                    "INSERT INTO bill_lines (bill_id, account_id, description, quantity, unit_price, line_number)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![bill_id, line.account_id, line.description, line.quantity.to_string(), line.unit_price.to_string(), line_number as i64],
                )?;
            }
            Ok(bill_id)
        })
        .map(|id| Bill { id: Some(id), ..bill.clone() })
    }

    pub fn get_bill(&self, id: i64) -> AppResult<Bill> {
        self.db.with_connection(|conn| {
            let mut header = conn
                .query_row(
                    "SELECT id, company_id, vendor_id, bill_date, due_date, status, transaction_id FROM bills WHERE id = ?1",
                    params![id],
                    map_bill,
                )
                .optional()?
                .ok_or_else(|| AppError::NotFound(format!("bill {id} not found")))?;

            let mut stmt = conn.prepare(
                "SELECT account_id, description, quantity, unit_price FROM bill_lines WHERE bill_id = ?1 ORDER BY line_number",
            )?;
            header.lines = stmt
                .query_map(params![id], map_bill_line)?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(header)
        })
    }

    /// Headers only -- mirrors `SalesRepository::list_invoices_for_company`.
    pub fn list_bills_for_company(&self, company_id: i64) -> AppResult<Vec<Bill>> {
        self.db.with_connection(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, company_id, vendor_id, bill_date, due_date, status, transaction_id
                 FROM bills WHERE company_id = ?1 ORDER BY bill_date DESC, id DESC",
            )?;
            let rows = stmt.query_map(params![company_id], map_bill)?;
            rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
        })
    }

    /// Mirrors `SalesRepository::post_invoice`.
    pub fn post_bill(&self, bill_id: i64, ap_account_id: i64) -> AppResult<(Bill, Transaction, Payable)> {
        let bill = self.get_bill(bill_id)?;
        if bill.status != BillStatus::Draft {
            return Err(AppError::Validation("only a draft bill can be posted".into()));
        }

        let transaction = bill.build_transaction(ap_account_id);
        let payable = bill.build_payable();
        let (posted_transaction, posted_payable) = self.accounting.post_transaction_with_payable(&transaction, &payable)?;

        self.db.with_connection(|conn| {
            conn.execute(
                "UPDATE bills SET status = ?1, transaction_id = ?2 WHERE id = ?3",
                params![BillStatus::Posted.as_str(), posted_transaction.id, bill_id],
            )?;
            Ok(())
        })?;

        Ok((
            Bill { status: BillStatus::Posted, transaction_id: posted_transaction.id, ..bill },
            posted_transaction,
            posted_payable,
        ))
    }

    // -- debit notes ----------------------------------------------------

    pub fn create_debit_note(&self, debit_note: &DebitNote) -> AppResult<DebitNote> {
        debit_note.validate()?;
        self.db.with_transaction(|tx| {
            tx.execute(
                "INSERT INTO debit_notes (company_id, vendor_id, payable_id, debit_note_date, transaction_id)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![debit_note.company_id, debit_note.vendor_id, debit_note.payable_id, debit_note.debit_note_date, debit_note.transaction_id],
            )?;
            let debit_note_id = tx.last_insert_rowid();
            for (line_number, line) in debit_note.lines.iter().enumerate() {
                tx.execute(
                    "INSERT INTO debit_note_lines (debit_note_id, account_id, description, amount, line_number)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![debit_note_id, line.account_id, line.description, line.amount.to_string(), line_number as i64],
                )?;
            }
            Ok(debit_note_id)
        })
        .map(|id| DebitNote { id: Some(id), ..debit_note.clone() })
    }

    // -- vendor payments --------------------------------------------------

    pub fn record_vendor_payment(&self, payment: &VendorPayment, ap_account_id: i64, cash_account_id: i64) -> AppResult<VendorPayment> {
        payment.validate()?;
        let transaction = payment.build_transaction(ap_account_id, cash_account_id);
        let posted = self.accounting.post_transaction(&transaction)?;

        let payment_id = self.db.with_connection(|conn| {
            conn.execute(
                "INSERT INTO vendor_payments (company_id, vendor_id, payment_date, amount, transaction_id)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![payment.company_id, payment.vendor_id, payment.payment_date, payment.amount.to_string(), posted.id],
            )?;
            Ok(conn.last_insert_rowid())
        })?;

        let open_payables: Vec<Payable> = self
            .accounting
            .list_open_payables_for_company(payment.company_id)?
            .into_iter()
            .filter(|p| p.vendor_id == payment.vendor_id)
            .collect();
        let allocations = crate::backend::accounting::accounts_payable::payments::allocate_payment(&open_payables, payment.amount);
        self.accounting.apply_payable_payments(&allocations)?;

        Ok(VendorPayment {
            id: Some(payment_id),
            transaction_id: posted.id,
            ..payment.clone()
        })
    }
}

fn map_purchase_order(row: &Row) -> rusqlite::Result<PurchaseOrder> {
    let status_text: String = row.get(5)?;
    let status = PurchaseOrderStatus::parse(&status_text)
        .ok_or_else(|| rusqlite::Error::FromSqlConversionFailure(5, rusqlite::types::Type::Text, "invalid purchase order status".into()))?;
    Ok(PurchaseOrder {
        id: Some(row.get(0)?),
        company_id: row.get(1)?,
        vendor_id: row.get(2)?,
        order_date: row.get(3)?,
        expected_date: row.get(4)?,
        status,
        lines: Vec::new(),
    })
}

fn map_purchase_order_line(row: &Row) -> rusqlite::Result<PurchaseOrderLine> {
    let quantity_text: String = row.get(1)?;
    let unit_price_text: String = row.get(2)?;
    Ok(PurchaseOrderLine {
        description: row.get(0)?,
        quantity: quantity_text.parse().unwrap_or_default(),
        unit_price: unit_price_text.parse().unwrap_or_default(),
    })
}

fn map_bill(row: &Row) -> rusqlite::Result<Bill> {
    let status_text: String = row.get(5)?;
    let status = BillStatus::parse(&status_text)
        .ok_or_else(|| rusqlite::Error::FromSqlConversionFailure(5, rusqlite::types::Type::Text, "invalid bill status".into()))?;
    Ok(Bill {
        id: Some(row.get(0)?),
        company_id: row.get(1)?,
        vendor_id: row.get(2)?,
        bill_date: row.get(3)?,
        due_date: row.get(4)?,
        status,
        lines: Vec::new(),
        transaction_id: row.get(6)?,
    })
}

fn map_bill_line(row: &Row) -> rusqlite::Result<BillLine> {
    let quantity_text: String = row.get(2)?;
    let unit_price_text: String = row.get(3)?;
    Ok(BillLine {
        account_id: row.get(0)?,
        description: row.get(1)?,
        quantity: quantity_text.parse().unwrap_or_default(),
        unit_price: unit_price_text.parse().unwrap_or_default(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use crate::backend::accounting::accounting_rules::AccountType;
    use crate::backend::accounting::chart_of_accounts::Account;
    use crate::backend::accounting::fiscal_periods::FiscalPeriod;
    use crate::backend::organization::companies::Company;
    use crate::backend::parties::vendors::Vendor;
    use crate::backend::repositories::accounting_repository::{self, AccountingRepository as AccRepo};
    use crate::backend::repositories::organization_repository::{self, OrganizationRepository};
    use crate::backend::repositories::party_repository::{self, PartyRepository};
    use rust_decimal_macros::dec;

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    struct TestFixture {
        purchases: PurchaseRepository,
        company_id: i64,
        vendor_id: i64,
        ap_account_id: i64,
        expense_account_id: i64,
        cash_account_id: i64,
    }

    fn fixture() -> TestFixture {
        let db = Database::open_in_memory().unwrap();
        db.run_migrations(organization_repository::MIGRATIONS).unwrap();
        db.run_migrations(party_repository::MIGRATIONS).unwrap();
        db.run_migrations(accounting_repository::MIGRATIONS).unwrap();
        db.run_migrations(crate::backend::audit::audit_logs::MIGRATIONS).unwrap();
        db.run_migrations(MIGRATIONS).unwrap();

        let org_repo = OrganizationRepository::new(db.clone());
        let company = org_repo.create_company(&Company::new("Acme Trading Co", "USD")).unwrap();
        let company_id = company.id.unwrap();

        let party_repo = PartyRepository::new(db.clone());
        let vendor = party_repo.create_vendor(&Vendor::new(company_id, "Acme Supplies Ltd")).unwrap();

        let acc_repo = AccRepo::new(db.clone());
        acc_repo
            .create_fiscal_period(&FiscalPeriod::new(company_id, "January 2026", date(2026, 1, 1), date(2026, 1, 31)))
            .unwrap();
        let ap_account = acc_repo.create_account(&Account::new(company_id, "2000", "Accounts Payable", AccountType::Liability)).unwrap();
        let expense_account = acc_repo.create_account(&Account::new(company_id, "5000", "Office Expense", AccountType::Expense)).unwrap();
        let cash_account = acc_repo.create_account(&Account::new(company_id, "1000", "Cash", AccountType::Asset)).unwrap();

        TestFixture {
            purchases: PurchaseRepository::new(db),
            company_id,
            vendor_id: vendor.id.unwrap(),
            ap_account_id: ap_account.id.unwrap(),
            expense_account_id: expense_account.id.unwrap(),
            cash_account_id: cash_account.id.unwrap(),
        }
    }

    #[test]
    fn posting_a_bill_creates_transaction_and_payable_and_marks_it_posted() {
        let f = fixture();
        let mut bill = Bill::new(f.company_id, f.vendor_id, date(2026, 1, 10), date(2026, 2, 9));
        bill.lines.push(BillLine::new(f.expense_account_id, "Office supplies", dec!(1), dec!(300)));
        let bill = f.purchases.create_bill(&bill).unwrap();

        let (posted_bill, transaction, payable) = f.purchases.post_bill(bill.id.unwrap(), f.ap_account_id).unwrap();

        assert_eq!(posted_bill.status, BillStatus::Posted);
        assert!(transaction.is_balanced());
        assert_eq!(payable.amount, dec!(300));

        let refetched = f.purchases.get_bill(bill.id.unwrap()).unwrap();
        assert_eq!(refetched.status, BillStatus::Posted);
    }

    #[test]
    fn recording_a_vendor_payment_settles_the_payable() {
        let f = fixture();
        let mut bill = Bill::new(f.company_id, f.vendor_id, date(2026, 1, 10), date(2026, 2, 9));
        bill.lines.push(BillLine::new(f.expense_account_id, "Office supplies", dec!(1), dec!(300)));
        let bill = f.purchases.create_bill(&bill).unwrap();
        f.purchases.post_bill(bill.id.unwrap(), f.ap_account_id).unwrap();

        let payment = VendorPayment::new(f.company_id, f.vendor_id, date(2026, 1, 20), dec!(300));
        let recorded = f.purchases.record_vendor_payment(&payment, f.ap_account_id, f.cash_account_id).unwrap();
        assert!(recorded.transaction_id.is_some());

        let accounting = AccRepo::new(f.purchases.db.clone());
        assert!(accounting.list_open_payables_for_company(f.company_id).unwrap().is_empty());
    }
}
