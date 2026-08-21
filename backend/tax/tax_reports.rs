//! The tax return calculation: total output tax collected minus total
//! input tax paid over a period, i.e. what's owed to (or reclaimable
//! from) the tax authority. `backend/reports/tax.rs` and
//! `vat_summary.rs` (Development Order step 15) present this; this
//! module is the underlying aggregation.

use chrono::NaiveDate;
use rust_decimal::Decimal;

use crate::backend::tax::tax_rates::TaxDirection;
use crate::backend::tax::tax_transactions::TaxTransaction;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TaxSummary {
    pub period_start: NaiveDate,
    pub period_end: NaiveDate,
    pub total_output_tax: Decimal,
    pub total_input_tax: Decimal,
}

impl TaxSummary {
    /// Positive means owed to the tax authority; negative means
    /// reclaimable (input tax exceeded output tax for the period).
    pub fn net_tax_payable(&self) -> Decimal {
        self.total_output_tax - self.total_input_tax
    }
}

/// Sums every tax transaction dated within `[period_start, period_end]`
/// by direction.
pub fn summarize(transactions: &[TaxTransaction], period_start: NaiveDate, period_end: NaiveDate) -> TaxSummary {
    let in_period = transactions.iter().filter(|t| t.tax_date >= period_start && t.tax_date <= period_end);

    let mut summary = TaxSummary {
        period_start,
        period_end,
        total_output_tax: Decimal::ZERO,
        total_input_tax: Decimal::ZERO,
    };
    for tax in in_period {
        match tax.direction {
            TaxDirection::Output => summary.total_output_tax += tax.tax_amount,
            TaxDirection::Input => summary.total_input_tax += tax.tax_amount,
        }
    }
    summary
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    fn tax(direction: TaxDirection, on: NaiveDate, amount: Decimal) -> TaxTransaction {
        TaxTransaction::new(1, 1, on, amount * dec!(10), amount, direction)
    }

    #[test]
    fn summarize_nets_output_against_input() {
        let transactions = vec![
            tax(TaxDirection::Output, date(2026, 1, 10), dec!(150)),
            tax(TaxDirection::Output, date(2026, 1, 20), dec!(50)),
            tax(TaxDirection::Input, date(2026, 1, 15), dec!(60)),
        ];
        let summary = summarize(&transactions, date(2026, 1, 1), date(2026, 1, 31));
        assert_eq!(summary.total_output_tax, dec!(200));
        assert_eq!(summary.total_input_tax, dec!(60));
        assert_eq!(summary.net_tax_payable(), dec!(140));
    }

    #[test]
    fn transactions_outside_the_period_are_excluded() {
        let transactions = vec![tax(TaxDirection::Output, date(2026, 2, 1), dec!(100))];
        let summary = summarize(&transactions, date(2026, 1, 1), date(2026, 1, 31));
        assert_eq!(summary.total_output_tax, Decimal::ZERO);
    }

    #[test]
    fn more_input_than_output_is_reclaimable() {
        let transactions = vec![
            tax(TaxDirection::Output, date(2026, 1, 5), dec!(50)),
            tax(TaxDirection::Input, date(2026, 1, 5), dec!(200)),
        ];
        let summary = summarize(&transactions, date(2026, 1, 1), date(2026, 1, 31));
        assert_eq!(summary.net_tax_payable(), dec!(-150));
    }
}
