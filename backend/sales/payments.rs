//! Recording a payment received from a customer: posts a transaction
//! (debit cash, credit AR) and applies the amount across the customer's
//! open receivables via `accounts_receivable::collections`.

use chrono::NaiveDate;
use rust_decimal::Decimal;

use crate::backend::core::errors::{AppError, AppResult};
use crate::backend::core::validation::{require_positive, Validate};
use crate::backend::transactions::transaction::Transaction;
use crate::backend::transactions::transaction_lines::TransactionLine;
use crate::backend::transactions::transaction_types::TransactionType;

#[derive(Debug, Clone, PartialEq)]
pub struct CustomerPayment {
    pub id: Option<i64>,
    pub company_id: i64,
    pub customer_id: i64,
    pub payment_date: NaiveDate,
    pub amount: Decimal,
    pub transaction_id: Option<i64>,
}

impl CustomerPayment {
    pub fn new(company_id: i64, customer_id: i64, payment_date: NaiveDate, amount: Decimal) -> Self {
        Self {
            id: None,
            company_id,
            customer_id,
            payment_date,
            amount,
            transaction_id: None,
        }
    }

    pub fn build_transaction(&self, cash_account_id: i64, ar_account_id: i64) -> Transaction {
        Transaction::new(self.company_id, TransactionType::CustomerPayment, self.payment_date)
            .with_line(TransactionLine::debit(cash_account_id, self.amount))
            .with_line(TransactionLine::credit(ar_account_id, self.amount))
    }
}

impl Validate for CustomerPayment {
    fn validate(&self) -> AppResult<()> {
        if self.company_id <= 0 {
            return Err(AppError::Validation("payment must reference a valid company".into()));
        }
        if self.customer_id <= 0 {
            return Err(AppError::Validation("payment must reference a valid customer".into()));
        }
        require_positive("payment amount", self.amount)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn build_transaction_debits_cash_and_credits_ar() {
        let payment = CustomerPayment::new(1, 1, NaiveDate::from_ymd_opt(2026, 2, 1).unwrap(), dec!(500));
        let transaction = payment.build_transaction(10, 20);
        assert!(transaction.is_balanced());
        assert_eq!(transaction.lines[0].account_id, 10);
        assert_eq!(transaction.lines[0].debit, dec!(500));
        assert_eq!(transaction.lines[1].account_id, 20);
        assert_eq!(transaction.lines[1].credit, dec!(500));
    }

    #[test]
    fn validate_rejects_non_positive_amount() {
        let payment = CustomerPayment::new(1, 1, NaiveDate::from_ymd_opt(2026, 2, 1).unwrap(), dec!(0));
        assert!(payment.validate().is_err());
    }
}
