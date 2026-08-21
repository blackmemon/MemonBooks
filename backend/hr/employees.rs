//! An employee. `user_id` links to a system login account
//! (`core::users::User`) only when the employee actually needs access
//! to the app -- most don't.

use chrono::NaiveDate;

use crate::backend::core::errors::{AppError, AppResult};
use crate::backend::core::validation::{require_max_length, require_non_empty, Validate};

#[derive(Debug, Clone, PartialEq)]
pub struct Employee {
    pub id: Option<i64>,
    pub company_id: i64,
    pub branch_id: Option<i64>,
    pub department_id: Option<i64>,
    pub user_id: Option<i64>,
    pub employee_code: String,
    pub full_name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub hire_date: NaiveDate,
    pub termination_date: Option<NaiveDate>,
    pub is_active: bool,
}

impl Employee {
    pub fn new(company_id: i64, employee_code: impl Into<String>, full_name: impl Into<String>, hire_date: NaiveDate) -> Self {
        Self {
            id: None,
            company_id,
            branch_id: None,
            department_id: None,
            user_id: None,
            employee_code: employee_code.into(),
            full_name: full_name.into(),
            email: None,
            phone: None,
            hire_date,
            termination_date: None,
            is_active: true,
        }
    }
}

impl Validate for Employee {
    fn validate(&self) -> AppResult<()> {
        if self.company_id <= 0 {
            return Err(AppError::Validation("employee must reference a valid company".into()));
        }
        require_non_empty("employee code", &self.employee_code)?;
        require_max_length("employee code", &self.employee_code, 30)?;
        require_non_empty("employee full name", &self.full_name)?;
        require_max_length("employee full name", &self.full_name, 200)?;
        if let Some(termination_date) = self.termination_date {
            if termination_date < self.hire_date {
                return Err(AppError::Validation("employee termination_date must not be before hire_date".into()));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    #[test]
    fn validate_rejects_termination_before_hire() {
        let mut employee = Employee::new(1, "EMP-1", "Jane Doe", date(2026, 1, 1));
        employee.termination_date = Some(date(2025, 12, 1));
        assert!(employee.validate().is_err());
    }

    #[test]
    fn validate_accepts_a_well_formed_employee() {
        let employee = Employee::new(1, "EMP-1", "Jane Doe", date(2026, 1, 1));
        assert!(employee.validate().is_ok());
    }
}
