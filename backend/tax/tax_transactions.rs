//! A tax amount charged or paid on a specific taxable event (an invoice,
//! a bill, ...). Kept as its own subledger-style record -- like
//! receivables/payables -- with a supplementary GL posting, since the
//! sales/purchase documents built in step 10 don't carry tax lines
//! themselves.

use chrono::NaiveDate;
use rust_decimal::Decimal;

use crate::backend::core::errors::{AppError, AppResult};
use crate::backend::core::validation::{require_non_negative, require_positive, Validate};
use crate::backend::tax::tax_rates::TaxDirection;
use crate::backend::transactions::transaction::Transaction;
use crate::backend::transactions::transaction_lines::TransactionLine;
use crate::backend::transactions::transaction_types::TransactionType;

#[derive(Debug, Clone, PartialEq)]
pub struct TaxTransaction {
    pub id: Option<i64>,
    pub company_id: i64,
    pub tax_rate_id: i64,
    /// e.g. "INV-123" or "BILL-45".
    pub source_reference: Option<String>,
    pub tax_date: NaiveDate,
    pub taxable_amount: Decimal,
    pub tax_amount: Decimal,
    pub direction: TaxDirection,
    pub transaction_id: Option<i64>,
}

impl TaxTransaction {
    pub fn new(
        company_id: i64,
        tax_rate_id: i64,
        tax_date: NaiveDate,
        taxable_amount: Decimal,
        tax_amount: Decimal,
        direction: TaxDirection,
    ) -> Self {
        Self {
            id: None,
            company_id,
            tax_rate_id,
            source_reference: None,
            tax_date,
            taxable_amount,
            tax_amount,
            direction,
            transaction_id: None,
        }
    }

    /// Output tax: debits `offset_account_id` (typically AR -- the tax
    /// portion increases what the customer owes) and credits
    /// `tax_gl_account_id` (a tax payable liability). Input tax: the
    /// mirror image -- debits the tax account (a reclaimable asset),
    /// credits the offset account (typically AP).
    pub fn build_transaction(&self, tax_gl_account_id: i64, offset_account_id: i64) -> Transaction {
        let mut transaction = Transaction::new(
            self.company_id,
            TransactionType::ManualJournal,
            self.tax_date,
        );
        transaction.description = Some(match &self.source_reference {
            Some(reference) => format!("Tax on {reference}"),
            None => "Tax".into(),
        });

        match self.direction {
            TaxDirection::Output => {
                transaction
                    .lines
                    .push(TransactionLine::debit(offset_account_id, self.tax_amount));
                transaction
                    .lines
                    .push(TransactionLine::credit(tax_gl_account_id, self.tax_amount));
            }
            TaxDirection::Input => {
                transaction
                    .lines
                    .push(TransactionLine::debit(tax_gl_account_id, self.tax_amount));
                transaction
                    .lines
                    .push(TransactionLine::credit(offset_account_id, self.tax_amount));
            }
        }
        transaction
    }
}

impl Validate for TaxTransaction {
    fn validate(&self) -> AppResult<()> {
        if self.company_id <= 0 {
            return Err(AppError::Validation(
                "tax transaction must reference a valid company".into(),
            ));
        }
        if self.tax_rate_id <= 0 {
            return Err(AppError::Validation(
                "tax transaction must reference a valid tax rate".into(),
            ));
        }
        require_non_negative("taxable amount", self.taxable_amount)?;
        require_positive("tax amount", self.tax_amount)?;
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
    fn output_tax_debits_offset_and_credits_the_tax_account() {
        let tax = TaxTransaction::new(
            1,
            1,
            date(2026, 1, 1),
            dec!(100),
            dec!(15),
            TaxDirection::Output,
        );
        let transaction = tax.build_transaction(90, 80);
        assert!(transaction.is_balanced());
        assert_eq!(transaction.lines[0].account_id, 80);
        assert_eq!(transaction.lines[0].debit, dec!(15));
        assert_eq!(transaction.lines[1].account_id, 90);
        assert_eq!(transaction.lines[1].credit, dec!(15));
    }

    #[test]
    fn input_tax_debits_the_tax_account_and_credits_offset() {
        let tax = TaxTransaction::new(
            1,
            1,
            date(2026, 1, 1),
            dec!(100),
            dec!(15),
            TaxDirection::Input,
        );
        let transaction = tax.build_transaction(90, 80);
        assert!(transaction.is_balanced());
        assert_eq!(transaction.lines[0].account_id, 90);
        assert_eq!(transaction.lines[0].debit, dec!(15));
        assert_eq!(transaction.lines[1].account_id, 80);
        assert_eq!(transaction.lines[1].credit, dec!(15));
    }

    #[test]
    fn validate_rejects_non_positive_tax_amount() {
        let tax = TaxTransaction::new(
            1,
            1,
            date(2026, 1, 1),
            dec!(100),
            Decimal::ZERO,
            TaxDirection::Output,
        );
        assert!(tax.validate().is_err());
    }
}
