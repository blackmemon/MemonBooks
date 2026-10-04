//! A direct company expense (petty cash purchase, misc. cost) --
//! distinct from a vendor bill (step 10, for expenses billed by a
//! vendor with terms) and from an expense claim (this step's
//! `expense_claims.rs`, for an employee's own reimbursable spending).

use chrono::NaiveDate;
use rust_decimal::Decimal;

use crate::backend::core::errors::{AppError, AppResult};
use crate::backend::core::validation::{require_non_empty, require_positive, Validate};
use crate::backend::transactions::transaction::Transaction;
use crate::backend::transactions::transaction_lines::TransactionLine;
use crate::backend::transactions::transaction_types::TransactionType;

#[derive(Debug, Clone, PartialEq)]
pub struct Expense {
    pub id: Option<i64>,
    pub company_id: i64,
    pub category_id: i64,
    pub expense_date: NaiveDate,
    pub amount: Decimal,
    pub description: String,
    /// The cash/bank GL account this was paid from.
    pub payment_account_id: i64,
    pub transaction_id: Option<i64>,
}

impl Expense {
    pub fn new(
        company_id: i64,
        category_id: i64,
        expense_date: NaiveDate,
        amount: Decimal,
        description: impl Into<String>,
        payment_account_id: i64,
    ) -> Self {
        Self {
            id: None,
            company_id,
            category_id,
            expense_date,
            amount,
            description: description.into(),
            payment_account_id,
            transaction_id: None,
        }
    }

    /// Debits the category's expense account, credits the payment
    /// account. `expense_gl_account_id` comes from the category (a
    /// join the repository does, since this type doesn't hold it).
    pub fn build_transaction(&self, expense_gl_account_id: i64) -> Transaction {
        let mut transaction =
            Transaction::new(self.company_id, TransactionType::Expense, self.expense_date);
        transaction.description = Some(self.description.clone());
        transaction
            .lines
            .push(TransactionLine::debit(expense_gl_account_id, self.amount));
        transaction.lines.push(TransactionLine::credit(
            self.payment_account_id,
            self.amount,
        ));
        transaction
    }
}

impl Validate for Expense {
    fn validate(&self) -> AppResult<()> {
        if self.company_id <= 0 {
            return Err(AppError::Validation(
                "expense must reference a valid company".into(),
            ));
        }
        if self.category_id <= 0 {
            return Err(AppError::Validation(
                "expense must reference a valid category".into(),
            ));
        }
        if self.payment_account_id <= 0 {
            return Err(AppError::Validation(
                "expense must reference a valid payment account".into(),
            ));
        }
        require_positive("expense amount", self.amount)?;
        require_non_empty("expense description", &self.description)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn build_transaction_debits_expense_and_credits_payment_account() {
        let expense = Expense::new(
            1,
            5,
            NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            dec!(45),
            "Taxi fare",
            10,
        );
        let transaction = expense.build_transaction(50);
        assert!(transaction.is_balanced());
        assert_eq!(transaction.lines[0].account_id, 50);
        assert_eq!(transaction.lines[0].debit, dec!(45));
        assert_eq!(transaction.lines[1].account_id, 10);
        assert_eq!(transaction.lines[1].credit, dec!(45));
    }

    #[test]
    fn validate_rejects_non_positive_amount() {
        let expense = Expense::new(
            1,
            5,
            NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            Decimal::ZERO,
            "Taxi fare",
            10,
        );
        assert!(expense.validate().is_err());
    }
}
