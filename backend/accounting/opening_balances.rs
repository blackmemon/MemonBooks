//! Setting up a company's (or a new fiscal year's) opening balances is
//! itself just a transaction -- `TransactionType::OpeningBalance` -- with
//! one line per account, dated at the start of the books. There's no
//! separate "opening balance" column anywhere: the transaction engine's
//! existing balance check is exactly the constraint that makes a set of
//! opening balances valid (they must already balance, the same as a real
//! pre-existing trial balance would).

use chrono::NaiveDate;
use rust_decimal::Decimal;

use crate::backend::transactions::transaction::Transaction;
use crate::backend::transactions::transaction_lines::TransactionLine;
use crate::backend::transactions::transaction_types::TransactionType;

pub struct OpeningBalances {
    transaction: Transaction,
}

impl OpeningBalances {
    pub fn new(company_id: i64, as_of: NaiveDate) -> Self {
        let mut transaction = Transaction::new(company_id, TransactionType::OpeningBalance, as_of);
        transaction.description = Some("Opening balances".into());
        Self { transaction }
    }

    /// Adds one account's opening balance, under the trial balance
    /// convention: positive is a debit balance, negative is a credit
    /// balance.
    pub fn with_balance(mut self, account_id: i64, balance: Decimal) -> Self {
        if balance > Decimal::ZERO {
            self.transaction.lines.push(TransactionLine::debit(account_id, balance));
        } else if balance < Decimal::ZERO {
            self.transaction.lines.push(TransactionLine::credit(account_id, -balance));
        }
        self
    }

    /// Hands the built transaction over to the transaction engine, via
    /// `AccountingRepository::post_transaction`. Will fail validation
    /// (and post nothing) if the given balances don't themselves balance.
    pub fn into_transaction(self) -> Transaction {
        self.transaction
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::core::validation::Validate;
    use rust_decimal_macros::dec;

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    #[test]
    fn builds_a_balanced_opening_transaction_from_a_valid_trial_balance() {
        let transaction = OpeningBalances::new(1, date(2026, 1, 1))
            .with_balance(1, dec!(10000)) // Cash, debit balance
            .with_balance(2, dec!(-10000)) // Owner's Capital, credit balance
            .into_transaction();

        assert_eq!(transaction.transaction_type, TransactionType::OpeningBalance);
        assert!(transaction.validate().is_ok());
    }

    #[test]
    fn an_unbalanced_set_of_opening_balances_fails_validation() {
        let transaction = OpeningBalances::new(1, date(2026, 1, 1))
            .with_balance(1, dec!(10000))
            .with_balance(2, dec!(-9000))
            .into_transaction();

        assert!(transaction.validate().is_err());
    }

    #[test]
    fn zero_balances_do_not_produce_lines() {
        let transaction = OpeningBalances::new(1, date(2026, 1, 1)).with_balance(1, Decimal::ZERO).into_transaction();
        assert!(transaction.lines.is_empty());
    }
}
