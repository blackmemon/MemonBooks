//! Cash flow for a period. Simplified to a direct net-change-in-cash
//! view (opening cash balance -> closing cash balance across every
//! bank/cash GL account) rather than a full operating/investing/
//! financing breakdown, since the chart of accounts doesn't tag
//! accounts by cash-flow activity category.

use chrono::NaiveDate;
use rust_decimal::Decimal;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CashFlowStatement {
    pub period_start: NaiveDate,
    pub period_end: NaiveDate,
    pub opening_cash: Decimal,
    pub closing_cash: Decimal,
}

impl CashFlowStatement {
    pub fn net_change(&self) -> Decimal {
        self.closing_cash - self.opening_cash
    }
}

/// `opening_cash`/`closing_cash` are the combined balance across every
/// cash/bank GL account as of the period's start and end.
pub fn build(period_start: NaiveDate, period_end: NaiveDate, opening_cash: Decimal, closing_cash: Decimal) -> CashFlowStatement {
    CashFlowStatement {
        period_start,
        period_end,
        opening_cash,
        closing_cash,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn net_change_is_closing_minus_opening() {
        let statement = build(NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(), NaiveDate::from_ymd_opt(2026, 1, 31).unwrap(), dec!(1000), dec!(1350));
        assert_eq!(statement.net_change(), dec!(350));
    }
}
