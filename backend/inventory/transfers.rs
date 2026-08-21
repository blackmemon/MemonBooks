//! Moving stock between two warehouses: a paired `TransferOut` from the
//! source and `TransferIn` to the destination, posted together so
//! quantity is never counted (or lost) in transit within the ledger.

use chrono::NaiveDate;
use rust_decimal::Decimal;

use crate::backend::core::errors::{AppError, AppResult};
use crate::backend::core::validation::require_positive;
use crate::backend::inventory::stock_movements::{MovementType, StockMovement};

pub struct StockTransfer {
    pub company_id: i64,
    pub product_id: i64,
    pub from_warehouse_id: i64,
    pub to_warehouse_id: i64,
    pub transfer_date: NaiveDate,
    pub quantity: Decimal,
}

impl StockTransfer {
    pub fn new(company_id: i64, product_id: i64, from_warehouse_id: i64, to_warehouse_id: i64, transfer_date: NaiveDate, quantity: Decimal) -> Self {
        Self {
            company_id,
            product_id,
            from_warehouse_id,
            to_warehouse_id,
            transfer_date,
            quantity,
        }
    }

    pub fn validate(&self) -> AppResult<()> {
        if self.from_warehouse_id == self.to_warehouse_id {
            return Err(AppError::Validation("stock transfer source and destination warehouses must differ".into()));
        }
        require_positive("transfer quantity", self.quantity)?;
        Ok(())
    }

    /// The two movements a transfer produces: `[out, in]`. Both must be
    /// recorded together (`InventoryRepository::post_transfer` does this
    /// atomically) so the ledger never shows stock leaving one warehouse
    /// without arriving at the other.
    pub fn build_movements(&self) -> [StockMovement; 2] {
        let reference = Some(format!("Transfer {} -> {}", self.from_warehouse_id, self.to_warehouse_id));
        let mut out = StockMovement::new(
            self.company_id,
            self.product_id,
            self.from_warehouse_id,
            self.transfer_date,
            MovementType::TransferOut,
            -self.quantity,
        );
        out.reference = reference.clone();

        let mut inbound = StockMovement::new(
            self.company_id,
            self.product_id,
            self.to_warehouse_id,
            self.transfer_date,
            MovementType::TransferIn,
            self.quantity,
        );
        inbound.reference = reference;

        [out, inbound]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn transfer() -> StockTransfer {
        StockTransfer::new(1, 1, 10, 20, NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(), dec!(50))
    }

    #[test]
    fn validate_rejects_same_source_and_destination() {
        let mut t = transfer();
        t.to_warehouse_id = t.from_warehouse_id;
        assert!(t.validate().is_err());
    }

    #[test]
    fn build_movements_produces_a_balanced_pair() {
        let [out, inbound] = transfer().build_movements();
        assert_eq!(out.warehouse_id, 10);
        assert_eq!(out.quantity_delta, dec!(-50));
        assert_eq!(inbound.warehouse_id, 20);
        assert_eq!(inbound.quantity_delta, dec!(50));
        assert_eq!(out.quantity_delta + inbound.quantity_delta, Decimal::ZERO);
    }
}
