//! An open item owed to the company by a customer. Standalone subledger
//! bookkeeping for now (Development Order step 9); wiring a receivable up
//! to the transaction engine when it's created by a real sales invoice
//! is step 10's job, once there's a specific revenue/tax account to post
//! against. `transaction_id` is here already so that wiring has
//! somewhere to go.

use chrono::NaiveDate;
use rust_decimal::Decimal;

use crate::backend::core::errors::{AppError, AppResult};
use crate::backend::core::validation::{require_positive, Validate};

#[derive(Debug, Clone, PartialEq)]
pub struct Receivable {
    pub id: Option<i64>,
    pub company_id: i64,
    pub customer_id: i64,
    /// e.g. an invoice number, once step 10 creates these from invoices.
    pub reference: Option<String>,
    pub invoice_date: NaiveDate,
    pub due_date: NaiveDate,
    pub amount: Decimal,
    pub amount_paid: Decimal,
    pub transaction_id: Option<i64>,
}

impl Receivable {
    pub fn new(
        company_id: i64,
        customer_id: i64,
        invoice_date: NaiveDate,
        due_date: NaiveDate,
        amount: Decimal,
    ) -> Self {
        Self {
            id: None,
            company_id,
            customer_id,
            reference: None,
            invoice_date,
            due_date,
            amount,
            amount_paid: Decimal::ZERO,
            transaction_id: None,
        }
    }

    pub fn balance(&self) -> Decimal {
        self.amount - self.amount_paid
    }

    pub fn is_settled(&self) -> bool {
        self.balance() <= Decimal::ZERO
    }

    pub fn is_overdue(&self, as_of: NaiveDate) -> bool {
        !self.is_settled() && as_of > self.due_date
    }
}

impl Validate for Receivable {
    fn validate(&self) -> AppResult<()> {
        if self.company_id <= 0 {
            return Err(AppError::Validation(
                "receivable must reference a valid company".into(),
            ));
        }
        if self.customer_id <= 0 {
            return Err(AppError::Validation(
                "receivable must reference a valid customer".into(),
            ));
        }
        require_positive("receivable amount", self.amount)?;
        if self.amount_paid < Decimal::ZERO {
            return Err(AppError::Validation(
                "receivable amount_paid must not be negative".into(),
            ));
        }
        if self.amount_paid > self.amount {
            return Err(AppError::Validation(
                "receivable amount_paid must not exceed its amount".into(),
            ));
        }
        if self.due_date < self.invoice_date {
            return Err(AppError::Validation(
                "receivable due_date must not be before its invoice_date".into(),
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

    fn receivable() -> Receivable {
        Receivable::new(1, 1, date(2026, 1, 1), date(2026, 1, 31), dec!(1000))
    }

    #[test]
    fn balance_reflects_amount_paid() {
        let mut r = receivable();
        r.amount_paid = dec!(400);
        assert_eq!(r.balance(), dec!(600));
        assert!(!r.is_settled());
    }

    #[test]
    fn fully_paid_is_settled() {
        let mut r = receivable();
        r.amount_paid = dec!(1000);
        assert!(r.is_settled());
    }

    #[test]
    fn overdue_only_when_unsettled_and_past_due_date() {
        let r = receivable();
        assert!(!r.is_overdue(date(2026, 1, 15)));
        assert!(r.is_overdue(date(2026, 2, 1)));

        let mut settled = receivable();
        settled.amount_paid = dec!(1000);
        assert!(!settled.is_overdue(date(2026, 2, 1)));
    }

    #[test]
    fn validate_rejects_amount_paid_exceeding_amount() {
        let mut r = receivable();
        r.amount_paid = dec!(1500);
        assert!(r.validate().is_err());
    }

    #[test]
    fn validate_rejects_due_date_before_invoice_date() {
        let r = Receivable::new(1, 1, date(2026, 1, 31), date(2026, 1, 1), dec!(1000));
        assert!(r.validate().is_err());
    }

    #[test]
    fn validate_accepts_a_well_formed_receivable() {
        assert!(receivable().validate().is_ok());
    }
}
