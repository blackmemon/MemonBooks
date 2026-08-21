//! A company is the top of the organizational hierarchy: it owns branches,
//! departments, and (starting with the chart of accounts) its own set of
//! books, all denominated in its base currency by default.

use crate::backend::core::errors::AppResult;
use crate::backend::core::validation::{require_max_length, require_non_empty, Validate};
use crate::backend::currency::currencies::validate_currency_code;

#[derive(Debug, Clone, PartialEq)]
pub struct Company {
    pub id: Option<i64>,
    pub name: String,
    pub legal_name: Option<String>,
    /// ISO 4217 code this company's ledgers are kept in by default.
    pub base_currency_code: String,
}

impl Company {
    pub fn new(name: impl Into<String>, base_currency_code: impl Into<String>) -> Self {
        Self {
            id: None,
            name: name.into(),
            legal_name: None,
            base_currency_code: base_currency_code.into().to_ascii_uppercase(),
        }
    }
}

impl Validate for Company {
    fn validate(&self) -> AppResult<()> {
        require_non_empty("company name", &self.name)?;
        require_max_length("company name", &self.name, 200)?;
        if let Some(legal_name) = &self.legal_name {
            require_max_length("company legal name", legal_name, 200)?;
        }
        validate_currency_code(&self.base_currency_code)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_rejects_blank_name() {
        let company = Company::new("", "USD");
        assert!(company.validate().is_err());
    }

    #[test]
    fn validate_rejects_invalid_base_currency() {
        let company = Company::new("Acme Trading Co", "dollars");
        assert!(company.validate().is_err());
    }

    #[test]
    fn validate_accepts_well_formed_company() {
        let company = Company::new("Acme Trading Co", "usd");
        assert!(company.validate().is_ok());
        assert_eq!(company.base_currency_code, "USD");
    }
}
