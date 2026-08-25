//! Persistence for purchase orders, bills, debit notes, and vendor
//! payments. Mirrors `sales_repository.rs`.

use rusqlite::{params, OptionalExtension, Row};
use rust_decimal::Decimal;

use crate::backend::accounting::accounts_payable::payables::Payable;
use crate::backend::core::errors::{AppError, AppResult};
use crate::backend::core::validation::Validate;
use crate::backend::purchases::bills::{Bill, BillLine, BillStatus};
use crate::backend::purchases::debit_notes::{DebitNote, DebitNoteLine};
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

    /// Updates a draft bill's vendor, dates, and lines in place -- only a
    /// Draft can be edited; once posted, corrections go through a debit
    /// note instead. Mirrors `SalesRepository::update_invoice`.
    pub fn update_bill(&self, bill: &Bill) -> AppResult<()> {
        bill.validate()?;
        let bill_id = bill.id.ok_or_else(|| AppError::Validation("cannot update a bill without an id".into()))?;
        let existing = self.get_bill(bill_id)?;
        if existing.status != BillStatus::Draft {
            return Err(AppError::Validation("only a draft bill can be edited".into()));
        }

        self.db.with_transaction(|tx| {
            tx.execute(
                "UPDATE bills SET vendor_id = ?1, bill_date = ?2, due_date = ?3 WHERE id = ?4",
                params![bill.vendor_id, bill.bill_date, bill.due_date, bill_id],
            )?;
            tx.execute("DELETE FROM bill_lines WHERE bill_id = ?1", params![bill_id])?;
            for (line_number, line) in bill.lines.iter().enumerate() {
                tx.execute(
                    "INSERT INTO bill_lines (bill_id, account_id, description, quantity, unit_price, line_number)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![bill_id, line.account_id, line.description, line.quantity.to_string(), line.unit_price.to_string(), line_number as i64],
                )?;
            }
            Ok(())
        })
    }

    /// Deletes a draft bill outright -- only a Draft has no ledger impact
    /// yet, so nothing else references it. Mirrors
    /// `SalesRepository::delete_invoice`.
    pub fn delete_bill(&self, bill_id: i64) -> AppResult<()> {
        let bill = self.get_bill(bill_id)?;
        if bill.status != BillStatus::Draft {
            return Err(AppError::Validation("only a draft bill can be deleted".into()));
        }
        self.db.with_transaction(|tx| {
            tx.execute("DELETE FROM bill_lines WHERE bill_id = ?1", params![bill_id])?;
            tx.execute("DELETE FROM bills WHERE id = ?1", params![bill_id])?;
            Ok(())
        })
    }

    /// Voids a draft bill -- skips the ledger entirely. Mirrors
    /// `SalesRepository::void_invoice`.
    pub fn void_bill(&self, bill_id: i64) -> AppResult<Bill> {
        let bill = self.get_bill(bill_id)?;
        if bill.status != BillStatus::Draft {
            return Err(AppError::Validation("only a draft bill can be voided".into()));
        }
        self.db.with_connection(|conn| {
            conn.execute("UPDATE bills SET status = ?1 WHERE id = ?2", params![BillStatus::Voided.as_str(), bill_id])?;
            Ok(())
        })?;
        Ok(Bill { status: BillStatus::Voided, ..bill })
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

    pub fn get_debit_note(&self, id: i64) -> AppResult<DebitNote> {
        self.db.with_connection(|conn| {
            let mut header = conn
                .query_row(
                    "SELECT id, company_id, vendor_id, payable_id, debit_note_date, transaction_id FROM debit_notes WHERE id = ?1",
                    params![id],
                    map_debit_note,
                )
                .optional()?
                .ok_or_else(|| AppError::NotFound(format!("debit note {id} not found")))?;

            let mut stmt =
                conn.prepare("SELECT account_id, description, amount FROM debit_note_lines WHERE debit_note_id = ?1 ORDER BY line_number")?;
            header.lines = stmt
                .query_map(params![id], map_debit_note_line)?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(header)
        })
    }

    /// Builds the GL transaction from a draft debit note and posts it
    /// (debit AP, credit each line's account). Mirrors
    /// `SalesRepository::post_credit_note`: if the debit note is linked
    /// to a specific payable, rejects it up front if its total exceeds
    /// that payable's remaining balance, for the same reconciliation
    /// reason.
    pub fn post_debit_note(&self, debit_note_id: i64, ap_account_id: i64) -> AppResult<(DebitNote, Transaction)> {
        let debit_note = self.get_debit_note(debit_note_id)?;
        if debit_note.transaction_id.is_some() {
            return Err(AppError::Validation("debit note is already posted".into()));
        }

        if let Some(payable_id) = debit_note.payable_id {
            let payable = self.accounting.get_payable(payable_id)?;
            if debit_note.total() > payable.balance() {
                return Err(AppError::Validation(format!(
                    "debit note amount {} exceeds the linked payable's remaining balance of {}",
                    debit_note.total(),
                    payable.balance()
                )));
            }
        }

        let transaction = debit_note.build_transaction(ap_account_id);
        let posted = self.accounting.post_transaction(&transaction)?;

        self.db.with_connection(|conn| {
            conn.execute("UPDATE debit_notes SET transaction_id = ?1 WHERE id = ?2", params![posted.id, debit_note_id])?;
            Ok(())
        })?;

        if let Some(payable_id) = debit_note.payable_id {
            self.accounting.apply_payable_payments(&[(payable_id, debit_note.total())])?;
        }

        Ok((DebitNote { transaction_id: posted.id, ..debit_note }, posted))
    }

    // -- vendor payments --------------------------------------------------

    /// Rejects a payment greater than the vendor's total open balance
    /// before posting anything -- see the matching note on
    /// `SalesRepository::record_customer_payment` for why: the GL
    /// transaction always moves the payment's full nominal amount, but
    /// subledger allocation caps at what's actually owed.
    pub fn record_vendor_payment(&self, payment: &VendorPayment, ap_account_id: i64, cash_account_id: i64) -> AppResult<VendorPayment> {
        payment.validate()?;

        let open_payables: Vec<Payable> = self
            .accounting
            .list_open_payables_for_company(payment.company_id)?
            .into_iter()
            .filter(|p| p.vendor_id == payment.vendor_id)
            .collect();
        let total_open_balance: Decimal = open_payables.iter().map(Payable::balance).sum();
        if payment.amount > total_open_balance {
            return Err(AppError::Validation(format!(
                "payment amount {} exceeds the vendor's total open balance of {total_open_balance}",
                payment.amount
            )));
        }

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

        let allocations = crate::backend::accounting::accounts_payable::payments::allocate_payment(&open_payables, payment.amount);
        self.accounting.apply_payable_payments(&allocations)?;

        Ok(VendorPayment {
            id: Some(payment_id),
            transaction_id: posted.id,
            ..payment.clone()
        })
    }

    /// Posts a payment against exactly one bill's own payable -- unlike
    /// `record_vendor_payment` (FIFO across the vendor's whole open
    /// balance), this never touches any other payable, even other open
    /// bills for the same vendor.
    ///
    /// The payable is found by the `"BILL-{bill_id}"` reference
    /// `Bill::build_payable` stamps it with at posting time -- the only
    /// link between the two record types today. Rejects if there's no
    /// open payable for this bill (never posted, already fully paid, or
    /// voided) or if `amount` exceeds that payable's own remaining
    /// balance, same conservative reasoning as `record_vendor_payment`'s
    /// whole-balance check, just scoped to one document.
    pub fn record_payment_for_bill(&self, bill_id: i64, payment_date: chrono::NaiveDate, amount: Decimal, ap_account_id: i64, cash_account_id: i64) -> AppResult<VendorPayment> {
        let bill = self.get_bill(bill_id)?;
        let payment = VendorPayment::new(bill.company_id, bill.vendor_id, payment_date, amount);
        payment.validate()?;

        let payable = self.open_payable_for_bill(bill_id)?;

        if amount > payable.balance() {
            return Err(AppError::Validation(format!("payment amount {amount} exceeds bill {bill_id}'s remaining balance of {}", payable.balance())));
        }

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

        self.accounting.apply_payable_payments(&[(payable.id.unwrap_or_default(), amount)])?;

        Ok(VendorPayment {
            id: Some(payment_id),
            transaction_id: posted.id,
            ..payment
        })
    }

    /// The bill's own open payable, found by the `"BILL-{bill_id}"`
    /// reference `Bill::build_payable` stamps it with at posting time --
    /// `pub` since both `bill_remaining_balance` and debit-note posting
    /// need this exact scoped lookup. Mirrors
    /// `SalesRepository::open_receivable_for_invoice`.
    pub fn open_payable_for_bill(&self, bill_id: i64) -> AppResult<Payable> {
        let bill = self.get_bill(bill_id)?;
        let reference = format!("BILL-{bill_id}");
        self.accounting
            .list_open_payables_for_company(bill.company_id)?
            .into_iter()
            .find(|p| p.reference.as_deref() == Some(reference.as_str()))
            .ok_or_else(|| AppError::Validation(format!("bill {bill_id} has no open payable")))
    }

    /// The bill's own remaining balance -- for prefilling a payment or
    /// debit-note form before it's actually submitted.
    pub fn bill_remaining_balance(&self, bill_id: i64) -> AppResult<Decimal> {
        Ok(self.open_payable_for_bill(bill_id)?.balance())
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

fn map_debit_note(row: &Row) -> rusqlite::Result<DebitNote> {
    Ok(DebitNote {
        id: Some(row.get(0)?),
        company_id: row.get(1)?,
        vendor_id: row.get(2)?,
        payable_id: row.get(3)?,
        debit_note_date: row.get(4)?,
        lines: Vec::new(),
        transaction_id: row.get(5)?,
    })
}

fn map_debit_note_line(row: &Row) -> rusqlite::Result<DebitNoteLine> {
    let amount_text: String = row.get(2)?;
    Ok(DebitNoteLine {
        account_id: row.get(0)?,
        description: row.get(1)?,
        amount: amount_text.parse().unwrap_or_default(),
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

    #[test]
    fn posting_a_debit_note_against_a_payable_reduces_its_balance() {
        let f = fixture();
        let mut bill = Bill::new(f.company_id, f.vendor_id, date(2026, 1, 10), date(2026, 2, 9));
        bill.lines.push(BillLine::new(f.expense_account_id, "Office supplies", dec!(1), dec!(300)));
        let bill = f.purchases.create_bill(&bill).unwrap();
        let (_, _, payable) = f.purchases.post_bill(bill.id.unwrap(), f.ap_account_id).unwrap();

        let mut debit_note = DebitNote::new(f.company_id, f.vendor_id, date(2026, 1, 15));
        debit_note.payable_id = Some(payable.id.unwrap());
        debit_note.lines.push(DebitNoteLine::new(f.expense_account_id, "Damaged goods return", dec!(100)));
        let debit_note = f.purchases.create_debit_note(&debit_note).unwrap();

        let (posted, transaction) = f.purchases.post_debit_note(debit_note.id.unwrap(), f.ap_account_id).unwrap();
        assert!(posted.transaction_id.is_some());
        assert!(transaction.is_balanced());

        let accounting = AccRepo::new(f.purchases.db.clone());
        let updated_payable = accounting.get_payable(payable.id.unwrap()).unwrap();
        assert_eq!(updated_payable.balance(), dec!(200));
    }

    #[test]
    fn posting_a_debit_note_exceeding_the_linked_payables_balance_is_rejected() {
        let f = fixture();
        let mut bill = Bill::new(f.company_id, f.vendor_id, date(2026, 1, 10), date(2026, 2, 9));
        bill.lines.push(BillLine::new(f.expense_account_id, "Office supplies", dec!(1), dec!(100)));
        let bill = f.purchases.create_bill(&bill).unwrap();
        let (_, _, payable) = f.purchases.post_bill(bill.id.unwrap(), f.ap_account_id).unwrap();

        let mut debit_note = DebitNote::new(f.company_id, f.vendor_id, date(2026, 1, 15));
        debit_note.payable_id = Some(payable.id.unwrap());
        debit_note.lines.push(DebitNoteLine::new(f.expense_account_id, "Damaged goods return", dec!(250))); // more than the 100 owed
        let debit_note = f.purchases.create_debit_note(&debit_note).unwrap();

        let result = f.purchases.post_debit_note(debit_note.id.unwrap(), f.ap_account_id);
        assert!(result.is_err());

        let accounting = AccRepo::new(f.purchases.db.clone());
        let unchanged_payable = accounting.get_payable(payable.id.unwrap()).unwrap();
        assert_eq!(unchanged_payable.balance(), dec!(100));
    }

    #[test]
    fn posting_an_already_posted_debit_note_is_rejected() {
        let f = fixture();
        let mut debit_note = DebitNote::new(f.company_id, f.vendor_id, date(2026, 1, 15));
        debit_note.lines.push(DebitNoteLine::new(f.expense_account_id, "General credit", dec!(50)));
        let debit_note = f.purchases.create_debit_note(&debit_note).unwrap();
        f.purchases.post_debit_note(debit_note.id.unwrap(), f.ap_account_id).unwrap();

        let result = f.purchases.post_debit_note(debit_note.id.unwrap(), f.ap_account_id);
        assert!(result.is_err());
    }
}
