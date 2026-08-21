//! A branch is a physical or operational location belonging to a company.

use crate::backend::core::errors::{AppError, AppResult};
use crate::backend::core::validation::{require_max_length, require_non_empty, Validate};

#[derive(Debug, Clone, PartialEq)]
pub struct Branch {
    pub id: Option<i64>,
    pub company_id: i64,
    pub name: String,
    pub code: String,
    pub address: Option<String>,
}

impl Branch {
    pub fn new(company_id: i64, name: impl Into<String>, code: impl Into<String>) -> Self {
        Self {
            id: None,
            company_id,
            name: name.into(),
            code: code.into(),
            address: None,
        }
    }
}

impl Validate for Branch {
    fn validate(&self) -> AppResult<()> {
        if self.company_id <= 0 {
            return Err(AppError::Validation("branch must reference a valid company".into()));
        }
        require_non_empty("branch name", &self.name)?;
        require_max_length("branch name", &self.name, 200)?;
        require_non_empty("branch code", &self.code)?;
        require_max_length("branch code", &self.code, 20)?;
        if let Some(address) = &self.address {
            require_max_length("branch address", address, 500)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_rejects_non_positive_company_id() {
        let branch = Branch::new(0, "Main Branch", "MAIN");
        assert!(branch.validate().is_err());
    }

    #[test]
    fn validate_rejects_blank_code() {
        let branch = Branch::new(1, "Main Branch", "");
        assert!(branch.validate().is_err());
    }

    #[test]
    fn validate_accepts_well_formed_branch() {
        let branch = Branch::new(1, "Main Branch", "MAIN");
        assert!(branch.validate().is_ok());
    }
}
