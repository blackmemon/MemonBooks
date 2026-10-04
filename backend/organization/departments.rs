//! A department belongs to a company and, optionally, one of its branches
//! (some departments -- e.g. Finance -- operate across every branch).

use crate::backend::core::errors::{AppError, AppResult};
use crate::backend::core::validation::{require_max_length, require_non_empty, Validate};

#[derive(Debug, Clone, PartialEq)]
pub struct Department {
    pub id: Option<i64>,
    pub company_id: i64,
    pub branch_id: Option<i64>,
    pub name: String,
    pub code: String,
}

impl Department {
    pub fn new(company_id: i64, name: impl Into<String>, code: impl Into<String>) -> Self {
        Self {
            id: None,
            company_id,
            branch_id: None,
            name: name.into(),
            code: code.into(),
        }
    }
}

impl Validate for Department {
    fn validate(&self) -> AppResult<()> {
        if self.company_id <= 0 {
            return Err(AppError::Validation(
                "department must reference a valid company".into(),
            ));
        }
        if matches!(self.branch_id, Some(id) if id <= 0) {
            return Err(AppError::Validation(
                "department's branch_id, when set, must be a valid branch".into(),
            ));
        }
        require_non_empty("department name", &self.name)?;
        require_max_length("department name", &self.name, 200)?;
        require_non_empty("department code", &self.code)?;
        require_max_length("department code", &self.code, 20)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_rejects_non_positive_company_id() {
        let department = Department::new(0, "Finance", "FIN");
        assert!(department.validate().is_err());
    }

    #[test]
    fn validate_rejects_invalid_branch_id_when_present() {
        let mut department = Department::new(1, "Finance", "FIN");
        department.branch_id = Some(0);
        assert!(department.validate().is_err());
    }

    #[test]
    fn validate_accepts_department_without_branch() {
        let department = Department::new(1, "Finance", "FIN");
        assert!(department.validate().is_ok());
    }

    #[test]
    fn validate_accepts_department_scoped_to_branch() {
        let mut department = Department::new(1, "Finance", "FIN");
        department.branch_id = Some(1);
        assert!(department.validate().is_ok());
    }
}
