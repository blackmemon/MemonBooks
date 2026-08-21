//! Applying a payment made to a vendor across their open payables.
//! Mirrors `accounts_receivable::collections`.

use rust_decimal::Decimal;

use crate::backend::accounting::accounts_payable::payables::Payable;

/// Allocates `amount` across `open_payables` oldest-due-date first.
/// Returns `(payable_id, amount_applied)` pairs.
pub fn allocate_payment(open_payables: &[Payable], amount: Decimal) -> Vec<(i64, Decimal)> {
    let mut remaining = amount;
    let mut sorted: Vec<&Payable> = open_payables.iter().filter(|p| !p.is_settled() && p.id.is_some()).collect();
    sorted.sort_by_key(|p| p.due_date);

    let mut allocations = Vec::new();
    for payable in sorted {
        if remaining <= Decimal::ZERO {
            break;
        }
        let applied = remaining.min(payable.balance());
        if applied > Decimal::ZERO {
            allocations.push((payable.id.unwrap(), applied));
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

    fn payable(id: i64, due: NaiveDate, amount: Decimal) -> Payable {
        let mut p = Payable::new(1, 1, due, due, amount);
        p.id = Some(id);
        p
    }

    #[test]
    fn allocates_oldest_due_date_first() {
        let payables = vec![
            payable(1, date(2026, 2, 1), dec!(500)),
            payable(2, date(2026, 1, 1), dec!(300)),
        ];
        let allocations = allocate_payment(&payables, dec!(400));
        assert_eq!(allocations, vec![(2, dec!(300)), (1, dec!(100))]);
    }

    #[test]
    fn does_not_overallocate_beyond_the_paid_amount() {
        let payables = vec![payable(1, date(2026, 1, 1), dec!(1000))];
        let allocations = allocate_payment(&payables, dec!(250));
        assert_eq!(allocations, vec![(1, dec!(250))]);
    }
}
