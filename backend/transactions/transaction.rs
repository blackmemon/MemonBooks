//! A transaction is a set of balanced debit/credit lines posted together,
//! atomically, against a company's ledger. This is the one shape every
//! domain (sales, purchases, POS, expenses, banking, inventory, payroll,
//! and manual journal entries) constructs and hands to the transaction
//! engine -- see Section 4's Core Accounting Flow.

use chrono::NaiveDate;
use rust_decimal::Decimal;

use crate::backend::core::errors::{AppError, AppResult};
use crate::backend::core::validation::Validate;
use crate::backend::transactions::transaction_lines::TransactionLine;
use crate::backend::transactions::transaction_status::TransactionStatus;
use crate::backend::transactions::transaction_types::TransactionType;

#[derive(Debug, Clone, PartialEq)]
pub struct Transaction {
    pub id: Option<i64>,
    pub company_id: i64,
    pub transaction_type: TransactionType,
    pub transaction_date: NaiveDate,
    pub description: Option<String>,
    pub status: TransactionStatus,
    pub lines: Vec<TransactionLine>,
    /// Set on a reversal, pointing back at the transaction it reverses.
    pub reverses_transaction_id: Option<i64>,
    /// Set on the original once a reversal of it has been posted.
    pub reversed_by_transaction_id: Option<i64>,
    pub posted_by_user_id: Option<i64>,
}

impl Transaction {
    pub fn new(
        company_id: i64,
        transaction_type: TransactionType,
        transaction_date: NaiveDate,
    ) -> Self {
        Self {
            id: None,
            company_id,
            transaction_type,
            transaction_date,
            description: None,
            status: TransactionStatus::Draft,
            lines: Vec::new(),
            reverses_transaction_id: None,
            reversed_by_transaction_id: None,
            posted_by_user_id: None,
        }
    }

    pub fn with_line(mut self, line: TransactionLine) -> Self {
        self.lines.push(line);
        self
    }

    pub fn total_debits(&self) -> Decimal {
        self.lines.iter().map(|line| line.debit).sum()
    }

    pub fn total_credits(&self) -> Decimal {
        self.lines.iter().map(|line| line.credit).sum()
    }

    pub fn is_balanced(&self) -> bool {
        self.total_debits() == self.total_credits()
    }

    /// Builds the reversing transaction: every line flipped, dated
    /// `reversal_date`, linked back to this transaction. Does not post it
    /// -- that's `transaction_engine`'s job.
    pub fn build_reversal(&self, reversal_date: NaiveDate) -> AppResult<Self> {
        let original_id = self.id.ok_or_else(|| {
            AppError::Validation("cannot reverse a transaction that hasn't been posted".into())
        })?;
        if self.status != TransactionStatus::Posted {
            return Err(AppError::Validation(
                "only a posted transaction can be reversed".into(),
            ));
        }

        Ok(Self {
            id: None,
            company_id: self.company_id,
            transaction_type: TransactionType::Reversal,
            transaction_date: reversal_date,
            description: Some(format!(
                "Reversal of transaction #{original_id}{}",
                self.description
                    .as_deref()
                    .map(|d| format!(": {d}"))
                    .unwrap_or_default()
            )),
            status: TransactionStatus::Draft,
            lines: self.lines.iter().map(TransactionLine::flipped).collect(),
            reverses_transaction_id: Some(original_id),
            reversed_by_transaction_id: None,
            posted_by_user_id: None,
        })
    }
}

impl Validate for Transaction {
    fn validate(&self) -> AppResult<()> {
        if self.company_id <= 0 {
            return Err(AppError::Validation(
                "transaction must reference a valid company".into(),
            ));
        }
        if self.lines.is_empty() {
            return Err(AppError::Validation(
                "a transaction must have at least one line".into(),
            ));
        }
        for line in &self.lines {
            line.validate()?;
        }
        if !self.is_balanced() {
            return Err(AppError::Validation(format!(
                "unbalanced transaction: total debits {} != total credits {}",
                self.total_debits(),
                self.total_credits()
            )));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    fn balanced_transaction() -> Transaction {
        Transaction::new(1, TransactionType::ManualJournal, date(2026, 1, 15))
            .with_line(TransactionLine::debit(1, dec!(100)))
            .with_line(TransactionLine::credit(2, dec!(100)))
    }

    #[test]
    fn is_balanced_true_when_debits_equal_credits() {
        assert!(balanced_transaction().is_balanced());
    }

    #[test]
    fn is_balanced_false_when_debits_and_credits_differ() {
        let transaction = Transaction::new(1, TransactionType::ManualJournal, date(2026, 1, 15))
            .with_line(TransactionLine::debit(1, dec!(100)))
            .with_line(TransactionLine::credit(2, dec!(99)));
        assert!(!transaction.is_balanced());
    }

    #[test]
    fn validate_rejects_a_transaction_with_no_lines() {
        let transaction = Transaction::new(1, TransactionType::ManualJournal, date(2026, 1, 15));
        assert!(transaction.validate().is_err());
    }

    #[test]
    fn validate_rejects_an_unbalanced_transaction() {
        let transaction = Transaction::new(1, TransactionType::ManualJournal, date(2026, 1, 15))
            .with_line(TransactionLine::debit(1, dec!(100)))
            .with_line(TransactionLine::credit(2, dec!(50)));
        assert!(transaction.validate().is_err());
    }

    #[test]
    fn validate_accepts_a_balanced_multi_line_transaction() {
        let transaction = Transaction::new(1, TransactionType::ManualJournal, date(2026, 1, 15))
            .with_line(TransactionLine::debit(1, dec!(60)))
            .with_line(TransactionLine::debit(2, dec!(40)))
            .with_line(TransactionLine::credit(3, dec!(100)));
        assert!(transaction.validate().is_ok());
    }

    #[test]
    fn build_reversal_requires_a_posted_transaction_with_an_id() {
        let draft = balanced_transaction();
        assert!(draft.build_reversal(date(2026, 2, 1)).is_err());
    }

    #[test]
    fn build_reversal_flips_every_line_and_links_back() {
        let mut posted = balanced_transaction();
        posted.id = Some(42);
        posted.status = TransactionStatus::Posted;

        let reversal = posted.build_reversal(date(2026, 2, 1)).unwrap();
        assert_eq!(reversal.reverses_transaction_id, Some(42));
        assert_eq!(reversal.transaction_type, TransactionType::Reversal);
        assert!(reversal.is_balanced());
        assert_eq!(reversal.lines[0].debit, posted.lines[0].credit);
        assert_eq!(reversal.lines[0].credit, posted.lines[0].debit);
    }
}
