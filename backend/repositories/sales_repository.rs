//! Persistence for quotations, invoices, credit notes, and customer
//! payments. Posting an invoice or credit note delegates the atomic
//! GL + receivable write to `AccountingRepository`, since that's where
//! `transactions`/`transaction_lines`/`receivables` already live.

use rusqlite::{params, OptionalExtension, Row};

use crate::backend::accounting::accounts_receivable::receivables::Receivable;
use crate::backend::core::errors::{AppError, AppResult};
use crate::backend::core::validation::Validate;
use crate::backend::repositories::accounting_repository::AccountingRepository;
use crate::backend::sales::credit_notes::CreditNote;
use crate::backend::sales::invoices::{Invoice, InvoiceLine, InvoiceStatus};
use crate::backend::sales::payments::CustomerPayment;
use crate::backend::sales::quotations::{Quotation, QuotationLine, QuotationStatus};
use crate::backend::transactions::transaction::Transaction;
use crate::database::database::{Database, Migration};

pub const MIGRATIONS: &[Migration] = &[Migration {
    version: 1000,
    name: "sales_schema",
    sql: "
        CREATE TABLE quotations (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            company_id      INTEGER NOT NULL REFERENCES companies(id),
            customer_id     INTEGER NOT NULL REFERENCES customers(id),
            quotation_date  TEXT NOT NULL,
            valid_until     TEXT NOT NULL,
            status          TEXT NOT NULL
        );
        CREATE TABLE quotation_lines (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            quotation_id    INTEGER NOT NULL REFERENCES quotations(id),
            description     TEXT NOT NULL,
            quantity        TEXT NOT NULL,
            unit_price      TEXT NOT NULL,
            line_number     INTEGER NOT NULL
        );

        CREATE TABLE invoices (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            company_id      INTEGER NOT NULL REFERENCES companies(id),
            customer_id     INTEGER NOT NULL REFERENCES customers(id),
            invoice_date    TEXT NOT NULL,
            due_date        TEXT NOT NULL,
            status          TEXT NOT NULL,
            transaction_id  INTEGER REFERENCES transactions(id)
        );
        CREATE TABLE invoice_lines (
            id                  INTEGER PRIMARY KEY AUTOINCREMENT,
            invoice_id          INTEGER NOT NULL REFERENCES invoices(id),
            revenue_account_id  INTEGER NOT NULL REFERENCES accounts(id),
            description         TEXT NOT NULL,
            quantity            TEXT NOT NULL,
            unit_price          TEXT NOT NULL,
            line_number         INTEGER NOT NULL
        );

        CREATE TABLE credit_notes (
            id                  INTEGER PRIMARY KEY AUTOINCREMENT,
            company_id          INTEGER NOT NULL REFERENCES companies(id),
            customer_id         INTEGER NOT NULL REFERENCES customers(id),
            receivable_id       INTEGER REFERENCES receivables(id),
            credit_note_date    TEXT NOT NULL,
            transaction_id      INTEGER REFERENCES transactions(id)
        );
        CREATE TABLE credit_note_lines (
            id                  INTEGER PRIMARY KEY AUTOINCREMENT,
            credit_note_id      INTEGER NOT NULL REFERENCES credit_notes(id),
            revenue_account_id  INTEGER NOT NULL REFERENCES accounts(id),
            description         TEXT NOT NULL,
            amount              TEXT NOT NULL,
            line_number         INTEGER NOT NULL
        );

        CREATE TABLE customer_payments (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            company_id      INTEGER NOT NULL REFERENCES companies(id),
            customer_id     INTEGER NOT NULL REFERENCES customers(id),
            payment_date    TEXT NOT NULL,
            amount          TEXT NOT NULL,
            transaction_id  INTEGER REFERENCES transactions(id)
        );
    ",
}];

#[derive(Clone)]
pub struct SalesRepository {
    db: Database,
    accounting: AccountingRepository,
}

impl SalesRepository {
    pub fn new(db: Database) -> Self {
        let accounting = AccountingRepository::new(db.clone());
        Self { db, accounting }
    }

    // -- quotations (no GL impact) --------------------------------------

    pub fn create_quotation(&self, quotation: &Quotation) -> AppResult<Quotation> {
        quotation.validate()?;
        self.db.with_transaction(|tx| {
            tx.execute(
                "INSERT INTO quotations (company_id, customer_id, quotation_date, valid_until, status)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![quotation.company_id, quotation.customer_id, quotation.quotation_date, quotation.valid_until, quotation.status.as_str()],
            )?;
            let quotation_id = tx.last_insert_rowid();
            for (line_number, line) in quotation.lines.iter().enumerate() {
                tx.execute(
                    "INSERT INTO quotation_lines (quotation_id, description, quantity, unit_price, line_number)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![quotation_id, line.description, line.quantity.to_string(), line.unit_price.to_string(), line_number as i64],
                )?;
            }
            Ok(quotation_id)
        })
        .map(|id| Quotation { id: Some(id), ..quotation.clone() })
    }

    pub fn get_quotation(&self, id: i64) -> AppResult<Quotation> {
        self.db.with_connection(|conn| {
            let mut header = conn
                .query_row(
                    "SELECT id, company_id, customer_id, quotation_date, valid_until, status FROM quotations WHERE id = ?1",
                    params![id],
                    map_quotation,
                )
                .optional()?
                .ok_or_else(|| AppError::NotFound(format!("quotation {id} not found")))?;

            let mut stmt = conn.prepare(
                "SELECT description, quantity, unit_price FROM quotation_lines WHERE quotation_id = ?1 ORDER BY line_number",
            )?;
            header.lines = stmt
                .query_map(params![id], map_quotation_line)?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(header)
        })
    }

    // -- invoices (the GL-posting document) ------------------------------

    pub fn create_invoice(&self, invoice: &Invoice) -> AppResult<Invoice> {
        invoice.validate()?;
        self.db.with_transaction(|tx| {
            tx.execute(
                "INSERT INTO invoices (company_id, customer_id, invoice_date, due_date, status, transaction_id)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![invoice.company_id, invoice.customer_id, invoice.invoice_date, invoice.due_date, invoice.status.as_str(), invoice.transaction_id],
            )?;
            let invoice_id = tx.last_insert_rowid();
            for (line_number, line) in invoice.lines.iter().enumerate() {
                tx.execute(
                    "INSERT INTO invoice_lines (invoice_id, revenue_account_id, description, quantity, unit_price, line_number)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![invoice_id, line.revenue_account_id, line.description, line.quantity.to_string(), line.unit_price.to_string(), line_number as i64],
                )?;
            }
            Ok(invoice_id)
        })
        .map(|id| Invoice { id: Some(id), ..invoice.clone() })
    }

    pub fn get_invoice(&self, id: i64) -> AppResult<Invoice> {
        self.db.with_connection(|conn| {
            let mut header = conn
                .query_row(
                    "SELECT id, company_id, customer_id, invoice_date, due_date, status, transaction_id FROM invoices WHERE id = ?1",
                    params![id],
                    map_invoice,
                )
                .optional()?
                .ok_or_else(|| AppError::NotFound(format!("invoice {id} not found")))?;

            let mut stmt = conn.prepare(
                "SELECT revenue_account_id, description, quantity, unit_price FROM invoice_lines WHERE invoice_id = ?1 ORDER BY line_number",
            )?;
            header.lines = stmt
                .query_map(params![id], map_invoice_line)?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(header)
        })
    }

    /// Builds the GL transaction and receivable from a draft invoice,
    /// posts both atomically (`AccountingRepository::post_transaction_with_receivable`),
    /// and marks the invoice Posted with the resulting transaction id.
    pub fn post_invoice(&self, invoice_id: i64, ar_account_id: i64) -> AppResult<(Invoice, Transaction, Receivable)> {
        let invoice = self.get_invoice(invoice_id)?;
        if invoice.status != InvoiceStatus::Draft {
            return Err(AppError::Validation("only a draft invoice can be posted".into()));
        }

        let transaction = invoice.build_transaction(ar_account_id);
        let receivable = invoice.build_receivable();
        let (posted_transaction, posted_receivable) = self.accounting.post_transaction_with_receivable(&transaction, &receivable)?;

        self.db.with_connection(|conn| {
            conn.execute(
                "UPDATE invoices SET status = ?1, transaction_id = ?2 WHERE id = ?3",
                params![InvoiceStatus::Posted.as_str(), posted_transaction.id, invoice_id],
            )?;
            Ok(())
        })?;

        Ok((
            Invoice { status: InvoiceStatus::Posted, transaction_id: posted_transaction.id, ..invoice },
            posted_transaction,
            posted_receivable,
        ))
    }

    // -- credit notes -----------------------------------------------------

    pub fn create_credit_note(&self, credit_note: &CreditNote) -> AppResult<CreditNote> {
        credit_note.validate()?;
        self.db.with_transaction(|tx| {
            tx.execute(
                "INSERT INTO credit_notes (company_id, customer_id, receivable_id, credit_note_date, transaction_id)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![credit_note.company_id, credit_note.customer_id, credit_note.receivable_id, credit_note.credit_note_date, credit_note.transaction_id],
            )?;
            let credit_note_id = tx.last_insert_rowid();
            for (line_number, line) in credit_note.lines.iter().enumerate() {
                tx.execute(
                    "INSERT INTO credit_note_lines (credit_note_id, revenue_account_id, description, amount, line_number)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![credit_note_id, line.revenue_account_id, line.description, line.amount.to_string(), line_number as i64],
                )?;
            }
            Ok(credit_note_id)
        })
        .map(|id| CreditNote { id: Some(id), ..credit_note.clone() })
    }

    // -- customer payments --------------------------------------------

    /// Posts the payment's GL transaction, records the payment header,
    /// and applies the amount across the customer's open receivables
    /// (oldest due date first).
    pub fn record_customer_payment(&self, payment: &CustomerPayment, cash_account_id: i64, ar_account_id: i64) -> AppResult<CustomerPayment> {
        payment.validate()?;
        let transaction = payment.build_transaction(cash_account_id, ar_account_id);
        let posted = self.accounting.post_transaction(&transaction)?;

        let payment_id = self.db.with_connection(|conn| {
            conn.execute(
                "INSERT INTO customer_payments (company_id, customer_id, payment_date, amount, transaction_id)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![payment.company_id, payment.customer_id, payment.payment_date, payment.amount.to_string(), posted.id],
            )?;
            Ok(conn.last_insert_rowid())
        })?;

        let open_receivables: Vec<Receivable> = self
            .accounting
            .list_open_receivables_for_company(payment.company_id)?
            .into_iter()
            .filter(|r| r.customer_id == payment.customer_id)
            .collect();
        let allocations = crate::backend::accounting::accounts_receivable::collections::allocate_collection(&open_receivables, payment.amount);
        self.accounting.apply_receivable_payments(&allocations)?;

        Ok(CustomerPayment {
            id: Some(payment_id),
            transaction_id: posted.id,
            ..payment.clone()
        })
    }
}

fn map_quotation(row: &Row) -> rusqlite::Result<Quotation> {
    let status_text: String = row.get(5)?;
    let status = QuotationStatus::parse(&status_text)
        .ok_or_else(|| rusqlite::Error::FromSqlConversionFailure(5, rusqlite::types::Type::Text, "invalid quotation status".into()))?;
    Ok(Quotation {
        id: Some(row.get(0)?),
        company_id: row.get(1)?,
        customer_id: row.get(2)?,
        quotation_date: row.get(3)?,
        valid_until: row.get(4)?,
        status,
        lines: Vec::new(),
    })
}

fn map_quotation_line(row: &Row) -> rusqlite::Result<QuotationLine> {
    let quantity_text: String = row.get(1)?;
    let unit_price_text: String = row.get(2)?;
    Ok(QuotationLine {
        description: row.get(0)?,
        quantity: quantity_text.parse().unwrap_or_default(),
        unit_price: unit_price_text.parse().unwrap_or_default(),
    })
}

fn map_invoice(row: &Row) -> rusqlite::Result<Invoice> {
    let status_text: String = row.get(5)?;
    let status = InvoiceStatus::parse(&status_text)
        .ok_or_else(|| rusqlite::Error::FromSqlConversionFailure(5, rusqlite::types::Type::Text, "invalid invoice status".into()))?;
    Ok(Invoice {
        id: Some(row.get(0)?),
        company_id: row.get(1)?,
        customer_id: row.get(2)?,
        invoice_date: row.get(3)?,
        due_date: row.get(4)?,
        status,
        lines: Vec::new(),
        transaction_id: row.get(6)?,
    })
}

fn map_invoice_line(row: &Row) -> rusqlite::Result<InvoiceLine> {
    let quantity_text: String = row.get(2)?;
    let unit_price_text: String = row.get(3)?;
    Ok(InvoiceLine {
        revenue_account_id: row.get(0)?,
        description: row.get(1)?,
        quantity: quantity_text.parse().unwrap_or_default(),
        unit_price: unit_price_text.parse().unwrap_or_default(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use crate::backend::organization::companies::Company;
    use crate::backend::parties::customers::Customer;
    use crate::backend::repositories::accounting_repository::{self, AccountingRepository as AccRepo};
    use crate::backend::repositories::organization_repository::{self, OrganizationRepository};
    use crate::backend::repositories::party_repository::{self, PartyRepository};
    use crate::backend::accounting::accounting_rules::AccountType;
    use crate::backend::accounting::chart_of_accounts::Account;
    use crate::backend::accounting::fiscal_periods::FiscalPeriod;
    use rust_decimal_macros::dec;

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    struct TestFixture {
        sales: SalesRepository,
        company_id: i64,
        customer_id: i64,
        ar_account_id: i64,
        revenue_account_id: i64,
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
        let customer = party_repo.create_customer(&Customer::new(company_id, "Acme Retail")).unwrap();

        let acc_repo = AccRepo::new(db.clone());
        acc_repo
            .create_fiscal_period(&FiscalPeriod::new(company_id, "January 2026", date(2026, 1, 1), date(2026, 1, 31)))
            .unwrap();
        let ar_account = acc_repo.create_account(&Account::new(company_id, "1100", "Accounts Receivable", AccountType::Asset)).unwrap();
        let revenue_account = acc_repo.create_account(&Account::new(company_id, "4000", "Sales Revenue", AccountType::Revenue)).unwrap();
        let cash_account = acc_repo.create_account(&Account::new(company_id, "1000", "Cash", AccountType::Asset)).unwrap();

        TestFixture {
            sales: SalesRepository::new(db),
            company_id,
            customer_id: customer.id.unwrap(),
            ar_account_id: ar_account.id.unwrap(),
            revenue_account_id: revenue_account.id.unwrap(),
            cash_account_id: cash_account.id.unwrap(),
        }
    }

    #[test]
    fn quotation_round_trips_with_its_lines() {
        let f = fixture();
        let mut quotation = Quotation::new(f.company_id, f.customer_id, date(2026, 1, 1), date(2026, 1, 31));
        quotation.lines.push(QuotationLine::new("Widget", dec!(2), dec!(50)));
        let created = f.sales.create_quotation(&quotation).unwrap();

        let fetched = f.sales.get_quotation(created.id.unwrap()).unwrap();
        assert_eq!(fetched.lines.len(), 1);
        assert_eq!(fetched.total(), dec!(100));
    }

    #[test]
    fn posting_an_invoice_creates_transaction_and_receivable_and_marks_it_posted() {
        let f = fixture();
        let mut invoice = Invoice::new(f.company_id, f.customer_id, date(2026, 1, 10), date(2026, 2, 9));
        invoice.lines.push(InvoiceLine::new(f.revenue_account_id, "Consulting", dec!(1), dec!(2000)));
        let invoice = f.sales.create_invoice(&invoice).unwrap();

        let (posted_invoice, transaction, receivable) = f.sales.post_invoice(invoice.id.unwrap(), f.ar_account_id).unwrap();

        assert_eq!(posted_invoice.status, InvoiceStatus::Posted);
        assert_eq!(posted_invoice.transaction_id, transaction.id);
        assert!(transaction.is_balanced());
        assert_eq!(receivable.amount, dec!(2000));
        assert_eq!(receivable.transaction_id, transaction.id);

        // re-fetching from storage reflects the posted status
        let refetched = f.sales.get_invoice(invoice.id.unwrap()).unwrap();
        assert_eq!(refetched.status, InvoiceStatus::Posted);
    }

    #[test]
    fn posting_an_already_posted_invoice_is_rejected() {
        let f = fixture();
        let mut invoice = Invoice::new(f.company_id, f.customer_id, date(2026, 1, 10), date(2026, 2, 9));
        invoice.lines.push(InvoiceLine::new(f.revenue_account_id, "Consulting", dec!(1), dec!(2000)));
        let invoice = f.sales.create_invoice(&invoice).unwrap();
        f.sales.post_invoice(invoice.id.unwrap(), f.ar_account_id).unwrap();

        let result = f.sales.post_invoice(invoice.id.unwrap(), f.ar_account_id);
        assert!(result.is_err());
    }

    #[test]
    fn recording_a_customer_payment_settles_the_receivable() {
        let f = fixture();
        let mut invoice = Invoice::new(f.company_id, f.customer_id, date(2026, 1, 10), date(2026, 2, 9));
        invoice.lines.push(InvoiceLine::new(f.revenue_account_id, "Consulting", dec!(1), dec!(2000)));
        let invoice = f.sales.create_invoice(&invoice).unwrap();
        f.sales.post_invoice(invoice.id.unwrap(), f.ar_account_id).unwrap();

        let payment = CustomerPayment::new(f.company_id, f.customer_id, date(2026, 1, 20), dec!(2000));
        let recorded = f.sales.record_customer_payment(&payment, f.cash_account_id, f.ar_account_id).unwrap();
        assert!(recorded.transaction_id.is_some());

        let accounting = AccRepo::new(f.sales.db.clone());
        assert!(accounting.list_open_receivables_for_company(f.company_id).unwrap().is_empty());
    }
}
