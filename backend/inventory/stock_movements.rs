//! The append-only ledger of quantity changes. Per the same principle as
//! the general ledger (Architecture Rules, Section 3: derive rather than
//! store duplicated totals), on-hand quantity (`stock.rs`) is computed
//! from these, not maintained as a separately-updated counter.

use chrono::NaiveDate;
use rust_decimal::Decimal;

use crate::backend::core::errors::{AppError, AppResult};
use crate::backend::core::validation::Validate;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MovementType {
    OpeningStock,
    Purchase,
    Sale,
    Adjustment,
    TransferIn,
    TransferOut,
}

impl MovementType {
    pub fn as_str(self) -> &'static str {
        match self {
            MovementType::OpeningStock => "opening_stock",
            MovementType::Purchase => "purchase",
            MovementType::Sale => "sale",
            MovementType::Adjustment => "adjustment",
            MovementType::TransferIn => "transfer_in",
            MovementType::TransferOut => "transfer_out",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "opening_stock" => Some(MovementType::OpeningStock),
            "purchase" => Some(MovementType::Purchase),
            "sale" => Some(MovementType::Sale),
            "adjustment" => Some(MovementType::Adjustment),
            "transfer_in" => Some(MovementType::TransferIn),
            "transfer_out" => Some(MovementType::TransferOut),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct StockMovement {
    pub id: Option<i64>,
    pub company_id: i64,
    pub product_id: i64,
    pub warehouse_id: i64,
    pub movement_date: NaiveDate,
    pub movement_type: MovementType,
    /// Positive for stock coming in, negative for stock going out.
    pub quantity_delta: Decimal,
    pub reference: Option<String>,
    /// The posted GL transaction this movement's cost effect belongs to,
    /// if any -- inventory valuation/COGS posting is out of this step's
    /// scope, so this is usually `None` for now.
    pub transaction_id: Option<i64>,
}

impl StockMovement {
    pub fn new(company_id: i64, product_id: i64, warehouse_id: i64, movement_date: NaiveDate, movement_type: MovementType, quantity_delta: Decimal) -> Self {
        Self {
            id: None,
            company_id,
            product_id,
            warehouse_id,
            movement_date,
            movement_type,
            quantity_delta,
            reference: None,
            transaction_id: None,
        }
    }
}

impl Validate for StockMovement {
    fn validate(&self) -> AppResult<()> {
        if self.company_id <= 0 {
            return Err(AppError::Validation("stock movement must reference a valid company".into()));
        }
        if self.product_id <= 0 {
            return Err(AppError::Validation("stock movement must reference a valid product".into()));
        }
        if self.warehouse_id <= 0 {
            return Err(AppError::Validation("stock movement must reference a valid warehouse".into()));
        }
        if self.quantity_delta == Decimal::ZERO {
            return Err(AppError::Validation("stock movement quantity_delta must not be zero".into()));
        }

        let sign_matches_type = match self.movement_type {
            MovementType::Sale | MovementType::TransferOut => self.quantity_delta < Decimal::ZERO,
            MovementType::Purchase | MovementType::TransferIn | MovementType::OpeningStock => self.quantity_delta > Decimal::ZERO,
            MovementType::Adjustment => true,
        };
        if !sign_matches_type {
            return Err(AppError::Validation(format!(
                "quantity_delta sign does not match movement type {}",
                self.movement_type.as_str()
            )));
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

    #[test]
    fn validate_rejects_zero_quantity_delta() {
        let movement = StockMovement::new(1, 1, 1, date(2026, 1, 1), MovementType::Adjustment, Decimal::ZERO);
        assert!(movement.validate().is_err());
    }

    #[test]
    fn validate_rejects_a_sale_with_a_positive_delta() {
        let movement = StockMovement::new(1, 1, 1, date(2026, 1, 1), MovementType::Sale, dec!(5));
        assert!(movement.validate().is_err());
    }

    #[test]
    fn validate_accepts_a_sale_with_a_negative_delta() {
        let movement = StockMovement::new(1, 1, 1, date(2026, 1, 1), MovementType::Sale, dec!(-5));
        assert!(movement.validate().is_ok());
    }

    #[test]
    fn validate_accepts_either_sign_for_an_adjustment() {
        assert!(StockMovement::new(1, 1, 1, date(2026, 1, 1), MovementType::Adjustment, dec!(5)).validate().is_ok());
        assert!(StockMovement::new(1, 1, 1, date(2026, 1, 1), MovementType::Adjustment, dec!(-5)).validate().is_ok());
    }

    #[test]
    fn as_str_and_parse_round_trip_for_every_variant() {
        for movement_type in [
            MovementType::OpeningStock,
            MovementType::Purchase,
            MovementType::Sale,
            MovementType::Adjustment,
            MovementType::TransferIn,
            MovementType::TransferOut,
        ] {
            assert_eq!(MovementType::parse(movement_type.as_str()), Some(movement_type));
        }
    }
}
