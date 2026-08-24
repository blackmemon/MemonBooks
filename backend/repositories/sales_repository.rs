//! Persistence for quotations, invoices, credit notes, and customer
//! payments. Posting an invoice or credit note delegates the atomic
//! GL + receivable write to `AccountingRepository`, since that's where
//! `transactions`/`transaction_lines`/`receivables` already live.
//!
//! Also POS (Development Order step 16): terminals, sessions, cash
//! drawer events, and POS sales. A POS sale is still fundamentally a
//! sale (build a transaction, post it through the same engine), so it
//! lives here rather than inventing a dedicated repository the frozen
//! structure doesn't have a slot for.

use chrono::{DateTime, Utc};
use rusqlite::{params, OptionalExtension, Row};
use rust_decimal::Decimal;

use crate::backend::accounting::accounts_receivable::receivables::Receivable;
use crate::backend::core::errors::{AppError, AppResult};
use crate::backend::core::validation::Validate;
use crate::backend::pos::cash_drawer::{CashDrawerEvent, CashDrawerEventType};
use crate::backend::pos::payments::PaymentMethod;
use crate::backend::pos::sales::PosSale;
use crate::backend::pos::sessions::{PosSession, PosSessionStatus};
use crate::backend::pos::terminals::Terminal;
use crate::backend::repositories::accounting_repository::AccountingRepository;
use crate::backend::sales::credit_notes::{CreditNote, CreditNoteLine};
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
}, Migration {
    version: 1601,
    name: "pos_schema",
    sql: "
        CREATE TABLE pos_terminals (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            company_id      INTEGER NOT NULL REFERENCES companies(id),
            branch_id       INTEGER REFERENCES branches(id),
            warehouse_id    INTEGER NOT NULL REFERENCES warehouses(id),
            name            TEXT NOT NULL,
            is_active       INTEGER NOT NULL DEFAULT 1
        );

        CREATE TABLE pos_sessions (
            id                  INTEGER PRIMARY KEY AUTOINCREMENT,
            terminal_id         INTEGER NOT NULL REFERENCES pos_terminals(id),
            cashier_user_id     INTEGER NOT NULL REFERENCES users(id),
            opened_at           TEXT NOT NULL,
            closed_at           TEXT,
            opening_cash        TEXT NOT NULL,
            closing_cash        TEXT,
            status              TEXT NOT NULL
        );

        CREATE TABLE cash_drawer_events (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            session_id      INTEGER NOT NULL REFERENCES pos_sessions(id),
            event_type      TEXT NOT NULL,
            amount          TEXT NOT NULL,
            reason          TEXT,
            occurred_at     TEXT NOT NULL
        );

        CREATE TABLE pos_sales (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            company_id      INTEGER NOT NULL REFERENCES companies(id),
            terminal_id     INTEGER NOT NULL REFERENCES pos_terminals(id),
            session_id      INTEGER NOT NULL REFERENCES pos_sessions(id),
            sale_date       TEXT NOT NULL,
            transaction_id  INTEGER REFERENCES transactions(id)
        );

        CREATE TABLE pos_sale_lines (
            id                  INTEGER PRIMARY KEY AUTOINCREMENT,
            pos_sale_id         INTEGER NOT NULL REFERENCES pos_sales(id),
            product_id          INTEGER NOT NULL REFERENCES products(id),
            revenue_account_id  INTEGER NOT NULL REFERENCES accounts(id),
            description         TEXT NOT NULL,
            quantity            TEXT NOT NULL,
            unit_price          TEXT NOT NULL,
            line_number         INTEGER NOT NULL
        );

        CREATE TABLE pos_sale_payments (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            pos_sale_id     INTEGER NOT NULL REFERENCES pos_sales(id),
            method          TEXT NOT NULL,
            amount          TEXT NOT NULL
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

    /// Headers only (no line items) -- enough for a list view; call
    /// `get_invoice` for the full detail of one invoice.
    pub fn list_invoices_for_company(&self, company_id: i64) -> AppResult<Vec<Invoice>> {
        self.db.with_connection(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, company_id, customer_id, invoice_date, due_date, status, transaction_id
                 FROM invoices WHERE company_id = ?1 ORDER BY invoice_date DESC, id DESC",
            )?;
            let rows = stmt.query_map(params![company_id], map_invoice)?;
            rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
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

    pub fn get_credit_note(&self, id: i64) -> AppResult<CreditNote> {
        self.db.with_connection(|conn| {
            let mut header = conn
                .query_row(
                    "SELECT id, company_id, customer_id, receivable_id, credit_note_date, transaction_id FROM credit_notes WHERE id = ?1",
                    params![id],
                    map_credit_note,
                )
                .optional()?
                .ok_or_else(|| AppError::NotFound(format!("credit note {id} not found")))?;

            let mut stmt = conn.prepare(
                "SELECT revenue_account_id, description, amount FROM credit_note_lines WHERE credit_note_id = ?1 ORDER BY line_number",
            )?;
            header.lines = stmt
                .query_map(params![id], map_credit_note_line)?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(header)
        })
    }

    /// Builds the GL transaction from a draft credit note and posts it
    /// (credit AR, debit each line's revenue account). If the credit
    /// note is linked to a specific receivable, rejects it up front if
    /// its total exceeds that receivable's remaining balance -- same
    /// reasoning as `record_customer_payment`'s cap: crediting more than
    /// what's actually still owed would leave the AR control account
    /// and the receivables subledger out of agreement.
    pub fn post_credit_note(&self, credit_note_id: i64, ar_account_id: i64) -> AppResult<(CreditNote, Transaction)> {
        let credit_note = self.get_credit_note(credit_note_id)?;
        if credit_note.transaction_id.is_some() {
            return Err(AppError::Validation("credit note is already posted".into()));
        }

        if let Some(receivable_id) = credit_note.receivable_id {
            let receivable = self.accounting.get_receivable(receivable_id)?;
            if credit_note.total() > receivable.balance() {
                return Err(AppError::Validation(format!(
                    "credit note amount {} exceeds the linked receivable's remaining balance of {}",
                    credit_note.total(),
                    receivable.balance()
                )));
            }
        }

        let transaction = credit_note.build_transaction(ar_account_id);
        let posted = self.accounting.post_transaction(&transaction)?;

        self.db.with_connection(|conn| {
            conn.execute("UPDATE credit_notes SET transaction_id = ?1 WHERE id = ?2", params![posted.id, credit_note_id])?;
            Ok(())
        })?;

        if let Some(receivable_id) = credit_note.receivable_id {
            self.accounting.apply_receivable_payments(&[(receivable_id, credit_note.total())])?;
        }

        Ok((CreditNote { transaction_id: posted.id, ..credit_note }, posted))
    }

    // -- customer payments --------------------------------------------

    /// Posts the payment's GL transaction, records the payment header,
    /// and applies the amount across the customer's open receivables
    /// (oldest due date first).
    ///
    /// Rejects a payment greater than the customer's total open
    /// balance before posting anything -- the GL transaction always
    /// moves the payment's full nominal amount, but allocation to the
    /// subledger caps at what's actually owed, so an unchecked
    /// overpayment would leave the AR control account and the
    /// receivables subledger permanently out of agreement with no
    /// record of where the excess went.
    pub fn record_customer_payment(&self, payment: &CustomerPayment, cash_account_id: i64, ar_account_id: i64) -> AppResult<CustomerPayment> {
        payment.validate()?;

        let open_receivables: Vec<Receivable> = self
            .accounting
            .list_open_receivables_for_company(payment.company_id)?
            .into_iter()
            .filter(|r| r.customer_id == payment.customer_id)
            .collect();
        let total_open_balance: Decimal = open_receivables.iter().map(Receivable::balance).sum();
        if payment.amount > total_open_balance {
            return Err(AppError::Validation(format!(
                "payment amount {} exceeds the customer's total open balance of {total_open_balance}",
                payment.amount
            )));
        }

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

        let allocations = crate::backend::accounting::accounts_receivable::collections::allocate_collection(&open_receivables, payment.amount);
        self.accounting.apply_receivable_payments(&allocations)?;

        Ok(CustomerPayment {
            id: Some(payment_id),
            transaction_id: posted.id,
            ..payment.clone()
        })
    }

    /// Posts a payment against exactly one invoice's own receivable --
    /// unlike `record_customer_payment` (FIFO across the customer's
    /// whole open balance), this never touches any other receivable,
    /// even other open invoices for the same customer.
    ///
    /// The receivable is found by the `"INV-{invoice_id}"` reference
    /// `Invoice::build_receivable` stamps it with at posting time -- the
    /// only link between the two record types today. Rejects if there's
    /// no open receivable for this invoice (never posted, already fully
    /// paid, or voided) or if `amount` exceeds that receivable's own
    /// remaining balance, same conservative reasoning as
    /// `record_customer_payment`'s whole-balance check, just scoped to
    /// one document.
    pub fn record_payment_for_invoice(&self, invoice_id: i64, payment_date: chrono::NaiveDate, amount: Decimal, cash_account_id: i64, ar_account_id: i64) -> AppResult<CustomerPayment> {
        let invoice = self.get_invoice(invoice_id)?;
        let payment = CustomerPayment::new(invoice.company_id, invoice.customer_id, payment_date, amount);
        payment.validate()?;

        let receivable = self.find_open_receivable_for_invoice(invoice_id, invoice.company_id)?;

        if amount > receivable.balance() {
            return Err(AppError::Validation(format!(
                "payment amount {amount} exceeds invoice {invoice_id}'s remaining balance of {}",
                receivable.balance()
            )));
        }

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

        self.accounting.apply_receivable_payments(&[(receivable.id.unwrap_or_default(), amount)])?;

        Ok(CustomerPayment {
            id: Some(payment_id),
            transaction_id: posted.id,
            ..payment
        })
    }

    fn find_open_receivable_for_invoice(&self, invoice_id: i64, company_id: i64) -> AppResult<Receivable> {
        let reference = format!("INV-{invoice_id}");
        self.accounting
            .list_open_receivables_for_company(company_id)?
            .into_iter()
            .find(|r| r.reference.as_deref() == Some(reference.as_str()))
            .ok_or_else(|| AppError::Validation(format!("invoice {invoice_id} has no open receivable to pay")))
    }

    /// The invoice's own remaining balance -- for prefilling a payment
    /// form before `record_payment_for_invoice` is actually called.
    pub fn invoice_remaining_balance(&self, invoice_id: i64) -> AppResult<Decimal> {
        let invoice = self.get_invoice(invoice_id)?;
        Ok(self.find_open_receivable_for_invoice(invoice_id, invoice.company_id)?.balance())
    }

    // -- POS: terminals -------------------------------------------------

    pub fn create_terminal(&self, terminal: &Terminal) -> AppResult<Terminal> {
        terminal.validate()?;
        self.db.with_connection(|conn| {
            conn.execute(
                "INSERT INTO pos_terminals (company_id, branch_id, warehouse_id, name, is_active) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![terminal.company_id, terminal.branch_id, terminal.warehouse_id, terminal.name, terminal.is_active],
            )?;
            Ok(Terminal { id: Some(conn.last_insert_rowid()), ..terminal.clone() })
        })
    }

    pub fn list_terminals_for_company(&self, company_id: i64) -> AppResult<Vec<Terminal>> {
        self.db.with_connection(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, company_id, branch_id, warehouse_id, name, is_active FROM pos_terminals WHERE company_id = ?1 ORDER BY name",
            )?;
            let rows = stmt.query_map(params![company_id], map_terminal)?;
            rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
        })
    }

    // -- POS: sessions and cash drawer ---------------------------------

    pub fn open_session(&self, session: &PosSession) -> AppResult<PosSession> {
        session.validate()?;
        self.db.with_connection(|conn| {
            conn.execute(
                "INSERT INTO pos_sessions (terminal_id, cashier_user_id, opened_at, closed_at, opening_cash, closing_cash, status)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    session.terminal_id,
                    session.cashier_user_id,
                    session.opened_at,
                    session.closed_at,
                    session.opening_cash.to_string(),
                    session.closing_cash.map(|c| c.to_string()),
                    session.status.as_str(),
                ],
            )?;
            Ok(PosSession { id: Some(conn.last_insert_rowid()), ..session.clone() })
        })
    }

    pub fn get_session(&self, id: i64) -> AppResult<PosSession> {
        self.db.with_connection(|conn| {
            conn.query_row(
                "SELECT id, terminal_id, cashier_user_id, opened_at, closed_at, opening_cash, closing_cash, status FROM pos_sessions WHERE id = ?1",
                params![id],
                map_session,
            )
            .optional()?
            .ok_or_else(|| AppError::NotFound(format!("POS session {id} not found")))
        })
    }

    pub fn record_cash_drawer_event(&self, event: &CashDrawerEvent) -> AppResult<CashDrawerEvent> {
        event.validate()?;
        self.db.with_connection(|conn| {
            conn.execute(
                "INSERT INTO cash_drawer_events (session_id, event_type, amount, reason, occurred_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![event.session_id, event.event_type.as_str(), event.amount.to_string(), event.reason, event.occurred_at],
            )?;
            Ok(CashDrawerEvent { id: Some(conn.last_insert_rowid()), ..event.clone() })
        })
    }

    pub fn list_cash_drawer_events(&self, session_id: i64) -> AppResult<Vec<CashDrawerEvent>> {
        self.db.with_connection(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, session_id, event_type, amount, reason, occurred_at FROM cash_drawer_events WHERE session_id = ?1 ORDER BY occurred_at",
            )?;
            let rows = stmt.query_map(params![session_id], map_cash_drawer_event)?;
            rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
        })
    }

    /// Closes the session with the counted amount. Does not itself judge
    /// over/short -- pair with `cash_drawer::expected_closing_cash` using
    /// `list_cash_drawer_events` first, if that comparison is needed.
    pub fn close_session(&self, session_id: i64, closing_cash: rust_decimal::Decimal, closed_at: DateTime<Utc>) -> AppResult<PosSession> {
        let session = self.get_session(session_id)?;
        if session.status != PosSessionStatus::Open {
            return Err(AppError::Validation("only an open POS session can be closed".into()));
        }
        self.db.with_connection(|conn| {
            conn.execute(
                "UPDATE pos_sessions SET status = ?1, closing_cash = ?2, closed_at = ?3 WHERE id = ?4",
                params![PosSessionStatus::Closed.as_str(), closing_cash.to_string(), closed_at, session_id],
            )?;
            Ok(())
        })?;
        Ok(PosSession {
            status: PosSessionStatus::Closed,
            closing_cash: Some(closing_cash),
            closed_at: Some(closed_at),
            ..session
        })
    }

    // -- POS: sales -----------------------------------------------------

    /// Validates and posts the sale's GL transaction, then stores the
    /// sale (header, lines, payments) linked to it -- atomically, same
    /// pattern as `post_invoice`.
    pub fn post_pos_sale(&self, sale: &PosSale, payment_accounts: &[(PaymentMethod, i64)]) -> AppResult<(PosSale, Transaction)> {
        sale.validate()?;
        let transaction = sale.build_transaction(payment_accounts)?;
        let posted = self.accounting.post_transaction(&transaction)?;

        let sale_id = self.db.with_transaction(|tx| {
            tx.execute(
                "INSERT INTO pos_sales (company_id, terminal_id, session_id, sale_date, transaction_id) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![sale.company_id, sale.terminal_id, sale.session_id, sale.sale_date, posted.id],
            )?;
            let sale_id = tx.last_insert_rowid();

            for (line_number, line) in sale.lines.iter().enumerate() {
                tx.execute(
                    "INSERT INTO pos_sale_lines (pos_sale_id, product_id, revenue_account_id, description, quantity, unit_price, line_number)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![sale_id, line.product_id, line.revenue_account_id, line.description, line.quantity.to_string(), line.unit_price.to_string(), line_number as i64],
                )?;
            }
            for payment in &sale.payments {
                tx.execute(
                    "INSERT INTO pos_sale_payments (pos_sale_id, method, amount) VALUES (?1, ?2, ?3)",
                    params![sale_id, payment.method.as_str(), payment.amount.to_string()],
                )?;
            }

            Ok(sale_id)
        })?;

        Ok((PosSale { id: Some(sale_id), transaction_id: posted.id, ..sale.clone() }, posted))
    }
}

fn map_terminal(row: &Row) -> rusqlite::Result<Terminal> {
    Ok(Terminal {
        id: Some(row.get(0)?),
        company_id: row.get(1)?,
        branch_id: row.get(2)?,
        warehouse_id: row.get(3)?,
        name: row.get(4)?,
        is_active: row.get(5)?,
    })
}

fn map_session(row: &Row) -> rusqlite::Result<PosSession> {
    let opening_text: String = row.get(5)?;
    let closing_text: Option<String> = row.get(6)?;
    let status_text: String = row.get(7)?;
    let status = PosSessionStatus::parse(&status_text)
        .ok_or_else(|| rusqlite::Error::FromSqlConversionFailure(7, rusqlite::types::Type::Text, "invalid POS session status".into()))?;
    Ok(PosSession {
        id: Some(row.get(0)?),
        terminal_id: row.get(1)?,
        cashier_user_id: row.get(2)?,
        opened_at: row.get(3)?,
        closed_at: row.get(4)?,
        opening_cash: opening_text.parse().unwrap_or(rust_decimal::Decimal::ZERO),
        closing_cash: closing_text.and_then(|c| c.parse().ok()),
        status,
    })
}

fn map_cash_drawer_event(row: &Row) -> rusqlite::Result<CashDrawerEvent> {
    let amount_text: String = row.get(3)?;
    let event_type_text: String = row.get(2)?;
    let event_type = CashDrawerEventType::parse(&event_type_text)
        .ok_or_else(|| rusqlite::Error::FromSqlConversionFailure(2, rusqlite::types::Type::Text, "invalid cash drawer event type".into()))?;
    Ok(CashDrawerEvent {
        id: Some(row.get(0)?),
        session_id: row.get(1)?,
        event_type,
        amount: amount_text.parse().unwrap_or(rust_decimal::Decimal::ZERO),
        reason: row.get(4)?,
        occurred_at: row.get(5)?,
    })
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

fn map_credit_note(row: &Row) -> rusqlite::Result<CreditNote> {
    Ok(CreditNote {
        id: Some(row.get(0)?),
        company_id: row.get(1)?,
        customer_id: row.get(2)?,
        receivable_id: row.get(3)?,
        credit_note_date: row.get(4)?,
        lines: Vec::new(),
        transaction_id: row.get(5)?,
    })
}

fn map_credit_note_line(row: &Row) -> rusqlite::Result<CreditNoteLine> {
    let amount_text: String = row.get(2)?;
    Ok(CreditNoteLine {
        revenue_account_id: row.get(0)?,
        description: row.get(1)?,
        amount: amount_text.parse().unwrap_or_default(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use crate::backend::organization::companies::Company;
    use crate::backend::pos::payments::PosPayment;
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
        warehouse_id: i64,
        product_id: i64,
        cashier_user_id: i64,
    }

    fn fixture() -> TestFixture {
        let db = Database::open_in_memory().unwrap();
        db.run_migrations(organization_repository::MIGRATIONS).unwrap();
        db.run_migrations(party_repository::MIGRATIONS).unwrap();
        db.run_migrations(accounting_repository::MIGRATIONS).unwrap();
        db.run_migrations(crate::backend::repositories::user_repository::MIGRATIONS).unwrap();
        db.run_migrations(crate::backend::repositories::inventory_repository::MIGRATIONS).unwrap();
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

        let user_repo = crate::backend::repositories::user_repository::UserRepository::new(db.clone());
        let role = user_repo.create_role(&crate::backend::core::roles::Role::new("Cashier")).unwrap();
        let cashier = user_repo
            .create_user(&crate::backend::core::users::User::new("cashier", "Cashier One", "hash", role.id.unwrap()))
            .unwrap();

        let inventory_repo = crate::backend::repositories::inventory_repository::InventoryRepository::new(db.clone());
        let warehouse = inventory_repo
            .create_warehouse(&crate::backend::inventory::warehouses::Warehouse::new(company_id, "Main Warehouse", "MAIN"))
            .unwrap();
        let product = inventory_repo.create_product(&crate::backend::inventory::products::Product::new(company_id, "WIDGET-1", "Widget")).unwrap();

        TestFixture {
            sales: SalesRepository::new(db),
            company_id,
            customer_id: customer.id.unwrap(),
            ar_account_id: ar_account.id.unwrap(),
            revenue_account_id: revenue_account.id.unwrap(),
            cash_account_id: cash_account.id.unwrap(),
            warehouse_id: warehouse.id.unwrap(),
            product_id: product.id.unwrap(),
            cashier_user_id: cashier.id.unwrap(),
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

    #[test]
    fn posting_a_credit_note_against_a_receivable_reduces_its_balance() {
        let f = fixture();
        let mut invoice = Invoice::new(f.company_id, f.customer_id, date(2026, 1, 10), date(2026, 2, 9));
        invoice.lines.push(InvoiceLine::new(f.revenue_account_id, "Consulting", dec!(1), dec!(2000)));
        let invoice = f.sales.create_invoice(&invoice).unwrap();
        let (_, _, receivable) = f.sales.post_invoice(invoice.id.unwrap(), f.ar_account_id).unwrap();

        let mut credit_note = CreditNote::new(f.company_id, f.customer_id, date(2026, 1, 15));
        credit_note.receivable_id = Some(receivable.id.unwrap());
        credit_note.lines.push(CreditNoteLine::new(f.revenue_account_id, "Return", dec!(500)));
        let credit_note = f.sales.create_credit_note(&credit_note).unwrap();

        let (posted, transaction) = f.sales.post_credit_note(credit_note.id.unwrap(), f.ar_account_id).unwrap();
        assert!(posted.transaction_id.is_some());
        assert!(transaction.is_balanced());

        let accounting = AccRepo::new(f.sales.db.clone());
        let updated_receivable = accounting.get_receivable(receivable.id.unwrap()).unwrap();
        assert_eq!(updated_receivable.balance(), dec!(1500));
    }

    #[test]
    fn posting_a_credit_note_exceeding_the_linked_receivables_balance_is_rejected() {
        let f = fixture();
        let mut invoice = Invoice::new(f.company_id, f.customer_id, date(2026, 1, 10), date(2026, 2, 9));
        invoice.lines.push(InvoiceLine::new(f.revenue_account_id, "Consulting", dec!(1), dec!(500)));
        let invoice = f.sales.create_invoice(&invoice).unwrap();
        let (_, _, receivable) = f.sales.post_invoice(invoice.id.unwrap(), f.ar_account_id).unwrap();

        let mut credit_note = CreditNote::new(f.company_id, f.customer_id, date(2026, 1, 15));
        credit_note.receivable_id = Some(receivable.id.unwrap());
        credit_note.lines.push(CreditNoteLine::new(f.revenue_account_id, "Return", dec!(1000))); // more than the 500 owed
        let credit_note = f.sales.create_credit_note(&credit_note).unwrap();

        let result = f.sales.post_credit_note(credit_note.id.unwrap(), f.ar_account_id);
        assert!(result.is_err());

        let accounting = AccRepo::new(f.sales.db.clone());
        let unchanged_receivable = accounting.get_receivable(receivable.id.unwrap()).unwrap();
        assert_eq!(unchanged_receivable.balance(), dec!(500));
    }

    #[test]
    fn posting_an_already_posted_credit_note_is_rejected() {
        let f = fixture();
        let mut credit_note = CreditNote::new(f.company_id, f.customer_id, date(2026, 1, 15));
        credit_note.lines.push(CreditNoteLine::new(f.revenue_account_id, "General credit", dec!(100)));
        let credit_note = f.sales.create_credit_note(&credit_note).unwrap();
        f.sales.post_credit_note(credit_note.id.unwrap(), f.ar_account_id).unwrap();

        let result = f.sales.post_credit_note(credit_note.id.unwrap(), f.ar_account_id);
        assert!(result.is_err());
    }

    #[test]
    fn pos_session_lifecycle_open_drawer_events_close() {
        let f = fixture();
        let terminal = f
            .sales
            .create_terminal(&Terminal::new(f.company_id, f.warehouse_id, "Front Counter"))
            .unwrap();

        let session = f.sales.open_session(&PosSession::open(terminal.id.unwrap(), f.cashier_user_id, dec!(200))).unwrap();
        assert_eq!(session.status, PosSessionStatus::Open);

        f.sales
            .record_cash_drawer_event(&crate::backend::pos::cash_drawer::CashDrawerEvent::new(
                session.id.unwrap(),
                crate::backend::pos::cash_drawer::CashDrawerEventType::CashSale,
                dec!(150),
            ))
            .unwrap();

        let events = f.sales.list_cash_drawer_events(session.id.unwrap()).unwrap();
        let expected = crate::backend::pos::cash_drawer::expected_closing_cash(session.opening_cash, &events);
        assert_eq!(expected, dec!(350));

        let closed = f.sales.close_session(session.id.unwrap(), dec!(350), Utc::now()).unwrap();
        assert_eq!(closed.status, PosSessionStatus::Closed);
        assert_eq!(closed.closing_cash, Some(dec!(350)));
    }

    #[test]
    fn posting_a_pos_sale_posts_the_transaction_and_stores_the_sale() {
        let f = fixture();
        let terminal = f
            .sales
            .create_terminal(&Terminal::new(f.company_id, f.warehouse_id, "Front Counter"))
            .unwrap();
        let session = f.sales.open_session(&PosSession::open(terminal.id.unwrap(), f.cashier_user_id, dec!(200))).unwrap();

        let mut sale = PosSale::new(f.company_id, terminal.id.unwrap(), session.id.unwrap(), date(2026, 1, 15));
        sale.lines.push(crate::backend::pos::sales::PosSaleLine::new(f.product_id, f.revenue_account_id, "Widget", dec!(2), dec!(50)));
        sale.payments.push(PosPayment::new(PaymentMethod::Cash, dec!(100)));

        let (recorded, transaction) = f.sales.post_pos_sale(&sale, &[(PaymentMethod::Cash, f.cash_account_id)]).unwrap();
        assert!(transaction.is_balanced());
        assert_eq!(recorded.transaction_id, transaction.id);
    }
}
