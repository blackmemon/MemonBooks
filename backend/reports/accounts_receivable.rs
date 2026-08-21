//! Export formatting for a company's open receivables
//! (`accounts_receivable::receivables::Receivable`, Development Order
//! step 9).

use chrono::NaiveDate;

use crate::backend::accounting::accounts_receivable::receivables::Receivable;

pub fn to_rows(receivables: &[Receivable], as_of: NaiveDate) -> (Vec<&'static str>, Vec<Vec<String>>) {
    let rows = receivables
        .iter()
        .map(|r| {
            vec![
                r.reference.clone().unwrap_or_default(),
                r.due_date.to_string(),
                r.amount.to_string(),
                r.amount_paid.to_string(),
                r.balance().to_string(),
                r.is_overdue(as_of).to_string(),
            ]
        })
        .collect();
    (vec!["Reference", "Due Date", "Amount", "Paid", "Balance", "Overdue"], rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn to_rows_shapes_one_row_per_receivable() {
        let receivable = Receivable::new(1, 1, NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(), NaiveDate::from_ymd_opt(2026, 1, 31).unwrap(), dec!(1000));
        let (headers, rows) = to_rows(&[receivable], NaiveDate::from_ymd_opt(2026, 2, 1).unwrap());
        assert_eq!(headers.len(), 6);
        assert_eq!(rows[0][5], "true"); // overdue
    }
}
