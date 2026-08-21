//! A product (or service, if `track_inventory` is false, in which case
//! stock quantities never apply to it).

use rust_decimal::Decimal;

use crate::backend::core::errors::{AppError, AppResult};
use crate::backend::core::validation::{require_max_length, require_non_empty, require_non_negative, Validate};

#[derive(Debug, Clone, PartialEq)]
pub struct Product {
    pub id: Option<i64>,
    pub company_id: i64,
    pub sku: String,
    pub name: String,
    pub category_id: Option<i64>,
    pub unit_of_measure: String,
    pub cost_price: Decimal,
    pub sale_price: Decimal,
    pub track_inventory: bool,
    pub is_active: bool,
}

impl Product {
    pub fn new(company_id: i64, sku: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: None,
            company_id,
            sku: sku.into(),
            name: name.into(),
            category_id: None,
            unit_of_measure: "each".into(),
            cost_price: Decimal::ZERO,
            sale_price: Decimal::ZERO,
            track_inventory: true,
            is_active: true,
        }
    }
}

impl Validate for Product {
    fn validate(&self) -> AppResult<()> {
        if self.company_id <= 0 {
            return Err(AppError::Validation("product must reference a valid company".into()));
        }
        require_non_empty("product SKU", &self.sku)?;
        require_max_length("product SKU", &self.sku, 50)?;
        require_non_empty("product name", &self.name)?;
        require_max_length("product name", &self.name, 200)?;
        require_non_empty("unit of measure", &self.unit_of_measure)?;
        require_non_negative("cost price", self.cost_price)?;
        require_non_negative("sale price", self.sale_price)?;
        if matches!(self.category_id, Some(id) if id <= 0) {
            return Err(AppError::Validation("product's category_id, when set, must be a valid category".into()));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn validate_rejects_negative_prices() {
        let mut product = Product::new(1, "WIDGET-1", "Widget");
        product.cost_price = dec!(-1);
        assert!(product.validate().is_err());
    }

    #[test]
    fn validate_accepts_a_well_formed_product() {
        let product = Product::new(1, "WIDGET-1", "Widget");
        assert!(product.validate().is_ok());
    }
}
