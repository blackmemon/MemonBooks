//! The purchases domain's service layer, for `inventory/bills_view.rs`.
//! Also wraps `PartyRepository` (for vendors) and `AccountingRepository`
//! (for GL accounts) internally -- a service is free to reach across
//! domains internally; the Architecture Rule only forbids the view
//! doing so.

use crate::backend::accounting::accounts_payable::payables::Payable;
use crate::backend::accounting::chart_of_accounts::Account;
use crate::backend::core::errors::AppResult;
use crate::backend::parties::vendors::Vendor;
use crate::backend::purchases::bills::Bill;
use crate::backend::purchases::debit_notes::{DebitNote, DebitNoteLine};
use crate::backend::purchases::payments::VendorPayment;
use crate::backend::repositories::accounting_repository::AccountingRepository;
use crate::backend::repositories::party_repository::PartyRepository;
use crate::backend::repositories::purchase_repository::PurchaseRepository;
use crate::backend::transactions::transaction::Transaction;
use crate::database::database::Database;

#[derive(Clone)]
pub struct PurchaseService {
    purchase_repo: PurchaseRepository,
    party_repo: PartyRepository,
    accounting_repo: AccountingRepository,
}

impl PurchaseService {
    pub fn new(db: Database) -> Self {
        Self {
            purchase_repo: PurchaseRepository::new(db.clone()),
            party_repo: PartyRepository::new(db.clone()),
            accounting_repo: AccountingRepository::new(db),
        }
    }

    pub fn list_vendors(&self, company_id: i64) -> AppResult<Vec<Vendor>> {
        self.party_repo.list_vendors_for_company(company_id)
    }

    /// Read-only AP ledger for one vendor; see `PurchaseRepository::vendor_ledger`.
    pub fn vendor_ledger(
        &self,
        company_id: i64,
        vendor_id: i64,
    ) -> AppResult<crate::backend::purchases::vendor_ledger::VendorLedger> {
        self.purchase_repo.vendor_ledger(company_id, vendor_id)
    }

    pub fn list_gl_accounts(&self, company_id: i64) -> AppResult<Vec<Account>> {
        self.accounting_repo.list_accounts_for_company(company_id)
    }

    pub fn list_bills(&self, company_id: i64) -> AppResult<Vec<Bill>> {
        self.purchase_repo.list_bills_for_company(company_id)
    }

    pub fn create_bill(&self, bill: &Bill) -> AppResult<Bill> {
        self.purchase_repo.create_bill(bill)
    }

    /// Full detail (with lines) of one bill -- `list_bills` only returns
    /// headers, so editing a draft needs this to prefill its line.
    pub fn get_bill(&self, bill_id: i64) -> AppResult<Bill> {
        self.purchase_repo.get_bill(bill_id)
    }

    /// Updates a draft bill in place -- see
    /// `PurchaseRepository::update_bill`. Rejects anything not Draft.
    pub fn update_bill(&self, bill: &Bill) -> AppResult<()> {
        self.purchase_repo.update_bill(bill)
    }

    /// Deletes a draft bill outright -- see
    /// `PurchaseRepository::delete_bill`. Rejects anything not Draft.
    pub fn delete_bill(&self, bill_id: i64) -> AppResult<()> {
        self.purchase_repo.delete_bill(bill_id)
    }

    /// Voids a draft bill -- see `PurchaseRepository::void_bill`.
    pub fn void_bill(&self, bill_id: i64) -> AppResult<Bill> {
        self.purchase_repo.void_bill(bill_id)
    }

    pub fn post_bill(
        &self,
        bill_id: i64,
        ap_account_id: i64,
    ) -> AppResult<(Bill, Transaction, Payable)> {
        self.purchase_repo.post_bill(bill_id, ap_account_id)
    }

    /// Creates and posts a debit note against exactly one bill's own
    /// payable, in one step -- mirrors `record_payment_for_bill`'s
    /// scoping: `PurchaseRepository::post_debit_note` rejects (rather
    /// than caps) an amount exceeding that bill's own remaining balance,
    /// and never touches any other open bill for the same vendor.
    #[allow(clippy::too_many_arguments)]
    pub fn create_and_post_debit_note_for_bill(
        &self,
        bill_id: i64,
        debit_note_date: chrono::NaiveDate,
        account_id: i64,
        description: String,
        amount: rust_decimal::Decimal,
        ap_account_id: i64,
    ) -> AppResult<(DebitNote, Transaction)> {
        let bill = self.purchase_repo.get_bill(bill_id)?;
        let payable = self.purchase_repo.open_payable_for_bill(bill_id)?;
        let mut debit_note = DebitNote::new(bill.company_id, bill.vendor_id, debit_note_date);
        debit_note.payable_id = payable.id;
        debit_note
            .lines
            .push(DebitNoteLine::new(account_id, description, amount));
        let created = self.purchase_repo.create_debit_note(&debit_note)?;
        self.purchase_repo
            .post_debit_note(created.id.unwrap_or_default(), ap_account_id)
    }

    /// Applies FIFO across the vendor's whole open balance, not a single
    /// bill -- see `PurchaseRepository::record_vendor_payment`. Not
    /// currently routed to from any view; kept for a possible future
    /// vendor-level "make a payment" action.
    pub fn record_vendor_payment(
        &self,
        payment: &VendorPayment,
        ap_account_id: i64,
        cash_account_id: i64,
    ) -> AppResult<VendorPayment> {
        self.purchase_repo
            .record_vendor_payment(payment, ap_account_id, cash_account_id)
    }

    /// Applies only to the one bill's own payable, capped at its own
    /// remaining balance -- see `PurchaseRepository::
    /// record_payment_for_bill`. This is what Bills' row-level "Pay"
    /// actually calls.
    pub fn record_payment_for_bill(
        &self,
        bill_id: i64,
        payment_date: chrono::NaiveDate,
        amount: rust_decimal::Decimal,
        ap_account_id: i64,
        cash_account_id: i64,
    ) -> AppResult<VendorPayment> {
        self.purchase_repo.record_payment_for_bill(
            bill_id,
            payment_date,
            amount,
            ap_account_id,
            cash_account_id,
        )
    }

    /// The bill's own remaining balance -- for prefilling a payment form
    /// before the amount is actually submitted.
    pub fn bill_remaining_balance(&self, bill_id: i64) -> AppResult<rust_decimal::Decimal> {
        self.purchase_repo.bill_remaining_balance(bill_id)
    }
}
