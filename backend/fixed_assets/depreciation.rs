//! Depreciation calculation and the journal entry it produces. Two
//! methods are supported: straight-line (equal expense every year) and
//! declining balance (a fixed percentage of remaining book value).

use chrono::NaiveDate;
use rust_decimal::Decimal;

use crate::backend::transactions::transaction::Transaction;
use crate::backend::transactions::transaction_lines::TransactionLine;
use crate::backend::transactions::transaction_types::TransactionType;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DepreciationMethod {
    StraightLine,
    /// `rate` is the fraction of remaining book value expensed each year
    /// (e.g. 0.20 for a 20%-per-year declining balance).
    DecliningBalance {
        rate: Decimal,
    },
}

impl DepreciationMethod {
    pub fn as_str(self) -> &'static str {
        match self {
            DepreciationMethod::StraightLine => "straight_line",
            DepreciationMethod::DecliningBalance { .. } => "declining_balance",
        }
    }
}

/// The depreciation expense for one year, never exceeding what would
/// take the asset below its salvage value.
pub fn annual_depreciation(
    method: DepreciationMethod,
    cost: Decimal,
    salvage_value: Decimal,
    useful_life_years: u32,
    book_value_start_of_year: Decimal,
) -> Decimal {
    let depreciable_remaining = (book_value_start_of_year - salvage_value).max(Decimal::ZERO);
    if depreciable_remaining == Decimal::ZERO || useful_life_years == 0 {
        return Decimal::ZERO;
    }

    let raw = match method {
        DepreciationMethod::StraightLine => {
            (cost - salvage_value) / Decimal::from(useful_life_years)
        }
        DepreciationMethod::DecliningBalance { rate } => book_value_start_of_year * rate,
    };
    raw.max(Decimal::ZERO).min(depreciable_remaining)
}

/// Debits depreciation expense, credits accumulated depreciation.
pub fn build_depreciation_transaction(
    company_id: i64,
    depreciation_date: NaiveDate,
    depreciation_expense_gl_account_id: i64,
    accumulated_depreciation_gl_account_id: i64,
    amount: Decimal,
    asset_name: &str,
) -> Transaction {
    let mut transaction = Transaction::new(
        company_id,
        TransactionType::ManualJournal,
        depreciation_date,
    );
    transaction.description = Some(format!("Depreciation: {asset_name}"));
    transaction.lines.push(TransactionLine::debit(
        depreciation_expense_gl_account_id,
        amount,
    ));
    transaction.lines.push(TransactionLine::credit(
        accumulated_depreciation_gl_account_id,
        amount,
    ));
    transaction
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn straight_line_splits_depreciable_cost_evenly() {
        // 10,000 cost, 1,000 salvage, 5 years -> 1,800/year.
        let amount = annual_depreciation(
            DepreciationMethod::StraightLine,
            dec!(10000),
            dec!(1000),
            5,
            dec!(10000),
        );
        assert_eq!(amount, dec!(1800));
    }

    #[test]
    fn straight_line_never_depreciates_below_salvage_value() {
        // Only 500 of depreciable value left, but a full year would be 1,800.
        let amount = annual_depreciation(
            DepreciationMethod::StraightLine,
            dec!(10000),
            dec!(1000),
            5,
            dec!(1500),
        );
        assert_eq!(amount, dec!(500));
    }

    #[test]
    fn declining_balance_applies_the_rate_to_book_value() {
        let amount = annual_depreciation(
            DepreciationMethod::DecliningBalance { rate: dec!(0.20) },
            dec!(10000),
            dec!(0),
            5,
            dec!(10000),
        );
        assert_eq!(amount, dec!(2000));
    }

    #[test]
    fn fully_depreciated_asset_has_no_further_depreciation() {
        let amount = annual_depreciation(
            DepreciationMethod::StraightLine,
            dec!(10000),
            dec!(1000),
            5,
            dec!(1000),
        );
        assert_eq!(amount, Decimal::ZERO);
    }

    #[test]
    fn build_depreciation_transaction_is_balanced() {
        let transaction = build_depreciation_transaction(
            1,
            NaiveDate::from_ymd_opt(2026, 12, 31).unwrap(),
            60,
            15,
            dec!(1800),
            "Office Laptop",
        );
        assert!(transaction.is_balanced());
        assert_eq!(transaction.lines[0].debit, dec!(1800));
    }
}
