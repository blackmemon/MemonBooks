//! A credit note: reduces what a customer owes (a return or adjustment
//! against a previously issued invoice). Posting one is the mirror image
//! of posting an invoice -- credit AR, debit each line's revenue account
//! -- and it reduces the linked receivable's balance directly rather than
//! creating a new one.

use chrono::NaiveDate;
use rust_decimal::Decimal;

use crate::backend::core::errors::{AppError, AppResult};
use crate::backend::core::validation::{require_non_empty, require_positive, Validate};
use crate::backend::transactions::transaction::Transaction;
use crate::backend::transactions::transaction_lines::TransactionLine;
use crate::backend::transactions::transaction_types::TransactionType;

#[derive(Debug, Clone, PartialEq)]
pub struct CreditNoteLine {
    pub revenue_account_id: i64,
    pub description: String,
    pub amount: Decimal,
}

impl CreditNoteLine {
    pub fn new(revenue_account_id: i64, description: impl Into<String>, amount: Decimal) -> Self {
        Self {
            revenue_account_id,
            description: description.into(),
            amount,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CreditNote {
    pub id: Option<i64>,
    pub company_id: i64,
    pub customer_id: i64,
    /// The receivable this credit note reduces, if it's against a
    /// specific invoice rather than a general credit.
    pub receivable_id: Option<i64>,
    pub credit_note_date: NaiveDate,
    pub lines: Vec<CreditNoteLine>,
    pub transaction_id: Option<i64>,
}

impl CreditNote {
    pub fn new(company_id: i64, customer_id: i64, credit_note_date: NaiveDate) -> Self {
        Self {
            id: None,
            company_id,
            customer_id,
            receivable_id: None,
            credit_note_date,
            lines: Vec::new(),
            transaction_id: None,
        }
    }

    pub fn total(&self) -> Decimal {
        self.lines.iter().map(|line| line.amount).sum()
    }

    pub fn build_transaction(&self, ar_account_id: i64) -> Transaction {
        let mut transaction = Transaction::new(
            self.company_id,
            TransactionType::SalesInvoice,
            self.credit_note_date,
        );
        transaction.description = Some(format!("Credit note for customer #{}", self.customer_id));
        transaction
            .lines
            .push(TransactionLine::credit(ar_account_id, self.total()));
        for line in &self.lines {
            transaction.lines.push(TransactionLine {
                description: Some(line.description.clone()),
                ..TransactionLine::debit(line.revenue_account_id, line.amount)
            });
        }
        transaction
    }
}

impl Validate for CreditNote {
    fn validate(&self) -> AppResult<()> {
        if self.company_id <= 0 {
            return Err(AppError::Validation(
                "credit note must reference a valid company".into(),
            ));
        }
        if self.customer_id <= 0 {
            return Err(AppError::Validation(
                "credit note must reference a valid customer".into(),
            ));
        }
        if self.lines.is_empty() {
            return Err(AppError::Validation(
                "a credit note must have at least one line".into(),
            ));
        }
        for line in &self.lines {
            require_non_empty("credit note line description", &line.description)?;
            require_positive("credit note line amount", line.amount)?;
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
    fn build_transaction_credits_ar_and_debits_revenue() {
        let mut credit_note = CreditNote::new(1, 1, date(2026, 2, 1));
        credit_note
            .lines
            .push(CreditNoteLine::new(10, "Return", dec!(100)));

        let transaction = credit_note.build_transaction(99);
        assert!(transaction.is_balanced());
        assert_eq!(transaction.lines[0].account_id, 99);
        assert_eq!(transaction.lines[0].credit, dec!(100));
        assert_eq!(transaction.lines[1].debit, dec!(100));
    }

    #[test]
    fn validate_rejects_a_credit_note_with_no_lines() {
        let credit_note = CreditNote::new(1, 1, date(2026, 2, 1));
        assert!(credit_note.validate().is_err());
    }
}
