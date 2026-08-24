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

    pub fn post_invoice(&self, invoice_id: i64, ar_account_id: i64) -> AppResult<(Invoice, Transaction, Receivable)> {
        self.sales_repo.post_invoice(invoice_id, ar_account_id)
    }

    /// Applies FIFO across the customer's whole open balance, not a
    /// single invoice -- see `SalesRepository::record_customer_payment`.
    pub fn record_customer_payment(&self, payment: &CustomerPayment, cash_account_id: i64, ar_account_id: i64) -> AppResult<CustomerPayment> {
        self.sales_repo.record_customer_payment(payment, cash_account_id, ar_account_id)
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
