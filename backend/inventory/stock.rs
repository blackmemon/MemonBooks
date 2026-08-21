//! On-hand stock quantity: derived from `stock_movements`, the same way
//! an account's balance derives from its transaction lines
//! (`accounting::general_ledger`) rather than being stored and kept in
//! sync by hand.

use rust_decimal::Decimal;

use crate::backend::inventory::stock_movements::StockMovement;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StockKey {
    pub product_id: i64,
    pub warehouse_id: i64,
}

/// Sums every movement's `quantity_delta` for the same (product,
/// warehouse) pair -- the on-hand quantity as of whatever cutoff the
/// caller already filtered `movements` to.
pub fn on_hand_quantity(movements: &[StockMovement], product_id: i64, warehouse_id: i64) -> Decimal {
    movements
        .iter()
        .filter(|m| m.product_id == product_id && m.warehouse_id == warehouse_id)
        .map(|m| m.quantity_delta)
        .sum()
}

/// On-hand quantity for a product across every warehouse it's held in.
pub fn total_on_hand_quantity(movements: &[StockMovement], product_id: i64) -> Decimal {
    movements.iter().filter(|m| m.product_id == product_id).map(|m| m.quantity_delta).sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::inventory::stock_movements::MovementType;
    use chrono::NaiveDate;
    use rust_decimal_macros::dec;

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    fn movement(product_id: i64, warehouse_id: i64, movement_type: MovementType, delta: Decimal) -> StockMovement {
        StockMovement::new(1, product_id, warehouse_id, date(2026, 1, 1), movement_type, delta)
    }

    #[test]
    fn on_hand_quantity_sums_movements_for_the_same_product_and_warehouse() {
        let movements = vec![
            movement(1, 10, MovementType::Purchase, dec!(100)),
            movement(1, 10, MovementType::Sale, dec!(-30)),
            movement(1, 20, MovementType::Purchase, dec!(50)), // different warehouse
            movement(2, 10, MovementType::Purchase, dec!(999)), // different product
        ];
        assert_eq!(on_hand_quantity(&movements, 1, 10), dec!(70));
    }

    #[test]
    fn total_on_hand_quantity_sums_across_warehouses() {
        let movements = vec![
            movement(1, 10, MovementType::Purchase, dec!(100)),
            movement(1, 20, MovementType::Purchase, dec!(50)),
            movement(1, 10, MovementType::Sale, dec!(-30)),
        ];
        assert_eq!(total_on_hand_quantity(&movements, 1), dec!(120));
    }

    #[test]
    fn no_movements_means_zero_on_hand() {
        assert_eq!(on_hand_quantity(&[], 1, 1), Decimal::ZERO);
    }
}
