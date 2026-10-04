//! The statement of retained earnings: opening balance, plus the
//! period's net income, equals the closing balance. No dividend/
//! distribution tracking exists yet, so that adjustment isn't modeled.

use chrono::NaiveDate;
use rust_decimal::Decimal;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RetainedEarningsStatement {
    pub period_start: NaiveDate,
    pub period_end: NaiveDate,
    pub opening_balance: Decimal,
    pub net_income: Decimal,
}

impl RetainedEarningsStatement {
    pub fn closing_balance(&self) -> Decimal {
        self.opening_balance + self.net_income
    }
}

pub fn build(
    period_start: NaiveDate,
    period_end: NaiveDate,
    opening_balance: Decimal,
    net_income: Decimal,
) -> RetainedEarningsStatement {
    RetainedEarningsStatement {
        period_start,
        period_end,
        opening_balance,
        net_income,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn closing_balance_adds_net_income_to_opening() {
        let statement = build(
            NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2026, 12, 31).unwrap(),
            dec!(10000),
            dec!(2500),
        );
        assert_eq!(statement.closing_balance(), dec!(12500));
    }

    #[test]
    fn a_net_loss_reduces_the_closing_balance() {
        let statement = build(
            NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2026, 12, 31).unwrap(),
            dec!(10000),
            dec!(-1500),
        );
        assert_eq!(statement.closing_balance(), dec!(8500));
    }
}
