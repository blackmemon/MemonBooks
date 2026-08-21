//! A payroll run: one entry per employee for a pay period, and the GL
//! transaction it posts -- debit salary expense for the total gross pay,
//! credit a deductions-payable account (tax withheld, owed to the
//! authority until remitted) and a net-pay account (cash, or a payable
//! if paid separately) for the rest.

use chrono::NaiveDate;
use rust_decimal::Decimal;

use crate::backend::core::errors::{AppError, AppResult};
use crate::backend::hr::salary::SalaryStructure;
use crate::backend::transactions::transaction::Transaction;
use crate::backend::transactions::transaction_lines::TransactionLine;
use crate::backend::transactions::transaction_types::TransactionType;

#[derive(Debug, Clone, PartialEq)]
pub struct PayrollEntry {
    pub employee_id: i64,
    pub gross_pay: Decimal,
    pub total_deductions: Decimal,
}

impl PayrollEntry {
    pub fn net_pay(&self) -> Decimal {
        self.gross_pay - self.total_deductions
    }
}

impl From<&SalaryStructure> for PayrollEntry {
    fn from(structure: &SalaryStructure) -> Self {
        Self {
            employee_id: structure.employee_id,
            gross_pay: structure.gross_pay(),
            total_deductions: structure.total_deductions(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PayrollRun {
    pub id: Option<i64>,
    pub company_id: i64,
    pub pay_period_start: NaiveDate,
    pub pay_period_end: NaiveDate,
    pub payment_date: NaiveDate,
    pub entries: Vec<PayrollEntry>,
}

impl PayrollRun {
    pub fn new(company_id: i64, pay_period_start: NaiveDate, pay_period_end: NaiveDate, payment_date: NaiveDate) -> Self {
        Self {
            id: None,
            company_id,
            pay_period_start,
            pay_period_end,
            payment_date,
            entries: Vec::new(),
        }
    }

    pub fn total_gross(&self) -> Decimal {
        self.entries.iter().map(|e| e.gross_pay).sum()
    }

    pub fn total_deductions(&self) -> Decimal {
        self.entries.iter().map(|e| e.total_deductions).sum()
    }

    pub fn total_net(&self) -> Decimal {
        self.entries.iter().map(PayrollEntry::net_pay).sum()
    }

    pub fn validate(&self) -> AppResult<()> {
        if self.company_id <= 0 {
            return Err(AppError::Validation("payroll run must reference a valid company".into()));
        }
        if self.entries.is_empty() {
            return Err(AppError::Validation("a payroll run must have at least one entry".into()));
        }
        if self.pay_period_end < self.pay_period_start {
            return Err(AppError::Validation("payroll pay_period_end must not be before pay_period_start".into()));
        }
        Ok(())
    }

    /// Debits salary expense for the total gross pay; credits
    /// deductions payable (withheld amounts owed elsewhere) and net pay
    /// (what actually goes out) for the rest -- always balances, since
    /// net pay is defined as gross minus deductions.
    pub fn build_transaction(&self, salary_expense_account_id: i64, deductions_payable_account_id: i64, net_pay_account_id: i64) -> Transaction {
        let mut transaction = Transaction::new(self.company_id, TransactionType::Payroll, self.payment_date);
        transaction.description = Some(format!("Payroll {} to {}", self.pay_period_start, self.pay_period_end));
        transaction.lines.push(TransactionLine::debit(salary_expense_account_id, self.total_gross()));
        if self.total_deductions() > Decimal::ZERO {
            transaction.lines.push(TransactionLine::credit(deductions_payable_account_id, self.total_deductions()));
        }
        transaction.lines.push(TransactionLine::credit(net_pay_account_id, self.total_net()));
        transaction
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    fn run_with_two_employees() -> PayrollRun {
        let mut run = PayrollRun::new(1, date(2026, 1, 1), date(2026, 1, 31), date(2026, 2, 1));
        run.entries.push(PayrollEntry { employee_id: 1, gross_pay: dec!(6000), total_deductions: dec!(800) });
        run.entries.push(PayrollEntry { employee_id: 2, gross_pay: dec!(4000), total_deductions: dec!(400) });
        run
    }

    #[test]
    fn totals_sum_across_every_entry() {
        let run = run_with_two_employees();
        assert_eq!(run.total_gross(), dec!(10000));
        assert_eq!(run.total_deductions(), dec!(1200));
        assert_eq!(run.total_net(), dec!(8800));
    }

    #[test]
    fn build_transaction_is_balanced() {
        let run = run_with_two_employees();
        let transaction = run.build_transaction(60, 21, 10);
        assert!(transaction.is_balanced());
        assert_eq!(transaction.total_debits(), dec!(10000));
    }

    #[test]
    fn a_run_with_no_deductions_skips_the_deductions_line() {
        let mut run = PayrollRun::new(1, date(2026, 1, 1), date(2026, 1, 31), date(2026, 2, 1));
        run.entries.push(PayrollEntry { employee_id: 1, gross_pay: dec!(1000), total_deductions: Decimal::ZERO });
        let transaction = run.build_transaction(60, 21, 10);
        assert_eq!(transaction.lines.len(), 2);
        assert!(transaction.is_balanced());
    }
}
