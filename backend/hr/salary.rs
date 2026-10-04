//! An employee's salary structure: named components (base salary,
//! allowances, deductions), effective from a date.

use chrono::NaiveDate;
use rust_decimal::Decimal;

use crate::backend::core::errors::{AppError, AppResult};
use crate::backend::core::validation::{require_non_empty, require_positive, Validate};

#[derive(Debug, Clone, PartialEq)]
pub struct SalaryComponent {
    pub name: String,
    pub amount: Decimal,
    pub is_deduction: bool,
}

impl SalaryComponent {
    pub fn addition(name: impl Into<String>, amount: Decimal) -> Self {
        Self {
            name: name.into(),
            amount,
            is_deduction: false,
        }
    }

    pub fn deduction(name: impl Into<String>, amount: Decimal) -> Self {
        Self {
            name: name.into(),
            amount,
            is_deduction: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SalaryStructure {
    pub id: Option<i64>,
    pub employee_id: i64,
    pub effective_date: NaiveDate,
    pub components: Vec<SalaryComponent>,
}

impl SalaryStructure {
    pub fn new(employee_id: i64, effective_date: NaiveDate) -> Self {
        Self {
            id: None,
            employee_id,
            effective_date,
            components: Vec::new(),
        }
    }

    pub fn gross_pay(&self) -> Decimal {
        self.components
            .iter()
            .filter(|c| !c.is_deduction)
            .map(|c| c.amount)
            .sum()
    }

    pub fn total_deductions(&self) -> Decimal {
        self.components
            .iter()
            .filter(|c| c.is_deduction)
            .map(|c| c.amount)
            .sum()
    }

    pub fn net_pay(&self) -> Decimal {
        self.gross_pay() - self.total_deductions()
    }
}

impl Validate for SalaryStructure {
    fn validate(&self) -> AppResult<()> {
        if self.employee_id <= 0 {
            return Err(AppError::Validation(
                "salary structure must reference a valid employee".into(),
            ));
        }
        if self.components.is_empty() {
            return Err(AppError::Validation(
                "a salary structure must have at least one component".into(),
            ));
        }
        for component in &self.components {
            require_non_empty("salary component name", &component.name)?;
            require_positive("salary component amount", component.amount)?;
        }
        if self.net_pay() < Decimal::ZERO {
            return Err(AppError::Validation(
                "salary structure deductions must not exceed gross pay".into(),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn structure() -> SalaryStructure {
        let mut structure = SalaryStructure::new(1, NaiveDate::from_ymd_opt(2026, 1, 1).unwrap());
        structure
            .components
            .push(SalaryComponent::addition("Base Salary", dec!(5000)));
        structure
            .components
            .push(SalaryComponent::addition("Housing Allowance", dec!(1000)));
        structure
            .components
            .push(SalaryComponent::deduction("Income Tax", dec!(800)));
        structure
    }

    #[test]
    fn net_pay_is_gross_minus_deductions() {
        let s = structure();
        assert_eq!(s.gross_pay(), dec!(6000));
        assert_eq!(s.total_deductions(), dec!(800));
        assert_eq!(s.net_pay(), dec!(5200));
    }

    #[test]
    fn validate_rejects_deductions_exceeding_gross_pay() {
        let mut s = SalaryStructure::new(1, NaiveDate::from_ymd_opt(2026, 1, 1).unwrap());
        s.components
            .push(SalaryComponent::addition("Base Salary", dec!(1000)));
        s.components
            .push(SalaryComponent::deduction("Advance Recovery", dec!(1500)));
        assert!(s.validate().is_err());
    }

    #[test]
    fn validate_accepts_a_well_formed_structure() {
        assert!(structure().validate().is_ok());
    }
}
