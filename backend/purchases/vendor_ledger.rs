//! A vendor's accounts-payable ledger, derived (never stored) from the
//! posted transactions behind the vendor's bills, debit notes and
//! payments -- see `PurchaseRepository::vendor_ledger`. Each entry is the
//! movement on the AP control account only, so the running balance is
//! what the company owes the vendor: credits (bills) increase it, debits
//! (payments, debit notes) decrease it. A negative balance means the
//! vendor owes the company (an overpayment or unapplied debit note).

use chrono::NaiveDate;
use rust_decimal::Decimal;

use crate::backend::accounting::accounts_payable::aging::{self, PayablesAgingSummary};
use crate::backend::accounting::accounts_payable::payables::Payable;

/// Column titles of the ledger's views: the transaction history, each
/// side of the T-account, the trial balance and the open bills.
pub const HISTORY_COLUMNS: [&str; 7] = [
    "Date",
    "Reference",
    "Type",
    "Description",
    "Debit",
    "Credit",
    "Balance",
];
pub const T_ACCOUNT_COLUMNS: [&str; 3] = ["Date", "Reference", "Amount"];
pub const TRIAL_BALANCE_COLUMNS: [&str; 4] = ["Code", "Account", "Debit", "Credit"];
pub const OPEN_BILL_COLUMNS: [&str; 5] = ["Bill", "Due Date", "Status", "Amount", "Balance"];

#[derive(Debug, Clone, PartialEq)]
pub struct VendorLedgerEntry {
    pub transaction_id: i64,
    pub date: NaiveDate,
    /// "Bill", "Payment", "Debit Note" or "Reversal".
    pub kind: String,
    /// The originating document, e.g. `BILL-3`; a reversal names the
    /// document it reverses.
    pub reference: String,
    pub description: String,
    pub debit: Decimal,
    pub credit: Decimal,
    pub running_balance: Decimal,
}

/// One ledger account touched by the vendor's transactions.
#[derive(Debug, Clone, PartialEq)]
pub struct VendorAccountTotal {
    pub code: String,
    pub name: String,
    pub debit: Decimal,
    pub credit: Decimal,
}

impl VendorAccountTotal {
    pub fn net(&self) -> Decimal {
        self.debit - self.credit
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct VendorLedger {
    pub currency: String,
    /// Chronological, with `running_balance` filled in.
    pub entries: Vec<VendorLedgerEntry>,
    pub accounts: Vec<VendorAccountTotal>,
    pub open_payables: Vec<Payable>,
}

impl VendorLedger {
    pub fn total_debit(&self) -> Decimal {
        self.entries.iter().map(|e| e.debit).sum()
    }

    pub fn total_credit(&self) -> Decimal {
        self.entries.iter().map(|e| e.credit).sum()
    }

    /// Positive: payable to the vendor. Negative: receivable from them.
    pub fn outstanding(&self) -> Decimal {
        self.total_credit() - self.total_debit()
    }

    pub fn last_transaction_date(&self) -> Option<NaiveDate> {
        self.entries.iter().map(|e| e.date).max()
    }

    pub fn first_transaction_date(&self) -> Option<NaiveDate> {
        self.entries.iter().map(|e| e.date).min()
    }

    pub fn aging(&self, as_of: NaiveDate) -> PayablesAgingSummary {
        aging::summarize(&self.open_payables, as_of)
    }
}

/// Orders entries by date then transaction id and fills in the running
/// balance (opening balance is zero: vendors carry none of their own).
pub fn with_running_balances(mut entries: Vec<VendorLedgerEntry>) -> Vec<VendorLedgerEntry> {
    entries.sort_by_key(|e| (e.date, e.transaction_id));
    let mut balance = Decimal::ZERO;
    for entry in &mut entries {
        balance += entry.credit - entry.debit;
        entry.running_balance = balance;
    }
    entries
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn entry(id: i64, day: u32, debit: Decimal, credit: Decimal) -> VendorLedgerEntry {
        VendorLedgerEntry {
            transaction_id: id,
            date: NaiveDate::from_ymd_opt(2026, 1, day).unwrap(),
            kind: "Bill".into(),
            reference: format!("BILL-{id}"),
            description: String::new(),
            debit,
            credit,
            running_balance: Decimal::ZERO,
        }
    }

    #[test]
    fn running_balance_follows_date_order_and_credit_minus_debit() {
        let entries = with_running_balances(vec![
            entry(2, 10, dec!(40), dec!(0)),
            entry(1, 5, dec!(0), dec!(100)),
        ]);
        assert_eq!(entries[0].transaction_id, 1);
        assert_eq!(entries[0].running_balance, dec!(100));
        assert_eq!(entries[1].running_balance, dec!(60));
    }

    #[test]
    fn totals_and_outstanding_agree_with_the_closing_running_balance() {
        let ledger = VendorLedger {
            entries: with_running_balances(vec![
                entry(1, 5, dec!(0), dec!(100)),
                entry(2, 10, dec!(130), dec!(0)),
            ]),
            ..Default::default()
        };
        assert_eq!(ledger.total_debit(), dec!(130));
        assert_eq!(ledger.total_credit(), dec!(100));
        assert_eq!(ledger.outstanding(), dec!(-30));
        assert_eq!(
            ledger.entries.last().unwrap().running_balance,
            ledger.outstanding()
        );
    }
}
