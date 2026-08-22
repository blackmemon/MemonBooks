//! Accounts receivable aging: how much of what's owed to the company
//! falls into each `AgingBucket` (Development Order step 9's shared rule,
//! in `accounting_rules.rs`).

use chrono::NaiveDate;
use rust_decimal::Decimal;

use crate::backend::accounting::accounting_rules::{aging_bucket_for, AgingBucket};
use crate::backend::accounting::accounts_receivable::receivables::Receivable;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ReceivablesAgingSummary {
    pub current: Decimal,
    pub days_1_to_30: Decimal,
    pub days_31_to_60: Decimal,
    pub days_61_to_90: Decimal,
    pub over_90: Decimal,
}

impl ReceivablesAgingSummary {
    pub fn total(&self) -> Decimal {
        self.current + self.days_1_to_30 + self.days_31_to_60 + self.days_61_to_90 + self.over_90
    }
}

/// Buckets every unsettled receivable's balance by how overdue it is,
/// relative to `as_of`. Settled receivables contribute nothing.
pub fn summarize(receivables: &[Receivable], as_of: NaiveDate) -> ReceivablesAgingSummary {
    let mut summary = ReceivablesAgingSummary::default();
    for receivable in receivables {
        if receivable.is_settled() {
            continue;
        }
        let bucket = aging_bucket_for(receivable.due_date, as_of);
        let target = match bucket {
            AgingBucket::Current => &mut summary.current,
            AgingBucket::Days1To30 => &mut summary.days_1_to_30,
            AgingBucket::Days31To60 => &mut summary.days_31_to_60,
            AgingBucket::Days61To90 => &mut summary.days_61_to_90,
            AgingBucket::Over90 => &mut summary.over_90,
        };
        *target += receivable.balance();
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

    fn receivable(due: NaiveDate, amount: Decimal) -> Receivable {
        Receivable::new(1, 1, due, due, amount)
    }

    #[test]
    fn summarize_buckets_by_days_overdue() {
        let receivables = vec![
            receivable(date(2026, 2, 15), dec!(100)), // current, as of Feb 1
            receivable(date(2026, 1, 1), dec!(200)),  // 31 days overdue
        ];
        let summary = summarize(&receivables, date(2026, 2, 1));
        assert_eq!(summary.current, dec!(100));
        assert_eq!(summary.days_31_to_60, dec!(200));
        assert_eq!(summary.total(), dec!(300));
    }

    #[test]
    fn settled_receivables_do_not_contribute() {
        let mut settled = receivable(date(2026, 1, 1), dec!(500));
        settled.amount_paid = dec!(500);
        let summary = summarize(&[settled], date(2026, 3, 1));
        assert_eq!(summary.total(), Decimal::ZERO);
    }

    #[test]
    fn partially_paid_receivables_contribute_only_their_remaining_balance() {
        let mut partially_paid = receivable(date(2026, 1, 1), dec!(500));
        partially_paid.amount_paid = dec!(300);
        let summary = summarize(&[partially_paid], date(2026, 1, 1));
        assert_eq!(summary.current, dec!(200));
        assert_eq!(summary.total(), dec!(200));
    }
}
