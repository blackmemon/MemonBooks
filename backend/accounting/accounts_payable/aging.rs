//! Accounts payable aging: mirrors `accounts_receivable::aging`.

use chrono::NaiveDate;
use rust_decimal::Decimal;

use crate::backend::accounting::accounting_rules::{aging_bucket_for, AgingBucket};
use crate::backend::accounting::accounts_payable::payables::Payable;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct PayablesAgingSummary {
    pub current: Decimal,
    pub days_1_to_30: Decimal,
    pub days_31_to_60: Decimal,
    pub days_61_to_90: Decimal,
    pub over_90: Decimal,
}

impl PayablesAgingSummary {
    pub fn total(&self) -> Decimal {
        self.current + self.days_1_to_30 + self.days_31_to_60 + self.days_61_to_90 + self.over_90
    }
}

pub fn summarize(payables: &[Payable], as_of: NaiveDate) -> PayablesAgingSummary {
    let mut summary = PayablesAgingSummary::default();
    for payable in payables {
        if payable.is_settled() {
            continue;
        }
        let bucket = aging_bucket_for(payable.due_date, as_of);
        let target = match bucket {
            AgingBucket::Current => &mut summary.current,
            AgingBucket::Days1To30 => &mut summary.days_1_to_30,
            AgingBucket::Days31To60 => &mut summary.days_31_to_60,
            AgingBucket::Days61To90 => &mut summary.days_61_to_90,
            AgingBucket::Over90 => &mut summary.over_90,
        };
        *target += payable.balance();
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

    fn payable(due: NaiveDate, amount: Decimal) -> Payable {
        Payable::new(1, 1, due, due, amount)
    }

    #[test]
    fn summarize_buckets_by_days_overdue() {
        let payables = vec![
            payable(date(2026, 2, 15), dec!(100)),
            payable(date(2026, 1, 1), dec!(200)),
        ];
        let summary = summarize(&payables, date(2026, 2, 1));
        assert_eq!(summary.current, dec!(100));
        assert_eq!(summary.days_31_to_60, dec!(200));
        assert_eq!(summary.total(), dec!(300));
    }

    #[test]
    fn settled_payables_do_not_contribute() {
        let mut settled = payable(date(2026, 1, 1), dec!(500));
        settled.amount_paid = dec!(500);
        let summary = summarize(&[settled], date(2026, 3, 1));
        assert_eq!(summary.total(), Decimal::ZERO);
    }

    #[test]
    fn partially_paid_payables_contribute_only_their_remaining_balance() {
        let mut partially_paid = payable(date(2026, 1, 1), dec!(500));
        partially_paid.amount_paid = dec!(300);
        let summary = summarize(&[partially_paid], date(2026, 1, 1));
        assert_eq!(summary.current, dec!(200));
        assert_eq!(summary.total(), dec!(200));
    }
}
