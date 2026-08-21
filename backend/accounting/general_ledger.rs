//! The general ledger: for one account, the chronological list of posted
//! movements and the running balance after each. Per Architecture Rules
//! (Section 3), this derives from `transactions`/`transaction_lines`
//! rather than being stored separately -- `AccountingRepository` fetches
//! the raw postings in order and this module turns them into a running
//! balance.

use chrono::NaiveDate;
use rust_decimal::Decimal;

use crate::backend::accounting::accounting_rules::NormalBalance;

#[derive(Debug, Clone, PartialEq)]
pub struct GeneralLedgerEntry {
    pub transaction_id: i64,
    pub transaction_date: NaiveDate,
    pub description: Option<String>,
    pub debit: Decimal,
    pub credit: Decimal,
    pub running_balance: Decimal,
}

/// A single posting to an account, before its running balance is known.
#[derive(Debug, Clone, Copy)]
pub struct Posting {
    pub transaction_id: i64,
    pub transaction_date: NaiveDate,
    pub debit: Decimal,
    pub credit: Decimal,
}

/// The amount a posting moves an account's balance, which side depends on
/// the account's normal balance: a debit *increases* a normal-debit
/// account and *decreases* a normal-credit one, and vice versa for a
/// credit.
fn signed_effect(normal_balance: NormalBalance, debit: Decimal, credit: Decimal) -> Decimal {
    match normal_balance {
        NormalBalance::Debit => debit - credit,
        NormalBalance::Credit => credit - debit,
    }
}

/// Turns `postings` (already ordered chronologically) plus an opening
/// balance into the running balance after each one.
pub fn running_balances(normal_balance: NormalBalance, opening_balance: Decimal, postings: &[Posting]) -> Vec<Decimal> {
    let mut balance = opening_balance;
    postings
        .iter()
        .map(|posting| {
            balance += signed_effect(normal_balance, posting.debit, posting.credit);
            balance
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    fn posting(id: i64, debit: Decimal, credit: Decimal) -> Posting {
        Posting {
            transaction_id: id,
            transaction_date: date(2026, 1, 1),
            debit,
            credit,
        }
    }

    #[test]
    fn debit_normal_account_balance_grows_with_debits() {
        let postings = vec![posting(1, dec!(100), dec!(0)), posting(2, dec!(0), dec!(40))];
        let balances = running_balances(NormalBalance::Debit, dec!(0), &postings);
        assert_eq!(balances, vec![dec!(100), dec!(60)]);
    }

    #[test]
    fn credit_normal_account_balance_grows_with_credits() {
        let postings = vec![posting(1, dec!(0), dec!(500)), posting(2, dec!(200), dec!(0))];
        let balances = running_balances(NormalBalance::Credit, dec!(0), &postings);
        assert_eq!(balances, vec![dec!(500), dec!(300)]);
    }

    #[test]
    fn opening_balance_carries_forward() {
        let postings = vec![posting(1, dec!(50), dec!(0))];
        let balances = running_balances(NormalBalance::Debit, dec!(1000), &postings);
        assert_eq!(balances, vec![dec!(1050)]);
    }

    #[test]
    fn no_postings_yields_no_balances() {
        let balances = running_balances(NormalBalance::Debit, dec!(1000), &[]);
        assert!(balances.is_empty());
    }
}
