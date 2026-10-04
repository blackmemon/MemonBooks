//! Currency reference data (ISO 4217-style codes).

use crate::backend::core::errors::{AppError, AppResult};
use crate::backend::core::validation::{require_max_length, require_non_empty, Validate};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Currency {
    /// 3-letter ISO 4217 code, e.g. "USD". Acts as the natural primary key.
    pub code: String,
    pub name: String,
    pub symbol: String,
    /// Number of decimal places normally used to display this currency
    /// (2 for USD, 0 for JPY, ...). Used to round converted amounts.
    pub decimal_places: u32,
}

impl Currency {
    pub fn new(
        code: impl Into<String>,
        name: impl Into<String>,
        symbol: impl Into<String>,
        decimal_places: u32,
    ) -> Self {
        Self {
            code: code.into().to_ascii_uppercase(),
            name: name.into(),
            symbol: symbol.into(),
            decimal_places,
        }
    }
}

impl Validate for Currency {
    fn validate(&self) -> AppResult<()> {
        validate_currency_code(&self.code)?;
        require_non_empty("currency name", &self.name)?;
        require_max_length("currency name", &self.name, 100)?;
        require_non_empty("currency symbol", &self.symbol)?;
        require_max_length("currency symbol", &self.symbol, 10)?;
        if self.decimal_places > 6 {
            return Err(AppError::Validation(
                "currency decimal places must be between 0 and 6".into(),
            ));
        }
        Ok(())
    }
}

/// Shared by any module that stores a currency code as a foreign key
/// (companies' base currency, exchange rate pairs, ...) so the "well
/// formed ISO 4217 code" rule lives in exactly one place.
pub fn validate_currency_code(code: &str) -> AppResult<()> {
    let is_well_formed = code.len() == 3 && code.chars().all(|c| c.is_ascii_uppercase());
    if !is_well_formed {
        return Err(AppError::Validation(format!(
            "'{code}' is not a valid 3-letter ISO 4217 currency code"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_well_formed_codes() {
        assert!(validate_currency_code("USD").is_ok());
        assert!(validate_currency_code("PKR").is_ok());
    }

    #[test]
    fn rejects_malformed_codes() {
        assert!(validate_currency_code("usd").is_err());
        assert!(validate_currency_code("US").is_err());
        assert!(validate_currency_code("DOLLAR").is_err());
        assert!(validate_currency_code("").is_err());
    }

    #[test]
    fn new_normalizes_code_to_uppercase() {
        let currency = Currency::new("usd", "US Dollar", "$", 2);
        assert_eq!(currency.code, "USD");
    }

    #[test]
    fn validate_rejects_excessive_decimal_places() {
        let currency = Currency::new("USD", "US Dollar", "$", 7);
        assert!(currency.validate().is_err());
    }

    #[test]
    fn validate_accepts_reasonable_currency() {
        let currency = Currency::new("JPY", "Japanese Yen", "\u{a5}", 0);
        assert!(currency.validate().is_ok());
    }
}
