//! Export formatting for a company's open payables
//! (`accounts_payable::payables::Payable`, Development Order step 9).

use chrono::NaiveDate;

use crate::backend::accounting::accounts_payable::payables::Payable;

pub fn to_rows(payables: &[Payable], as_of: NaiveDate) -> (Vec<&'static str>, Vec<Vec<String>>) {
    let rows = payables
        .iter()
        .map(|p| {
            vec![
                p.reference.clone().unwrap_or_default(),
                p.due_date.to_string(),
                p.amount.to_string(),
                p.amount_paid.to_string(),
                p.balance().to_string(),
                p.is_overdue(as_of).to_string(),
            ]
        })
        .collect();
    (
        vec![
            "Reference",
            "Due Date",
            "Amount",
            "Paid",
            "Balance",
            "Overdue",
        ],
        rows,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn to_rows_shapes_one_row_per_payable() {
        let payable = Payable::new(
            1,
            1,
            NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2026, 1, 31).unwrap(),
            dec!(500),
        );
        let (headers, rows) = to_rows(&[payable], NaiveDate::from_ymd_opt(2026, 1, 15).unwrap());
        assert_eq!(headers.len(), 6);
        assert_eq!(rows[0][5], "false"); // not yet overdue
    }
}
