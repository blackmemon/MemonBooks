//! The transaction engine: the one place that decides whether a
//! transaction is allowed to post. Per Architecture Rules (Section 3) and
//! the hard invariant in Section 4, this is enforced here -- not left to
//! the UI -- so nothing can reach the ledger unbalanced or into a closed
//! period. Persistence (and the atomic multi-table write) is
//! `AccountingRepository`'s job; this module only decides yes/no and
//! builds the reversal.
//!
//! `AccountingRepository::post_transaction` is the only caller that
//! matters in practice, but `validate_for_posting` is exposed standalone
//! so it can be unit tested without a database.

use crate::backend::accounting::fiscal_periods::FiscalPeriod;
use crate::backend::core::errors::{AppError, AppResult};
use crate::backend::core::validation::Validate;
use crate::backend::transactions::transaction::Transaction;
use crate::backend::transactions::transaction_status::TransactionStatus;

/// Everything a transaction must satisfy before it's allowed to post:
/// well-formed and balanced (`Transaction::validate`), still a draft, and
/// dated within a fiscal period that's open for postings.
pub fn validate_for_posting(transaction: &Transaction, period: &FiscalPeriod) -> AppResult<()> {
    transaction.validate()?;

    if transaction.status != TransactionStatus::Draft {
        return Err(AppError::Validation(
            "only a draft transaction can be posted".into(),
        ));
    }

    if period.company_id != transaction.company_id {
        return Err(AppError::Validation(
            "fiscal period does not belong to this transaction's company".into(),
        ));
    }

    if !period.contains(transaction.transaction_date) {
        return Err(AppError::Validation(format!(
            "transaction date {} does not fall within fiscal period '{}'",
            transaction.transaction_date, period.name
        )));
    }

    if !period.accepts_postings() {
        return Err(AppError::Validation(format!(
            "cannot post to fiscal period '{}' because it is {}",
            period.name,
            period.status.as_str()
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::transactions::transaction_lines::TransactionLine;
    use crate::backend::transactions::transaction_types::TransactionType;
    use chrono::NaiveDate;
    use rust_decimal_macros::dec;

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    fn open_january(company_id: i64) -> FiscalPeriod {
        FiscalPeriod::new(
            company_id,
            "January 2026",
            date(2026, 1, 1),
            date(2026, 1, 31),
        )
    }

    fn balanced_transaction(company_id: i64, on: NaiveDate) -> Transaction {
        Transaction::new(company_id, TransactionType::ManualJournal, on)
            .with_line(TransactionLine::debit(1, dec!(100)))
            .with_line(TransactionLine::credit(2, dec!(100)))
    }

    #[test]
    fn accepts_a_balanced_draft_dated_within_an_open_period() {
        let period = open_january(1);
        let transaction = balanced_transaction(1, date(2026, 1, 15));
        assert!(validate_for_posting(&transaction, &period).is_ok());
    }

    #[test]
    fn rejects_an_unbalanced_transaction() {
        let period = open_january(1);
        let transaction = Transaction::new(1, TransactionType::ManualJournal, date(2026, 1, 15))
            .with_line(TransactionLine::debit(1, dec!(100)))
            .with_line(TransactionLine::credit(2, dec!(99)));
        assert!(validate_for_posting(&transaction, &period).is_err());
    }

    #[test]
    fn rejects_a_transaction_dated_outside_the_period() {
        let period = open_january(1);
        let transaction = balanced_transaction(1, date(2026, 2, 1));
        assert!(validate_for_posting(&transaction, &period).is_err());
    }

    #[test]
    fn rejects_posting_into_a_closed_period() {
        let mut period = open_january(1);
        period.status = crate::backend::accounting::fiscal_periods::FiscalPeriodStatus::Closed;
        let transaction = balanced_transaction(1, date(2026, 1, 15));
        assert!(validate_for_posting(&transaction, &period).is_err());
    }

    #[test]
    fn rejects_a_transaction_already_posted() {
        let period = open_january(1);
        let mut transaction = balanced_transaction(1, date(2026, 1, 15));
        transaction.status = TransactionStatus::Posted;
        assert!(validate_for_posting(&transaction, &period).is_err());
    }

    #[test]
    fn rejects_a_period_belonging_to_a_different_company() {
        let period = open_january(2);
        let transaction = balanced_transaction(1, date(2026, 1, 15));
        assert!(validate_for_posting(&transaction, &period).is_err());
    }
}
