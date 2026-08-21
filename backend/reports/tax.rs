//! Transaction-level tax detail report: every `TaxTransaction`
//! (Development Order step 14) in a period. `vat_summary.rs` is the
//! netted totals view of the same data.

use crate::backend::tax::tax_transactions::TaxTransaction;

pub fn to_rows(transactions: &[TaxTransaction]) -> (Vec<&'static str>, Vec<Vec<String>>) {
    let rows = transactions
        .iter()
        .map(|t| {
            vec![
                t.tax_date.to_string(),
                t.source_reference.clone().unwrap_or_default(),
                t.direction.as_str().to_string(),
                t.taxable_amount.to_string(),
                t.tax_amount.to_string(),
            ]
        })
        .collect();
    (vec!["Date", "Reference", "Direction", "Taxable Amount", "Tax Amount"], rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::tax::tax_rates::TaxDirection;
    use chrono::NaiveDate;
    use rust_decimal_macros::dec;

    #[test]
    fn to_rows_shapes_one_row_per_tax_transaction() {
        let tax = TaxTransaction::new(1, 1, NaiveDate::from_ymd_opt(2026, 1, 5).unwrap(), dec!(1000), dec!(150), TaxDirection::Output);
        let (headers, rows) = to_rows(&[tax]);
        assert_eq!(headers.len(), 5);
        assert_eq!(rows[0][2], "output");
    }
}
