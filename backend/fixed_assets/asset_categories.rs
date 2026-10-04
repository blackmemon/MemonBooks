//! A fixed asset category: defaults for new assets, and the three GL
//! accounts depreciation and disposal need (the asset itself,
//! accumulated depreciation, and depreciation expense).

use crate::backend::core::errors::{AppError, AppResult};
use crate::backend::core::validation::{require_non_empty, Validate};
use crate::backend::fixed_assets::depreciation::DepreciationMethod;

#[derive(Debug, Clone, PartialEq)]
pub struct AssetCategory {
    pub id: Option<i64>,
    pub company_id: i64,
    pub name: String,
    pub asset_gl_account_id: i64,
    pub accumulated_depreciation_gl_account_id: i64,
    pub depreciation_expense_gl_account_id: i64,
    pub default_useful_life_years: u32,
    pub default_method: DepreciationMethod,
}

impl AssetCategory {
    pub fn new(
        company_id: i64,
        name: impl Into<String>,
        asset_gl_account_id: i64,
        accumulated_depreciation_gl_account_id: i64,
        depreciation_expense_gl_account_id: i64,
        default_useful_life_years: u32,
    ) -> Self {
        Self {
            id: None,
            company_id,
            name: name.into(),
            asset_gl_account_id,
            accumulated_depreciation_gl_account_id,
            depreciation_expense_gl_account_id,
            default_useful_life_years,
            default_method: DepreciationMethod::StraightLine,
        }
    }
}

impl Validate for AssetCategory {
    fn validate(&self) -> AppResult<()> {
        if self.company_id <= 0 {
            return Err(AppError::Validation(
                "asset category must reference a valid company".into(),
            ));
        }
        require_non_empty("asset category name", &self.name)?;
        if self.asset_gl_account_id <= 0
            || self.accumulated_depreciation_gl_account_id <= 0
            || self.depreciation_expense_gl_account_id <= 0
        {
            return Err(AppError::Validation(
                "asset category must reference valid GL accounts".into(),
            ));
        }
        if self.default_useful_life_years == 0 {
            return Err(AppError::Validation(
                "default useful life must be at least 1 year".into(),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_rejects_zero_useful_life() {
        let category = AssetCategory::new(1, "Computer Equipment", 10, 11, 12, 0);
        assert!(category.validate().is_err());
    }

    #[test]
    fn validate_accepts_a_well_formed_category() {
        let category = AssetCategory::new(1, "Computer Equipment", 10, 11, 12, 3);
        assert!(category.validate().is_ok());
    }
}
