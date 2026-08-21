//! An open item the company owes a vendor. Mirrors
//! `accounts_receivable::receivables::Receivable` -- same standalone
//! subledger approach, same reasoning for why it's not yet wired to the
//! transaction engine (that's step 10, once a real bill exists).

use chrono::NaiveDate;
use rust_decimal::Decimal;

use crate::backend::core::errors::{AppError, AppResult};
use crate::backend::core::validation::{require_positive, Validate};

#[derive(Debug, Clone, PartialEq)]
pub struct Payable {
    pub id: Option<i64>,
    pub company_id: i64,
    pub vendor_id: i64,
    /// e.g. a vendor bill number, once step 10 creates these from bills.
    pub reference: Option<String>,
    pub bill_date: NaiveDate,
    pub due_date: NaiveDate,
    pub amount: Decimal,
    pub amount_paid: Decimal,
    pub transaction_id: Option<i64>,
}

impl Payable {
    pub fn new(company_id: i64, vendor_id: i64, bill_date: NaiveDate, due_date: NaiveDate, amount: Decimal) -> Self {
        Self {
            id: None,
            company_id,
            vendor_id,
            reference: None,
            bill_date,
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

impl Validate for Payable {
    fn validate(&self) -> AppResult<()> {
        if self.company_id <= 0 {
            return Err(AppError::Validation("payable must reference a valid company".into()));
        }
        if self.vendor_id <= 0 {
            return Err(AppError::Validation("payable must reference a valid vendor".into()));
        }
        require_positive("payable amount", self.amount)?;
        if self.amount_paid < Decimal::ZERO {
            return Err(AppError::Validation("payable amount_paid must not be negative".into()));
        }
        if self.amount_paid > self.amount {
            return Err(AppError::Validation("payable amount_paid must not exceed its amount".into()));
        }
        if self.due_date < self.bill_date {
            return Err(AppError::Validation("payable due_date must not be before its bill_date".into()));
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

    fn payable() -> Payable {
        Payable::new(1, 1, date(2026, 1, 1), date(2026, 1, 31), dec!(1000))
    }

    #[test]
    fn balance_reflects_amount_paid() {
        let mut p = payable();
        p.amount_paid = dec!(400);
        assert_eq!(p.balance(), dec!(600));
        assert!(!p.is_settled());
    }

    #[test]
    fn fully_paid_is_settled() {
        let mut p = payable();
        p.amount_paid = dec!(1000);
        assert!(p.is_settled());
    }

    #[test]
    fn overdue_only_when_unsettled_and_past_due_date() {
        let p = payable();
        assert!(!p.is_overdue(date(2026, 1, 15)));
        assert!(p.is_overdue(date(2026, 2, 1)));
    }

    #[test]
    fn validate_rejects_amount_paid_exceeding_amount() {
        let mut p = payable();
        p.amount_paid = dec!(1500);
        assert!(p.validate().is_err());
    }

    #[test]
    fn validate_accepts_a_well_formed_payable() {
        assert!(payable().validate().is_ok());
    }
}
