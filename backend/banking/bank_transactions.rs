//! A line on a bank statement -- a staging/reference record for
//! reconciliation, distinct from the GL `Transaction`s it gets matched
//! against (most bank transactions correspond to a payment/receipt
//! already posted in steps 9-10; reconciliation is about confirming
//! that, not re-posting it).

use chrono::NaiveDate;
use rust_decimal::Decimal;

use crate::backend::core::errors::{AppError, AppResult};
use crate::backend::core::validation::{require_non_empty, Validate};
use crate::backend::transactions::transaction::Transaction;
use crate::backend::transactions::transaction_lines::TransactionLine;
use crate::backend::transactions::transaction_types::TransactionType;

#[derive(Debug, Clone, PartialEq)]
pub struct BankTransaction {
    pub id: Option<i64>,
    pub company_id: i64,
    pub bank_account_id: i64,
    pub transaction_date: NaiveDate,
    pub description: String,
    /// Positive for a deposit, negative for a withdrawal.
    pub amount: Decimal,
    /// The GL transaction this reconciles against, once matched.
    pub matched_transaction_id: Option<i64>,
}

impl BankTransaction {
    pub fn new(
        company_id: i64,
        bank_account_id: i64,
        transaction_date: NaiveDate,
        description: impl Into<String>,
        amount: Decimal,
    ) -> Self {
        Self {
            id: None,
            company_id,
            bank_account_id,
            transaction_date,
            description: description.into(),
            amount,
            matched_transaction_id: None,
        }
    }

    pub fn is_reconciled(&self) -> bool {
        self.matched_transaction_id.is_some()
    }

    /// For a bank-only item with no corresponding GL entry yet (a bank
    /// fee, interest earned, ...): builds the GL transaction that would
    /// bring the books in line with the statement. A deposit debits the
    /// bank's GL account; a withdrawal credits it, both against
    /// `offset_account_id` (e.g. a bank charges expense account).
    pub fn build_adjustment_transaction(
        &self,
        bank_gl_account_id: i64,
        offset_account_id: i64,
    ) -> Transaction {
        let mut transaction = Transaction::new(
            self.company_id,
            TransactionType::BankTransaction,
            self.transaction_date,
        );
        transaction.description = Some(self.description.clone());
        if self.amount >= Decimal::ZERO {
            transaction
                .lines
                .push(TransactionLine::debit(bank_gl_account_id, self.amount));
            transaction
                .lines
                .push(TransactionLine::credit(offset_account_id, self.amount));
        } else {
            transaction
                .lines
                .push(TransactionLine::credit(bank_gl_account_id, -self.amount));
            transaction
                .lines
                .push(TransactionLine::debit(offset_account_id, -self.amount));
        }
        transaction
    }
}

impl Validate for BankTransaction {
    fn validate(&self) -> AppResult<()> {
        if self.company_id <= 0 {
            return Err(AppError::Validation(
                "bank transaction must reference a valid company".into(),
            ));
        }
        if self.bank_account_id <= 0 {
            return Err(AppError::Validation(
                "bank transaction must reference a valid bank account".into(),
            ));
        }
        require_non_empty("bank transaction description", &self.description)?;
        if self.amount == Decimal::ZERO {
            return Err(AppError::Validation(
                "bank transaction amount must not be zero".into(),
            ));
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

    #[test]
    fn validate_rejects_zero_amount() {
        let transaction = BankTransaction::new(1, 1, date(2026, 1, 1), "Fee", Decimal::ZERO);
        assert!(transaction.validate().is_err());
    }

    #[test]
    fn build_adjustment_transaction_debits_bank_for_a_deposit() {
        let transaction = BankTransaction::new(1, 1, date(2026, 1, 1), "Interest earned", dec!(5));
        let gl = transaction.build_adjustment_transaction(10, 20);
        assert!(gl.is_balanced());
        assert_eq!(gl.lines[0].account_id, 10);
        assert_eq!(gl.lines[0].debit, dec!(5));
    }

    #[test]
    fn build_adjustment_transaction_credits_bank_for_a_withdrawal() {
        let transaction = BankTransaction::new(1, 1, date(2026, 1, 1), "Bank fee", dec!(-15));
        let gl = transaction.build_adjustment_transaction(10, 20);
        assert!(gl.is_balanced());
        assert_eq!(gl.lines[0].account_id, 10);
        assert_eq!(gl.lines[0].credit, dec!(15));
    }

    #[test]
    fn is_reconciled_reflects_whether_it_has_been_matched() {
        let mut transaction = BankTransaction::new(1, 1, date(2026, 1, 1), "Fee", dec!(-5));
        assert!(!transaction.is_reconciled());
        transaction.matched_transaction_id = Some(42);
        assert!(transaction.is_reconciled());
    }
}
