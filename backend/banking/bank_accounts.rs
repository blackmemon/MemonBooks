//! A real-world bank account, linked to the GL asset account that
//! represents it in the chart of accounts.

use crate::backend::core::errors::{AppError, AppResult};
use crate::backend::core::validation::{require_max_length, require_non_empty, Validate};

#[derive(Debug, Clone, PartialEq)]
pub struct BankAccount {
    pub id: Option<i64>,
    pub company_id: i64,
    /// The chart-of-accounts Asset account this bank account posts to.
    pub gl_account_id: i64,
    pub bank_name: String,
    pub account_number: String,
    pub currency_code: String,
    pub is_active: bool,
}

impl BankAccount {
    pub fn new(company_id: i64, gl_account_id: i64, bank_name: impl Into<String>, account_number: impl Into<String>, currency_code: impl Into<String>) -> Self {
        Self {
            id: None,
            company_id,
            gl_account_id,
            bank_name: bank_name.into(),
            account_number: account_number.into(),
            currency_code: currency_code.into().to_ascii_uppercase(),
            is_active: true,
        }
    }
}

impl Validate for BankAccount {
    fn validate(&self) -> AppResult<()> {
        if self.company_id <= 0 {
            return Err(AppError::Validation("bank account must reference a valid company".into()));
        }
        if self.gl_account_id <= 0 {
            return Err(AppError::Validation("bank account must reference a valid GL account".into()));
        }
        require_non_empty("bank name", &self.bank_name)?;
        require_max_length("bank name", &self.bank_name, 200)?;
        require_non_empty("account number", &self.account_number)?;
        require_max_length("account number", &self.account_number, 50)?;
        crate::backend::currency::currencies::validate_currency_code(&self.currency_code)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_rejects_invalid_currency_code() {
        let account = BankAccount::new(1, 10, "First National", "123456789", "dollars");
        assert!(account.validate().is_err());
    }

    #[test]
    fn validate_accepts_a_well_formed_bank_account() {
        let account = BankAccount::new(1, 10, "First National", "123456789", "usd");
        assert!(account.validate().is_ok());
        assert_eq!(account.currency_code, "USD");
    }
}
