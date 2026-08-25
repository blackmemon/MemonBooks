//! The sales domain's service layer, for `sales/invoices_view.rs` and
//! `pos/pos_view.rs` -- POS has no dedicated repository or service of
//! its own -- it was folded into `SalesRepository` back in the
//! repository layer, so it stays folded into `sales_service.rs` here
//! too. Also wraps `PartyRepository` (customers), `AccountingRepository`
//! (GL accounts) and `InventoryRepository` (products, for POS) internally
//! -- a service is free to reach across domains internally; the
//! Architecture Rule only forbids the view doing so.

use crate::backend::accounting::accounts_receivable::receivables::Receivable;
use crate::backend::accounting::chart_of_accounts::Account;
use crate::backend::core::errors::AppResult;
use crate::backend::inventory::products::Product;
use crate::backend::parties::customers::Customer;
use crate::backend::pos::payments::PaymentMethod;
use crate::backend::pos::sales::PosSale;
use crate::backend::pos::sessions::PosSession;
use crate::backend::pos::terminals::Terminal;
use crate::backend::repositories::accounting_repository::AccountingRepository;
use crate::backend::repositories::inventory_repository::InventoryRepository;
use crate::backend::repositories::party_repository::PartyRepository;
use crate::backend::repositories::sales_repository::SalesRepository;
use crate::backend::sales::credit_notes::{CreditNote, CreditNoteLine};
use crate::backend::sales::invoices::Invoice;
use crate::backend::sales::payments::CustomerPayment;
use crate::backend::transactions::transaction::Transaction;
use crate::database::database::Database;

#[derive(Clone)]
pub struct SalesService {
    sales_repo: SalesRepository,
    party_repo: PartyRepository,
    accounting_repo: AccountingRepository,
    inventory_repo: InventoryRepository,
}

impl SalesService {
    pub fn new(db: Database) -> Self {
        Self {
            sales_repo: SalesRepository::new(db.clone()),
            party_repo: PartyRepository::new(db.clone()),
            accounting_repo: AccountingRepository::new(db.clone()),
            inventory_repo: InventoryRepository::new(db),
        }
    }

    pub fn list_customers(&self, company_id: i64) -> AppResult<Vec<Customer>> {
        self.party_repo.list_customers_for_company(company_id)
    }

    pub fn list_gl_accounts(&self, company_id: i64) -> AppResult<Vec<Account>> {
        self.accounting_repo.list_accounts_for_company(company_id)
    }

    pub fn list_invoices(&self, company_id: i64) -> AppResult<Vec<Invoice>> {
        self.sales_repo.list_invoices_for_company(company_id)
    }

    pub fn create_invoice(&self, invoice: &Invoice) -> AppResult<Invoice> {
        self.sales_repo.create_invoice(invoice)
    }

    /// Full detail (with lines) of one invoice -- `list_invoices` only
    /// returns headers, so editing a draft needs this to prefill its
    /// line.
    pub fn get_invoice(&self, invoice_id: i64) -> AppResult<Invoice> {
        self.sales_repo.get_invoice(invoice_id)
    }

    /// Updates a draft invoice in place -- see
    /// `SalesRepository::update_invoice`. Rejects anything not Draft.
    pub fn update_invoice(&self, invoice: &Invoice) -> AppResult<()> {
        self.sales_repo.update_invoice(invoice)
    }

    /// Deletes a draft invoice outright -- see
    /// `SalesRepository::delete_invoice`. Rejects anything not Draft.
    pub fn delete_invoice(&self, invoice_id: i64) -> AppResult<()> {
        self.sales_repo.delete_invoice(invoice_id)
    }

    pub fn post_invoice(&self, invoice_id: i64, ar_account_id: i64) -> AppResult<(Invoice, Transaction, Receivable)> {
        self.sales_repo.post_invoice(invoice_id, ar_account_id)
    }

    /// Applies FIFO across the customer's whole open balance, not a
    /// single invoice -- see `SalesRepository::record_customer_payment`.
    /// Not currently routed to from any view; kept for a possible future
    /// customer-level "make a payment" action.
    pub fn record_customer_payment(&self, payment: &CustomerPayment, cash_account_id: i64, ar_account_id: i64) -> AppResult<CustomerPayment> {
        self.sales_repo.record_customer_payment(payment, cash_account_id, ar_account_id)
    }

    /// Applies only to the one invoice's own receivable, capped at its
    /// own remaining balance -- see `SalesRepository::
    /// record_payment_for_invoice`. This is what Invoices' row-level
    /// "Record Payment" actually calls.
    pub fn record_payment_for_invoice(&self, invoice_id: i64, payment_date: chrono::NaiveDate, amount: rust_decimal::Decimal, cash_account_id: i64, ar_account_id: i64) -> AppResult<CustomerPayment> {
        self.sales_repo.record_payment_for_invoice(invoice_id, payment_date, amount, cash_account_id, ar_account_id)
    }

    /// The invoice's own remaining balance -- for prefilling a payment
    /// or credit-note form before the amount is actually submitted.
    pub fn invoice_remaining_balance(&self, invoice_id: i64) -> AppResult<rust_decimal::Decimal> {
        self.sales_repo.invoice_remaining_balance(invoice_id)
    }

    /// Voids a draft invoice -- see `SalesRepository::void_invoice`.
    pub fn void_invoice(&self, invoice_id: i64) -> AppResult<Invoice> {
        self.sales_repo.void_invoice(invoice_id)
    }

    /// Creates and posts a credit note against exactly one invoice's own
    /// receivable, in one step -- mirrors `record_payment_for_invoice`'s
    /// scoping: `SalesRepository::post_credit_note` rejects (rather than
    /// caps) an amount exceeding that invoice's own remaining balance,
    /// and never touches any other open invoice for the same customer.
    #[allow(clippy::too_many_arguments)]
    pub fn create_and_post_credit_note_for_invoice(
        &self,
        invoice_id: i64,
        credit_note_date: chrono::NaiveDate,
        revenue_account_id: i64,
        description: String,
        amount: rust_decimal::Decimal,
        ar_account_id: i64,
    ) -> AppResult<(CreditNote, Transaction)> {
        let invoice = self.sales_repo.get_invoice(invoice_id)?;
        let receivable = self.sales_repo.open_receivable_for_invoice(invoice_id)?;
        let mut credit_note = CreditNote::new(invoice.company_id, invoice.customer_id, credit_note_date);
        credit_note.receivable_id = receivable.id;
        credit_note.lines.push(CreditNoteLine::new(revenue_account_id, description, amount));
        let created = self.sales_repo.create_credit_note(&credit_note)?;
        self.sales_repo.post_credit_note(created.id.unwrap_or_default(), ar_account_id)
    }

    pub fn list_terminals(&self, company_id: i64) -> AppResult<Vec<Terminal>> {
        self.sales_repo.list_terminals_for_company(company_id)
    }

    pub fn create_terminal(&self, terminal: &Terminal) -> AppResult<Terminal> {
        self.sales_repo.create_terminal(terminal)
    }

    pub fn list_products(&self, company_id: i64) -> AppResult<Vec<Product>> {
        self.inventory_repo.list_products_for_company(company_id)
    }

    pub fn open_session(&self, session: &PosSession) -> AppResult<PosSession> {
        self.sales_repo.open_session(session)
    }

    pub fn post_pos_sale(&self, sale: &PosSale, payment_accounts: &[(PaymentMethod, i64)]) -> AppResult<(PosSale, Transaction)> {
        self.sales_repo.post_pos_sale(sale, payment_accounts)
    }
}
