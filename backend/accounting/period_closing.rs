//! Closing a fiscal period has two independent parts: enforcing that
//! periods close in chronological order (the actual Open -> Closed
//! status flip is `FiscalPeriodStatus`'s state machine from step 5), and
//! -- separately, opt-in, typically only at a fiscal year boundary --
//! generating the closing entry that zeroes revenue and expense accounts
//! into retained earnings.
//!
//! The closing entry is still just a `Transaction`: whatever math this
//! module does to work out its lines, the transaction engine
//! independently re-checks that it balances before it can post, so a
//! mistake here produces a rejected (or semantically wrong but still
//! balanced) entry, never an unbalanced one.

use chrono::NaiveDate;
use rust_decimal::Decimal;

use crate::backend::accounting::accounting_rules::AccountType;
use crate::backend::accounting::fiscal_periods::FiscalPeriod;
use crate::backend::core::errors::{AppError, AppResult};
use crate::backend::transactions::transaction::Transaction;
use crate::backend::transactions::transaction_lines::TransactionLine;
use crate::backend::transactions::transaction_types::TransactionType;

/// Periods must close in chronological order -- otherwise a correction
/// later posted to an earlier-but-still-open period could retroactively
/// change a since-closed period's reported numbers without anyone
/// noticing. `other_periods` only needs to contain this company's other
/// periods; anything not still `Open` is ignored.
pub fn can_close(period: &FiscalPeriod, other_periods: &[FiscalPeriod]) -> AppResult<()> {
    if let Some(earlier) = other_periods
        .iter()
        .find(|p| p.accepts_postings() && p.start_date < period.start_date)
    {
        return Err(AppError::Validation(format!(
            "cannot close '{}' while earlier period '{}' is still open",
            period.name, earlier.name
        )));
    }
    Ok(())
}

/// One revenue or expense account's balance going into a closing entry,
/// under the trial balance convention (positive = debit, negative =
/// credit).
pub struct ClosingBalance {
    pub account_id: i64,
    pub account_type: AccountType,
    pub net_balance: Decimal,
}

/// Builds the transaction that zeroes every given revenue/expense
/// account, transferring their combined net income (or loss) to
/// `retained_earnings_account_id`. Returns `None` if every balance is
/// already zero -- there's nothing to close.
pub fn build_closing_transaction(
    company_id: i64,
    closing_date: NaiveDate,
    retained_earnings_account_id: i64,
    balances: &[ClosingBalance],
) -> Option<Transaction> {
    let mut lines = Vec::new();
    let mut net_income = Decimal::ZERO;

    for balance in balances {
        if balance.net_balance == Decimal::ZERO {
            continue;
        }
        debug_assert!(
            matches!(
                balance.account_type,
                AccountType::Revenue | AccountType::Expense
            ),
            "closing entries only zero revenue and expense accounts"
        );

        // Zero the account: its balance moves to the opposite side.
        if balance.net_balance > Decimal::ZERO {
            lines.push(TransactionLine::credit(
                balance.account_id,
                balance.net_balance,
            ));
        } else {
            lines.push(TransactionLine::debit(
                balance.account_id,
                -balance.net_balance,
            ));
        }

        // Revenue (normally a credit / negative net here) adds to net
        // income; expense (normally a debit / positive net) subtracts
        // from it -- both are just `-net_balance`.
        net_income -= balance.net_balance;
    }

    if lines.is_empty() {
        return None;
    }

    if net_income > Decimal::ZERO {
        lines.push(TransactionLine::credit(
            retained_earnings_account_id,
            net_income,
        ));
    } else if net_income < Decimal::ZERO {
        lines.push(TransactionLine::debit(
            retained_earnings_account_id,
            -net_income,
        ));
    }

    let mut transaction =
        Transaction::new(company_id, TransactionType::ManualJournal, closing_date);
    transaction.description =
        Some("Closing entry: transfer net income to retained earnings".into());
    transaction.lines = lines;
    Some(transaction)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::core::validation::Validate;
    use rust_decimal_macros::dec;

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    fn period(name: &str, start: NaiveDate, end: NaiveDate) -> FiscalPeriod {
        FiscalPeriod::new(1, name, start, end)
    }

    #[test]
    fn can_close_allows_closing_when_no_earlier_period_is_open() {
        let january = period("January 2026", date(2026, 1, 1), date(2026, 1, 31));
        let mut december = period("December 2025", date(2025, 12, 1), date(2025, 12, 31));
        december.status = crate::backend::accounting::fiscal_periods::FiscalPeriodStatus::Closed;

        assert!(can_close(&january, &[december]).is_ok());
    }

    #[test]
    fn can_close_rejects_closing_out_of_order() {
        let january = period("January 2026", date(2026, 1, 1), date(2026, 1, 31));
        let december = period("December 2025", date(2025, 12, 1), date(2025, 12, 31));

        assert!(can_close(&january, &[december]).is_err());
    }

    #[test]
    fn build_closing_transaction_returns_none_when_nothing_to_close() {
        let balances = [ClosingBalance {
            account_id: 1,
            account_type: AccountType::Revenue,
            net_balance: Decimal::ZERO,
        }];
        assert!(build_closing_transaction(1, date(2026, 1, 31), 99, &balances).is_none());
    }

    #[test]
    fn build_closing_transaction_zeroes_accounts_and_balances_on_a_profit() {
        // 5000 of revenue (credit balance, net = -5000) and 3000 of
        // expense (debit balance, net = +3000) -> 2000 net income.
        let balances = [
            ClosingBalance {
                account_id: 10,
                account_type: AccountType::Revenue,
                net_balance: dec!(-5000),
            },
            ClosingBalance {
                account_id: 20,
                account_type: AccountType::Expense,
                net_balance: dec!(3000),
            },
        ];
        let transaction = build_closing_transaction(1, date(2026, 12, 31), 99, &balances).unwrap();

        assert!(transaction.validate().is_ok());
        assert_eq!(transaction.total_debits(), transaction.total_credits());

        let retained_earnings_line = transaction
            .lines
            .iter()
            .find(|l| l.account_id == 99)
            .unwrap();
        assert_eq!(retained_earnings_line.credit, dec!(2000));

        let revenue_line = transaction
            .lines
            .iter()
            .find(|l| l.account_id == 10)
            .unwrap();
        assert_eq!(revenue_line.debit, dec!(5000));

        let expense_line = transaction
            .lines
            .iter()
            .find(|l| l.account_id == 20)
            .unwrap();
        assert_eq!(expense_line.credit, dec!(3000));
    }

    #[test]
    fn build_closing_transaction_debits_retained_earnings_on_a_loss() {
        // 1000 of revenue, 4000 of expense -> 3000 net loss.
        let balances = [
            ClosingBalance {
                account_id: 10,
                account_type: AccountType::Revenue,
                net_balance: dec!(-1000),
            },
            ClosingBalance {
                account_id: 20,
                account_type: AccountType::Expense,
                net_balance: dec!(4000),
            },
        ];
        let transaction = build_closing_transaction(1, date(2026, 12, 31), 99, &balances).unwrap();

        assert!(transaction.validate().is_ok());
        let retained_earnings_line = transaction
            .lines
            .iter()
            .find(|l| l.account_id == 99)
            .unwrap();
        assert_eq!(retained_earnings_line.debit, dec!(3000));
    }
}
