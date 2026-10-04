//! An employee's claim for reimbursement of their own spending. Goes
//! through an approval state before it can be reimbursed (unlike
//! `expenses.rs`, which posts immediately). `claimant_name` is a plain
//! string rather than an employee_id foreign key -- HR/employees isn't
//! built until Development Order step 17.

use chrono::NaiveDate;
use rust_decimal::Decimal;

use crate::backend::core::errors::{AppError, AppResult};
use crate::backend::core::validation::{require_non_empty, require_positive, Validate};
use crate::backend::transactions::transaction::Transaction;
use crate::backend::transactions::transaction_lines::TransactionLine;
use crate::backend::transactions::transaction_types::TransactionType;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExpenseClaimStatus {
    Submitted,
    Approved,
    Rejected,
    Reimbursed,
}

impl ExpenseClaimStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            ExpenseClaimStatus::Submitted => "submitted",
            ExpenseClaimStatus::Approved => "approved",
            ExpenseClaimStatus::Rejected => "rejected",
            ExpenseClaimStatus::Reimbursed => "reimbursed",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "submitted" => Some(ExpenseClaimStatus::Submitted),
            "approved" => Some(ExpenseClaimStatus::Approved),
            "rejected" => Some(ExpenseClaimStatus::Rejected),
            "reimbursed" => Some(ExpenseClaimStatus::Reimbursed),
            _ => None,
        }
    }

    /// Submitted -> Approved|Rejected, Approved -> Reimbursed. Rejected
    /// and Reimbursed are terminal.
    pub fn can_transition_to(self, to: ExpenseClaimStatus) -> bool {
        use ExpenseClaimStatus::*;
        matches!(
            (self, to),
            (Submitted, Approved) | (Submitted, Rejected) | (Approved, Reimbursed)
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExpenseClaim {
    pub id: Option<i64>,
    pub company_id: i64,
    pub claimant_name: String,
    pub category_id: i64,
    pub claim_date: NaiveDate,
    pub amount: Decimal,
    pub description: String,
    pub status: ExpenseClaimStatus,
    pub transaction_id: Option<i64>,
}

impl ExpenseClaim {
    pub fn new(
        company_id: i64,
        claimant_name: impl Into<String>,
        category_id: i64,
        claim_date: NaiveDate,
        amount: Decimal,
        description: impl Into<String>,
    ) -> Self {
        Self {
            id: None,
            company_id,
            claimant_name: claimant_name.into(),
            category_id,
            claim_date,
            amount,
            description: description.into(),
            status: ExpenseClaimStatus::Submitted,
            transaction_id: None,
        }
    }

    /// Reimbursing an approved claim: debits the category's expense
    /// account, credits the account reimbursement was paid from.
    pub fn build_reimbursement_transaction(
        &self,
        expense_gl_account_id: i64,
        payment_account_id: i64,
        reimbursement_date: NaiveDate,
    ) -> Transaction {
        let mut transaction = Transaction::new(
            self.company_id,
            TransactionType::Expense,
            reimbursement_date,
        );
        transaction.description = Some(format!(
            "Reimbursement to {}: {}",
            self.claimant_name, self.description
        ));
        transaction
            .lines
            .push(TransactionLine::debit(expense_gl_account_id, self.amount));
        transaction
            .lines
            .push(TransactionLine::credit(payment_account_id, self.amount));
        transaction
    }
}

impl Validate for ExpenseClaim {
    fn validate(&self) -> AppResult<()> {
        if self.company_id <= 0 {
            return Err(AppError::Validation(
                "expense claim must reference a valid company".into(),
            ));
        }
        require_non_empty("claimant name", &self.claimant_name)?;
        if self.category_id <= 0 {
            return Err(AppError::Validation(
                "expense claim must reference a valid category".into(),
            ));
        }
        require_positive("expense claim amount", self.amount)?;
        require_non_empty("expense claim description", &self.description)?;
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
    fn submitted_can_move_to_approved_or_rejected_but_not_reimbursed() {
        assert!(ExpenseClaimStatus::Submitted.can_transition_to(ExpenseClaimStatus::Approved));
        assert!(ExpenseClaimStatus::Submitted.can_transition_to(ExpenseClaimStatus::Rejected));
        assert!(!ExpenseClaimStatus::Submitted.can_transition_to(ExpenseClaimStatus::Reimbursed));
    }

    #[test]
    fn only_approved_can_move_to_reimbursed() {
        assert!(ExpenseClaimStatus::Approved.can_transition_to(ExpenseClaimStatus::Reimbursed));
        assert!(!ExpenseClaimStatus::Rejected.can_transition_to(ExpenseClaimStatus::Reimbursed));
    }

    #[test]
    fn build_reimbursement_transaction_is_balanced() {
        let claim = ExpenseClaim::new(
            1,
            "Jane Doe",
            5,
            date(2026, 1, 1),
            dec!(80),
            "Client dinner",
        );
        let transaction = claim.build_reimbursement_transaction(50, 10, date(2026, 1, 5));
        assert!(transaction.is_balanced());
        assert_eq!(transaction.lines[0].debit, dec!(80));
    }

    #[test]
    fn validate_rejects_blank_claimant_name() {
        let claim = ExpenseClaim::new(1, "", 5, date(2026, 1, 1), dec!(80), "Client dinner");
        assert!(claim.validate().is_err());
    }
}
