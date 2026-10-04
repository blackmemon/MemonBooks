//! Applying a payment received from a customer across their open
//! receivables.

use rust_decimal::Decimal;

use crate::backend::accounting::accounts_receivable::receivables::Receivable;

/// Allocates `amount` across `open_receivables` oldest-due-date first,
/// the common default collections strategy. Returns `(receivable_id,
/// amount_applied)` pairs; a receivable not fully covered by `amount`
/// simply doesn't appear (or appears with less than its full balance).
/// Ids without an id (not yet persisted) are skipped -- there's nothing
/// to apply a collection against yet.
pub fn allocate_collection(
    open_receivables: &[Receivable],
    amount: Decimal,
) -> Vec<(i64, Decimal)> {
    let mut remaining = amount;
    let mut sorted: Vec<&Receivable> = open_receivables
        .iter()
        .filter(|r| !r.is_settled() && r.id.is_some())
        .collect();
    sorted.sort_by_key(|r| r.due_date);

    let mut allocations = Vec::new();
    for receivable in sorted {
        if remaining <= Decimal::ZERO {
            break;
        }
        let applied = remaining.min(receivable.balance());
        if applied > Decimal::ZERO {
            allocations.push((receivable.id.unwrap(), applied));
            remaining -= applied;
        }
    }
    allocations
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use rust_decimal_macros::dec;

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    fn receivable(id: i64, due: NaiveDate, amount: Decimal) -> Receivable {
        let mut r = Receivable::new(1, 1, due, due, amount);
        r.id = Some(id);
        r
    }

    #[test]
    fn allocates_oldest_due_date_first() {
        let receivables = vec![
            receivable(1, date(2026, 2, 1), dec!(500)),
            receivable(2, date(2026, 1, 1), dec!(300)),
        ];
        let allocations = allocate_collection(&receivables, dec!(400));
        assert_eq!(allocations, vec![(2, dec!(300)), (1, dec!(100))]);
    }

    #[test]
    fn does_not_overallocate_beyond_the_collected_amount() {
        let receivables = vec![receivable(1, date(2026, 1, 1), dec!(1000))];
        let allocations = allocate_collection(&receivables, dec!(250));
        assert_eq!(allocations, vec![(1, dec!(250))]);
    }

    #[test]
    fn settled_receivables_are_skipped() {
        let mut settled = receivable(1, date(2026, 1, 1), dec!(500));
        settled.amount_paid = dec!(500);
        let receivables = vec![settled, receivable(2, date(2026, 1, 2), dec!(200))];

        let allocations = allocate_collection(&receivables, dec!(1000));
        assert_eq!(allocations, vec![(2, dec!(200))]);
    }

    #[test]
    fn leftover_amount_beyond_all_balances_is_simply_not_allocated() {
        let receivables = vec![receivable(1, date(2026, 1, 1), dec!(100))];
        let allocations = allocate_collection(&receivables, dec!(1000));
        assert_eq!(allocations, vec![(1, dec!(100))]);
    }
}
