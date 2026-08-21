//! A named tax rate, e.g. "Standard VAT 15%". `direction` distinguishes
//! output tax (charged on sales, owed to the tax authority) from input
//! tax (paid on purchases, generally reclaimable).

use rust_decimal::Decimal;

use crate::backend::core::errors::{AppError, AppResult};
use crate::backend::core::validation::{require_non_empty, require_non_negative, Validate};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TaxDirection {
    Output,
    Input,
}

impl TaxDirection {
    pub fn as_str(self) -> &'static str {
        match self {
            TaxDirection::Output => "output",
            TaxDirection::Input => "input",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "output" => Some(TaxDirection::Output),
            "input" => Some(TaxDirection::Input),
            _ => None,
        }
    }
}

impl std::fmt::Display for TaxDirection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TaxRate {
    pub id: Option<i64>,
    pub company_id: i64,
    pub name: String,
    pub rate_percent: Decimal,
    pub direction: TaxDirection,
    pub gl_account_id: i64,
    pub is_active: bool,
}

impl TaxRate {
    pub fn new(company_id: i64, name: impl Into<String>, rate_percent: Decimal, direction: TaxDirection, gl_account_id: i64) -> Self {
        Self {
            id: None,
            company_id,
            name: name.into(),
            rate_percent,
            direction,
            gl_account_id,
            is_active: true,
        }
    }
}

impl Validate for TaxRate {
    fn validate(&self) -> AppResult<()> {
        if self.company_id <= 0 {
            return Err(AppError::Validation("tax rate must reference a valid company".into()));
        }
        require_non_empty("tax rate name", &self.name)?;
        require_non_negative("tax rate percent", self.rate_percent)?;
        if self.gl_account_id <= 0 {
            return Err(AppError::Validation("tax rate must reference a valid GL account".into()));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn validate_rejects_negative_rate() {
        let rate = TaxRate::new(1, "Standard VAT", dec!(-5), TaxDirection::Output, 10);
        assert!(rate.validate().is_err());
    }

    #[test]
    fn validate_accepts_zero_rated() {
        let rate = TaxRate::new(1, "Zero-rated", dec!(0), TaxDirection::Output, 10);
        assert!(rate.validate().is_ok());
    }

    #[test]
    fn as_str_and_parse_round_trip_for_every_direction() {
        for direction in [TaxDirection::Output, TaxDirection::Input] {
            assert_eq!(TaxDirection::parse(direction.as_str()), Some(direction));
        }
    }
}
