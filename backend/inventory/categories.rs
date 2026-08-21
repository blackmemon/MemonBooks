//! A hierarchical product category, e.g. "Beverages" > "Soft Drinks".

use crate::backend::core::errors::{AppError, AppResult};
use crate::backend::core::validation::{require_max_length, require_non_empty, Validate};

#[derive(Debug, Clone, PartialEq)]
pub struct Category {
    pub id: Option<i64>,
    pub company_id: i64,
    pub name: String,
    pub parent_id: Option<i64>,
}

impl Category {
    pub fn new(company_id: i64, name: impl Into<String>) -> Self {
        Self {
            id: None,
            company_id,
            name: name.into(),
            parent_id: None,
        }
    }
}

impl Validate for Category {
    fn validate(&self) -> AppResult<()> {
        if self.company_id <= 0 {
            return Err(AppError::Validation("category must reference a valid company".into()));
        }
        require_non_empty("category name", &self.name)?;
        require_max_length("category name", &self.name, 200)?;
        if matches!(self.parent_id, Some(id) if id <= 0) {
            return Err(AppError::Validation("category's parent_id, when set, must be a valid category".into()));
        }
        if self.parent_id == self.id && self.id.is_some() {
            return Err(AppError::Validation("a category cannot be its own parent".into()));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_rejects_a_category_as_its_own_parent() {
        let mut category = Category::new(1, "Beverages");
        category.id = Some(5);
        category.parent_id = Some(5);
        assert!(category.validate().is_err());
    }

    #[test]
    fn validate_accepts_a_well_formed_category() {
        let category = Category::new(1, "Beverages");
        assert!(category.validate().is_ok());
    }
}
