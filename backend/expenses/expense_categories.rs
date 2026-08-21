//! An expense category, mapped to the GL expense account it posts to.

use crate::backend::core::errors::{AppError, AppResult};
use crate::backend::core::validation::{require_max_length, require_non_empty, Validate};

#[derive(Debug, Clone, PartialEq)]
pub struct ExpenseCategory {
    pub id: Option<i64>,
    pub company_id: i64,
    pub name: String,
    pub gl_account_id: i64,
}

impl ExpenseCategory {
    pub fn new(company_id: i64, name: impl Into<String>, gl_account_id: i64) -> Self {
        Self {
            id: None,
            company_id,
            name: name.into(),
            gl_account_id,
        }
    }
}

impl Validate for ExpenseCategory {
    fn validate(&self) -> AppResult<()> {
        if self.company_id <= 0 {
            return Err(AppError::Validation("expense category must reference a valid company".into()));
        }
        require_non_empty("expense category name", &self.name)?;
        require_max_length("expense category name", &self.name, 200)?;
        if self.gl_account_id <= 0 {
            return Err(AppError::Validation("expense category must reference a valid GL account".into()));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_accepts_a_well_formed_category() {
        let category = ExpenseCategory::new(1, "Travel", 10);
        assert!(category.validate().is_ok());
    }

    #[test]
    fn validate_rejects_blank_name() {
        let category = ExpenseCategory::new(1, "", 10);
        assert!(category.validate().is_err());
    }
}
