//! A customer: who invoices (Development Order step 10) and receivables
//! (this step) are owed by.

use rust_decimal::Decimal;

use crate::backend::core::errors::{AppError, AppResult};
use crate::backend::core::reference_code;
use crate::backend::core::validation::{require_max_length, require_non_empty, require_non_negative, Validate};

#[derive(Debug, Clone, PartialEq)]
pub struct Customer {
    pub id: Option<i64>,
    pub company_id: i64,
    pub name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub tax_id: Option<String>,
    /// 0 means no credit limit is enforced.
    pub credit_limit: Decimal,
    pub payment_terms_days: i32,
    pub is_active: bool,
    /// Auto-generated, collision-safe internal reference id (`CUST-
    /// YYYYMMDDHHMMSSmmm`) -- see `core::reference_code`. `None` only
    /// for customers created before this field existed; every customer
    /// created through `Customer::new` gets one automatically.
    pub reference_code: Option<String>,
}

impl Customer {
    pub fn new(company_id: i64, name: impl Into<String>) -> Self {
        Self {
            id: None,
            company_id,
            name: name.into(),
            email: None,
            phone: None,
            tax_id: None,
            credit_limit: Decimal::ZERO,
            payment_terms_days: 30,
            is_active: true,
            reference_code: Some(reference_code::generate_code("CUST")),
        }
    }
}

impl Validate for Customer {
    fn validate(&self) -> AppResult<()> {
        if self.company_id <= 0 {
            return Err(AppError::Validation("customer must reference a valid company".into()));
        }
        require_non_empty("customer name", &self.name)?;
        require_max_length("customer name", &self.name, 200)?;
        require_non_negative("credit limit", self.credit_limit)?;
        if self.payment_terms_days < 0 {
            return Err(AppError::Validation("payment terms (days) must not be negative".into()));
        }
        if let Some(email) = &self.email {
            if !email.contains('@') {
                return Err(AppError::Validation("customer email is not a valid email address".into()));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn validate_rejects_negative_credit_limit() {
        let mut customer = Customer::new(1, "Acme Retail");
        customer.credit_limit = dec!(-1);
        assert!(customer.validate().is_err());
    }

    #[test]
    fn validate_accepts_a_well_formed_customer() {
        let customer = Customer::new(1, "Acme Retail");
        assert!(customer.validate().is_ok());
    }
}
