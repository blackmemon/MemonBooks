//! A physical or logical stock location.

use crate::backend::core::errors::{AppError, AppResult};
use crate::backend::core::validation::{require_max_length, require_non_empty, Validate};

#[derive(Debug, Clone, PartialEq)]
pub struct Warehouse {
    pub id: Option<i64>,
    pub company_id: i64,
    pub name: String,
    pub code: String,
    pub address: Option<String>,
    pub is_active: bool,
}

impl Warehouse {
    pub fn new(company_id: i64, name: impl Into<String>, code: impl Into<String>) -> Self {
        Self {
            id: None,
            company_id,
            name: name.into(),
            code: code.into(),
            address: None,
            is_active: true,
        }
    }
}

impl Validate for Warehouse {
    fn validate(&self) -> AppResult<()> {
        if self.company_id <= 0 {
            return Err(AppError::Validation(
                "warehouse must reference a valid company".into(),
            ));
        }
        require_non_empty("warehouse name", &self.name)?;
        require_max_length("warehouse name", &self.name, 200)?;
        require_non_empty("warehouse code", &self.code)?;
        require_max_length("warehouse code", &self.code, 20)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_rejects_blank_code() {
        let warehouse = Warehouse::new(1, "Main Warehouse", "");
        assert!(warehouse.validate().is_err());
    }

    #[test]
    fn validate_accepts_a_well_formed_warehouse() {
        let warehouse = Warehouse::new(1, "Main Warehouse", "MAIN");
        assert!(warehouse.validate().is_ok());
    }
}
