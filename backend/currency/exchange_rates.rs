//! Historical exchange rates between currency pairs.

use chrono::NaiveDate;
use rust_decimal::Decimal;

use crate::backend::core::errors::{AppError, AppResult};
use crate::backend::core::validation::{require_positive, Validate};
use crate::backend::currency::currencies::validate_currency_code;

#[derive(Debug, Clone, PartialEq)]
pub struct ExchangeRate {
    pub id: Option<i64>,
    pub from_currency: String,
    pub to_currency: String,
    /// Units of `to_currency` per 1 unit of `from_currency`.
    pub rate: Decimal,
    pub as_of: NaiveDate,
}

impl ExchangeRate {
    pub fn new(from_currency: impl Into<String>, to_currency: impl Into<String>, rate: Decimal, as_of: NaiveDate) -> Self {
        Self {
            id: None,
            from_currency: from_currency.into().to_ascii_uppercase(),
            to_currency: to_currency.into().to_ascii_uppercase(),
            rate,
            as_of,
        }
    }
}

impl Validate for ExchangeRate {
    fn validate(&self) -> AppResult<()> {
        validate_currency_code(&self.from_currency)?;
        validate_currency_code(&self.to_currency)?;
        if self.from_currency == self.to_currency {
            return Err(AppError::Validation(
                "exchange rate from_currency and to_currency must differ".into(),
            ));
        }
        require_positive("exchange rate", self.rate)?;
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
    fn validate_rejects_same_currency_pair() {
        let rate = ExchangeRate::new("USD", "USD", dec!(1), date(2026, 1, 1));
        assert!(rate.validate().is_err());
    }

    #[test]
    fn validate_rejects_non_positive_rate() {
        let rate = ExchangeRate::new("USD", "PKR", dec!(0), date(2026, 1, 1));
        assert!(rate.validate().is_err());

        let rate = ExchangeRate::new("USD", "PKR", dec!(-278.5), date(2026, 1, 1));
        assert!(rate.validate().is_err());
    }

    #[test]
    fn validate_accepts_well_formed_rate() {
        let rate = ExchangeRate::new("usd", "pkr", dec!(278.50), date(2026, 1, 1));
        assert!(rate.validate().is_ok());
        assert_eq!(rate.from_currency, "USD");
        assert_eq!(rate.to_currency, "PKR");
    }
}
