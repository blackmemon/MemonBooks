//! A manual stock quantity correction (e.g. a stocktake found fewer
//! units than the system expected). Builds a single
//! `MovementType::Adjustment` stock movement -- no storage of its own.

use chrono::NaiveDate;
use rust_decimal::Decimal;

use crate::backend::core::errors::{AppError, AppResult};
use crate::backend::core::validation::require_non_empty;
use crate::backend::inventory::stock_movements::{MovementType, StockMovement};

pub struct StockAdjustment {
    pub company_id: i64,
    pub product_id: i64,
    pub warehouse_id: i64,
    pub adjustment_date: NaiveDate,
    /// Positive to increase on-hand quantity, negative to decrease it.
    pub quantity_delta: Decimal,
    pub reason: String,
}

impl StockAdjustment {
    pub fn new(company_id: i64, product_id: i64, warehouse_id: i64, adjustment_date: NaiveDate, quantity_delta: Decimal, reason: impl Into<String>) -> Self {
        Self {
            company_id,
            product_id,
            warehouse_id,
            adjustment_date,
            quantity_delta,
            reason: reason.into(),
        }
    }

    pub fn validate(&self) -> AppResult<()> {
        require_non_empty("adjustment reason", &self.reason)?;
        if self.quantity_delta == Decimal::ZERO {
            return Err(AppError::Validation("adjustment quantity_delta must not be zero".into()));
        }
        Ok(())
    }

    pub fn build_movement(&self) -> StockMovement {
        let mut movement = StockMovement::new(
            self.company_id,
            self.product_id,
            self.warehouse_id,
            self.adjustment_date,
            MovementType::Adjustment,
            self.quantity_delta,
        );
        movement.reference = Some(self.reason.clone());
        movement
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn validate_rejects_a_blank_reason() {
        let adjustment = StockAdjustment::new(1, 1, 1, NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(), dec!(-5), "");
        assert!(adjustment.validate().is_err());
    }

    #[test]
    fn build_movement_carries_the_reason_as_the_reference() {
        let adjustment = StockAdjustment::new(1, 1, 1, NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(), dec!(-5), "Stocktake shrinkage");
        let movement = adjustment.build_movement();
        assert_eq!(movement.reference.as_deref(), Some("Stocktake shrinkage"));
        assert_eq!(movement.quantity_delta, dec!(-5));
    }
}
