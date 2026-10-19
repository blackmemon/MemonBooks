//! A bank account's ledger, derived (never stored) from the posted GL
//! transactions that touch the account's linked GL account -- see
//! `BankingRepository::bank_ledger`. Each entry is the movement on that
//! GL account only; balances follow the account type's sign convention
//! (see [`BankAccountType`]) starting from the account's opening balance.

use chrono::{NaiveDate, NaiveDateTime};
use rust_decimal::Decimal;

use crate::backend::banking::bank_accounts::BankAccountType;

/// One other line of the same GL transaction, for the entry's details.
#[derive(Debug, Clone, PartialEq)]
pub struct BankLedgerLine {
    pub account_code: String,
    pub account_name: String,
    pub debit: Decimal,
    pub credit: Decimal,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BankLedgerEntry {
    pub transaction_id: i64,
    pub date: NaiveDate,
    /// When the transaction was posted, if recorded.
    pub posted_at: Option<NaiveDateTime>,
    /// The originating document's kind, e.g. "Vendor Payment".
    pub kind: String,
    /// The originating document, e.g. `PAY-3`, or `TX-12` for a
    /// transaction no document claims.
    pub reference: String,
    pub description: String,
    pub debit: Decimal,
    pub credit: Decimal,
    pub running_balance: Decimal,
    /// Every line of the GL transaction, this account's included.
    pub lines: Vec<BankLedgerLine>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BankBalances {
    pub opening: Decimal,
    /// Total of the debit-side movements.
    pub debit: Decimal,
    /// Total of the credit-side movements.
    pub credit: Decimal,
    pub account_type: BankAccountType,
}

impl BankBalances {
    pub fn net(&self) -> Decimal {
        self.opening + self.account_type.net_movement(self.debit, self.credit)
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct BankLedger {
    pub account_type: BankAccountType,
    pub opening_balance: Decimal,
    /// Chronological, with `running_balance` filled in.
    pub entries: Vec<BankLedgerEntry>,
}

impl BankLedger {
    /// Orders `entries` by date then transaction id and fills in each
    /// running balance from `opening_balance`.
    pub fn new(
        account_type: BankAccountType,
        opening_balance: Decimal,
        mut entries: Vec<BankLedgerEntry>,
    ) -> Self {
        entries.sort_by_key(|e| (e.date, e.transaction_id));
        let mut balance = opening_balance;
        for entry in &mut entries {
            balance += account_type.net_movement(entry.debit, entry.credit);
            entry.running_balance = balance;
        }
        Self {
            account_type,
            opening_balance,
            entries,
        }
    }

    pub fn balances(&self) -> BankBalances {
        BankBalances {
            opening: self.opening_balance,
            debit: self.entries.iter().map(|e| e.debit).sum(),
            credit: self.entries.iter().map(|e| e.credit).sum(),
            account_type: self.account_type,
        }
    }

    pub fn last_transaction_date(&self) -> Option<NaiveDate> {
        self.entries.iter().map(|e| e.date).max()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn entry(id: i64, day: u32, debit: Decimal, credit: Decimal) -> BankLedgerEntry {
        BankLedgerEntry {
            transaction_id: id,
            date: NaiveDate::from_ymd_opt(2026, 1, day).unwrap(),
            posted_at: None,
            kind: "Journal Entry".into(),
            reference: format!("TX-{id}"),
            description: String::new(),
            debit,
            credit,
            running_balance: Decimal::ZERO,
            lines: Vec::new(),
        }
    }

    fn ledger(
        kind: BankAccountType,
        opening: Decimal,
        entries: Vec<BankLedgerEntry>,
    ) -> BankLedger {
        BankLedger::new(kind, opening, entries)
    }

    #[test]
    fn zero_transactions_leave_only_the_opening_balance() {
        let ledger = ledger(BankAccountType::Debit, dec!(0), vec![]);
        let balances = ledger.balances();
        assert_eq!(balances.debit, dec!(0));
        assert_eq!(balances.credit, dec!(0));
        assert_eq!(balances.net(), dec!(0));
        assert_eq!(ledger.last_transaction_date(), None);
    }

    #[test]
    fn an_opening_balance_alone_is_the_net_balance() {
        for kind in [BankAccountType::Debit, BankAccountType::Credit] {
            let balances = ledger(kind, dec!(250), vec![]).balances();
            assert_eq!(balances.net(), dec!(250));
            assert_eq!(balances.debit + balances.credit, dec!(0));
        }
    }

    #[test]
    fn one_credit_lowers_a_debit_account_and_raises_a_credit_account() {
        let credit = || vec![entry(1, 5, dec!(0), dec!(40))];
        let debit_account = ledger(BankAccountType::Debit, dec!(100), credit());
        assert_eq!(debit_account.balances().credit, dec!(40));
        assert_eq!(debit_account.balances().net(), dec!(60));
        let credit_account = ledger(BankAccountType::Credit, dec!(100), credit());
        assert_eq!(credit_account.balances().net(), dec!(140));
    }

    #[test]
    fn one_debit_raises_a_debit_account_and_lowers_a_credit_account() {
        let debit = || vec![entry(1, 5, dec!(75), dec!(0))];
        let debit_account = ledger(BankAccountType::Debit, dec!(0), debit());
        assert_eq!(debit_account.balances().debit, dec!(75));
        assert_eq!(debit_account.balances().net(), dec!(75));
        let credit_account = ledger(BankAccountType::Credit, dec!(0), debit());
        assert_eq!(credit_account.balances().net(), dec!(-75));
    }

    #[test]
    fn mixed_movements_run_in_date_order_and_agree_with_the_totals() {
        let ledger = ledger(
            BankAccountType::Debit,
            dec!(1000),
            vec![
                entry(3, 20, dec!(0), dec!(300)),
                entry(1, 5, dec!(500), dec!(0)),
                entry(2, 10, dec!(0), dec!(1500)),
            ],
        );
        let ids: Vec<i64> = ledger.entries.iter().map(|e| e.transaction_id).collect();
        assert_eq!(ids, vec![1, 2, 3]);
        assert_eq!(ledger.entries[0].running_balance, dec!(1500));
        assert_eq!(ledger.entries[1].running_balance, dec!(0));
        assert_eq!(ledger.entries[2].running_balance, dec!(-300));
        let balances = ledger.balances();
        assert_eq!(balances.debit, dec!(500));
        assert_eq!(balances.credit, dec!(1800));
        assert_eq!(balances.net(), ledger.entries[2].running_balance);
        assert_eq!(
            ledger.last_transaction_date(),
            NaiveDate::from_ymd_opt(2026, 1, 20)
        );
    }
}
