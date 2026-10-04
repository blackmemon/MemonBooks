//! A debit note: reduces what the company owes a vendor (a return or
//! adjustment against a previously received bill). Mirrors
//! `sales::credit_notes::CreditNote`.

use chrono::NaiveDate;
use rust_decimal::Decimal;

use crate::backend::core::errors::{AppError, AppResult};
use crate::backend::core::validation::{require_non_empty, require_positive, Validate};
use crate::backend::transactions::transaction::Transaction;
use crate::backend::transactions::transaction_lines::TransactionLine;
use crate::backend::transactions::transaction_types::TransactionType;

#[derive(Debug, Clone, PartialEq)]
pub struct DebitNoteLine {
    pub account_id: i64,
    pub description: String,
    pub amount: Decimal,
}

impl DebitNoteLine {
    pub fn new(account_id: i64, description: impl Into<String>, amount: Decimal) -> Self {
        Self {
            account_id,
            description: description.into(),
            amount,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DebitNote {
    pub id: Option<i64>,
    pub company_id: i64,
    pub vendor_id: i64,
    pub payable_id: Option<i64>,
    pub debit_note_date: NaiveDate,
    pub lines: Vec<DebitNoteLine>,
    pub transaction_id: Option<i64>,
}

impl DebitNote {
    pub fn new(company_id: i64, vendor_id: i64, debit_note_date: NaiveDate) -> Self {
        Self {
            id: None,
            company_id,
            vendor_id,
            payable_id: None,
            debit_note_date,
            lines: Vec::new(),
            transaction_id: None,
        }
    }

    pub fn total(&self) -> Decimal {
        self.lines.iter().map(|line| line.amount).sum()
    }

    pub fn build_transaction(&self, ap_account_id: i64) -> Transaction {
        let mut transaction = Transaction::new(
            self.company_id,
            TransactionType::PurchaseBill,
            self.debit_note_date,
        );
        transaction.description = Some(format!("Debit note for vendor #{}", self.vendor_id));
        transaction
            .lines
            .push(TransactionLine::debit(ap_account_id, self.total()));
        for line in &self.lines {
            transaction.lines.push(TransactionLine {
                description: Some(line.description.clone()),
                ..TransactionLine::credit(line.account_id, line.amount)
            });
        }
        transaction
    }
}

impl Validate for DebitNote {
    fn validate(&self) -> AppResult<()> {
        if self.company_id <= 0 {
            return Err(AppError::Validation(
                "debit note must reference a valid company".into(),
            ));
        }
        if self.vendor_id <= 0 {
            return Err(AppError::Validation(
                "debit note must reference a valid vendor".into(),
            ));
        }
        if self.lines.is_empty() {
            return Err(AppError::Validation(
                "a debit note must have at least one line".into(),
            ));
        }
        for line in &self.lines {
            require_non_empty("debit note line description", &line.description)?;
            require_positive("debit note line amount", line.amount)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn build_transaction_debits_ap_and_credits_the_expense_account() {
        let mut debit_note = DebitNote::new(1, 1, NaiveDate::from_ymd_opt(2026, 2, 1).unwrap());
        debit_note
            .lines
            .push(DebitNoteLine::new(50, "Damaged goods return", dec!(150)));

        let transaction = debit_note.build_transaction(99);
        assert!(transaction.is_balanced());
        assert_eq!(transaction.lines[0].account_id, 99);
        assert_eq!(transaction.lines[0].debit, dec!(150));
        assert_eq!(transaction.lines[1].credit, dec!(150));
    }

    #[test]
    fn validate_rejects_a_debit_note_with_no_lines() {
        let debit_note = DebitNote::new(1, 1, NaiveDate::from_ymd_opt(2026, 2, 1).unwrap());
        assert!(debit_note.validate().is_err());
    }
}
