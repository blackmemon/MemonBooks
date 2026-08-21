//! Moving money between two cash/bank GL accounts (e.g. withdrawing cash
//! from the bank to top up a petty cash fund). Per Architecture Rules
//! (Section 3), banking posts through the transaction engine like
//! everything else -- this builds that transaction.

use chrono::NaiveDate;
use rust_decimal::Decimal;

use crate::backend::core::errors::{AppError, AppResult};
use crate::backend::core::validation::require_positive;
use crate::backend::transactions::transaction::Transaction;
use crate::backend::transactions::transaction_lines::TransactionLine;
use crate::backend::transactions::transaction_types::TransactionType;

pub struct CashTransfer {
    pub company_id: i64,
    pub from_account_id: i64,
    pub to_account_id: i64,
    pub transfer_date: NaiveDate,
    pub amount: Decimal,
    pub memo: Option<String>,
}

impl CashTransfer {
    pub fn new(company_id: i64, from_account_id: i64, to_account_id: i64, transfer_date: NaiveDate, amount: Decimal) -> Self {
        Self {
            company_id,
            from_account_id,
            to_account_id,
            transfer_date,
            amount,
            memo: None,
        }
    }

    pub fn validate(&self) -> AppResult<()> {
        if self.from_account_id == self.to_account_id {
            return Err(AppError::Validation("cash transfer source and destination accounts must differ".into()));
        }
        require_positive("cash transfer amount", self.amount)?;
        Ok(())
    }

    pub fn build_transaction(&self) -> Transaction {
        let mut transaction = Transaction::new(self.company_id, TransactionType::BankTransaction, self.transfer_date);
        transaction.description = self.memo.clone().or_else(|| Some("Cash transfer".into()));
        transaction.lines.push(TransactionLine::debit(self.to_account_id, self.amount));
        transaction.lines.push(TransactionLine::credit(self.from_account_id, self.amount));
        transaction
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn validate_rejects_same_source_and_destination() {
        let transfer = CashTransfer::new(1, 10, 10, NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(), dec!(500));
        assert!(transfer.validate().is_err());
    }

    #[test]
    fn build_transaction_debits_destination_and_credits_source() {
        let transfer = CashTransfer::new(1, 10, 20, NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(), dec!(500));
        let transaction = transfer.build_transaction();
        assert!(transaction.is_balanced());
        assert_eq!(transaction.lines[0].account_id, 20);
        assert_eq!(transaction.lines[0].debit, dec!(500));
        assert_eq!(transaction.lines[1].account_id, 10);
        assert_eq!(transaction.lines[1].credit, dec!(500));
    }
}
