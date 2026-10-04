//! A POS terminal/register: which warehouse it sells from (for stock
//! deduction) and, optionally, which branch it belongs to.

use crate::backend::core::errors::{AppError, AppResult};
use crate::backend::core::validation::{require_max_length, require_non_empty, Validate};

#[derive(Debug, Clone, PartialEq)]
pub struct Terminal {
    pub id: Option<i64>,
    pub company_id: i64,
    pub branch_id: Option<i64>,
    pub warehouse_id: i64,
    pub name: String,
    pub is_active: bool,
}

impl Terminal {
    pub fn new(company_id: i64, warehouse_id: i64, name: impl Into<String>) -> Self {
        Self {
            id: None,
            company_id,
            branch_id: None,
            warehouse_id,
            name: name.into(),
            is_active: true,
        }
    }
}

impl Validate for Terminal {
    fn validate(&self) -> AppResult<()> {
        if self.company_id <= 0 {
            return Err(AppError::Validation(
                "terminal must reference a valid company".into(),
            ));
        }
        if self.warehouse_id <= 0 {
            return Err(AppError::Validation(
                "terminal must reference a valid warehouse".into(),
            ));
        }
        require_non_empty("terminal name", &self.name)?;
        require_max_length("terminal name", &self.name, 100)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_accepts_a_well_formed_terminal() {
        let terminal = Terminal::new(1, 10, "Front Counter");
        assert!(terminal.validate().is_ok());
    }

    #[test]
    fn validate_rejects_missing_warehouse() {
        let terminal = Terminal::new(1, 0, "Front Counter");
        assert!(terminal.validate().is_err());
    }
}
