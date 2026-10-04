//! A single debit or credit posting to one account, within a transaction.

use rust_decimal::Decimal;

use crate::backend::core::errors::{AppError, AppResult};
use crate::backend::core::validation::{require_non_negative, Validate};

#[derive(Debug, Clone, PartialEq)]
pub struct TransactionLine {
    pub id: Option<i64>,
    pub transaction_id: Option<i64>,
    pub account_id: i64,
    pub debit: Decimal,
    pub credit: Decimal,
    pub description: Option<String>,
}

impl TransactionLine {
    pub fn debit(account_id: i64, amount: Decimal) -> Self {
        Self {
            id: None,
            transaction_id: None,
            account_id,
            debit: amount,
            credit: Decimal::ZERO,
            description: None,
        }
    }

    pub fn credit(account_id: i64, amount: Decimal) -> Self {
        Self {
            id: None,
            transaction_id: None,
            account_id,
            debit: Decimal::ZERO,
            credit: amount,
            description: None,
        }
    }

    /// Swaps debit and credit -- the building block of a reversal.
    pub fn flipped(&self) -> Self {
        Self {
            id: None,
            transaction_id: None,
            account_id: self.account_id,
            debit: self.credit,
            credit: self.debit,
            description: self.description.clone(),
        }
    }
}

impl Validate for TransactionLine {
    fn validate(&self) -> AppResult<()> {
        if self.account_id <= 0 {
            return Err(AppError::Validation(
                "transaction line must reference a valid account".into(),
            ));
        }
        require_non_negative("debit", self.debit)?;
        require_non_negative("credit", self.credit)?;

        let has_debit = self.debit > Decimal::ZERO;
        let has_credit = self.credit > Decimal::ZERO;
        if has_debit == has_credit {
            return Err(AppError::Validation(
                "a transaction line must have exactly one of debit or credit greater than zero"
                    .into(),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn validate_rejects_both_debit_and_credit_set() {
        let mut line = TransactionLine::debit(1, dec!(100));
        line.credit = dec!(50);
        assert!(line.validate().is_err());
    }

    #[test]
    fn validate_rejects_neither_debit_nor_credit_set() {
        let line = TransactionLine::debit(1, Decimal::ZERO);
        assert!(line.validate().is_err());
    }

    #[test]
    fn validate_rejects_negative_amounts() {
        let line = TransactionLine::debit(1, dec!(-10));
        assert!(line.validate().is_err());
    }

    #[test]
    fn validate_accepts_well_formed_debit_or_credit_line() {
        assert!(TransactionLine::debit(1, dec!(100)).validate().is_ok());
        assert!(TransactionLine::credit(1, dec!(100)).validate().is_ok());
    }

    #[test]
    fn flipped_swaps_debit_and_credit() {
        let line = TransactionLine::debit(1, dec!(100));
        let flipped = line.flipped();
        assert_eq!(flipped.debit, Decimal::ZERO);
        assert_eq!(flipped.credit, dec!(100));
        assert_eq!(flipped.account_id, line.account_id);
    }
}
