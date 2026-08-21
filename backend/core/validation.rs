//! Generic, reusable validation primitives shared across modules. Concrete
//! business rules (e.g. "an account code must be unique") live with the
//! module that owns them and build on these helpers.

use rust_decimal::Decimal;

use crate::backend::core::errors::{AppError, AppResult};

pub trait Validate {
    fn validate(&self) -> AppResult<()>;
}

pub fn require_non_empty(field: &str, value: &str) -> AppResult<()> {
    if value.trim().is_empty() {
        return Err(AppError::Validation(format!("{field} must not be empty")));
    }
    Ok(())
}

pub fn require_max_length(field: &str, value: &str, max: usize) -> AppResult<()> {
    if value.chars().count() > max {
        return Err(AppError::Validation(format!(
            "{field} must be at most {max} characters"
        )));
    }
    Ok(())
}

pub fn require_non_negative(field: &str, value: Decimal) -> AppResult<()> {
    if value.is_sign_negative() {
        return Err(AppError::Validation(format!("{field} must not be negative")));
    }
    Ok(())
}

pub fn require_positive(field: &str, value: Decimal) -> AppResult<()> {
    if value <= Decimal::ZERO {
        return Err(AppError::Validation(format!("{field} must be greater than zero")));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn non_empty_rejects_blank_and_whitespace() {
        assert!(require_non_empty("name", "").is_err());
        assert!(require_non_empty("name", "   ").is_err());
        assert!(require_non_empty("name", "Cash").is_ok());
    }

    #[test]
    fn max_length_counts_unicode_scalars_not_bytes() {
        assert!(require_max_length("name", "abc", 3).is_ok());
        assert!(require_max_length("name", "abcd", 3).is_err());
    }

    #[test]
    fn non_negative_allows_zero_rejects_negative() {
        assert!(require_non_negative("amount", dec!(0)).is_ok());
        assert!(require_non_negative("amount", dec!(1.50)).is_ok());
        assert!(require_non_negative("amount", dec!(-0.01)).is_err());
    }

    #[test]
    fn positive_rejects_zero_and_negative() {
        assert!(require_positive("amount", dec!(0)).is_err());
        assert!(require_positive("amount", dec!(-1)).is_err());
        assert!(require_positive("amount", dec!(0.01)).is_ok());
    }
}
