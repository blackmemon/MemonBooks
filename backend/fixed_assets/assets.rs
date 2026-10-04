//! A fixed asset owned by the company.

use chrono::NaiveDate;
use rust_decimal::Decimal;

use crate::backend::core::errors::{AppError, AppResult};
use crate::backend::core::validation::{
    require_non_empty, require_non_negative, require_positive, Validate,
};
use crate::backend::fixed_assets::depreciation::DepreciationMethod;

#[derive(Debug, Clone, PartialEq)]
pub struct Asset {
    pub id: Option<i64>,
    pub company_id: i64,
    pub category_id: i64,
    pub name: String,
    pub acquisition_date: NaiveDate,
    pub acquisition_cost: Decimal,
    pub salvage_value: Decimal,
    pub useful_life_years: u32,
    pub method: DepreciationMethod,
    pub accumulated_depreciation: Decimal,
    pub is_disposed: bool,
    pub transaction_id: Option<i64>,
}

impl Asset {
    pub fn new(
        company_id: i64,
        category_id: i64,
        name: impl Into<String>,
        acquisition_date: NaiveDate,
        acquisition_cost: Decimal,
        salvage_value: Decimal,
        useful_life_years: u32,
    ) -> Self {
        Self {
            id: None,
            company_id,
            category_id,
            name: name.into(),
            acquisition_date,
            acquisition_cost,
            salvage_value,
            useful_life_years,
            method: DepreciationMethod::StraightLine,
            accumulated_depreciation: Decimal::ZERO,
            is_disposed: false,
            transaction_id: None,
        }
    }

    pub fn book_value(&self) -> Decimal {
        self.acquisition_cost - self.accumulated_depreciation
    }
}

impl Validate for Asset {
    fn validate(&self) -> AppResult<()> {
        if self.company_id <= 0 {
            return Err(AppError::Validation(
                "asset must reference a valid company".into(),
            ));
        }
        if self.category_id <= 0 {
            return Err(AppError::Validation(
                "asset must reference a valid category".into(),
            ));
        }
        require_non_empty("asset name", &self.name)?;
        require_positive("acquisition cost", self.acquisition_cost)?;
        require_non_negative("salvage value", self.salvage_value)?;
        if self.salvage_value > self.acquisition_cost {
            return Err(AppError::Validation(
                "salvage value must not exceed acquisition cost".into(),
            ));
        }
        if self.useful_life_years == 0 {
            return Err(AppError::Validation(
                "useful life must be at least 1 year".into(),
            ));
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
    fn book_value_subtracts_accumulated_depreciation() {
        let mut asset = Asset::new(
            1,
            1,
            "Office Laptop",
            date(2026, 1, 1),
            dec!(2000),
            dec!(200),
            3,
        );
        asset.accumulated_depreciation = dec!(600);
        assert_eq!(asset.book_value(), dec!(1400));
    }

    #[test]
    fn validate_rejects_salvage_value_exceeding_cost() {
        let asset = Asset::new(
            1,
            1,
            "Office Laptop",
            date(2026, 1, 1),
            dec!(2000),
            dec!(2500),
            3,
        );
        assert!(asset.validate().is_err());
    }

    #[test]
    fn validate_accepts_a_well_formed_asset() {
        let asset = Asset::new(
            1,
            1,
            "Office Laptop",
            date(2026, 1, 1),
            dec!(2000),
            dec!(200),
            3,
        );
        assert!(asset.validate().is_ok());
    }
}
