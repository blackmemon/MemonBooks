//! Export formatting for `tax::tax_reports::TaxSummary` (Development
//! Order step 14) -- the netted VAT return view, as opposed to
//! `tax.rs`'s transaction-level detail.

use crate::backend::tax::tax_reports::TaxSummary;

pub fn to_rows(summary: &TaxSummary) -> (Vec<&'static str>, Vec<Vec<String>>) {
    let rows = vec![
        vec![
            "Output tax (collected)".into(),
            summary.total_output_tax.to_string(),
        ],
        vec![
            "Input tax (paid)".into(),
            summary.total_input_tax.to_string(),
        ],
        vec![
            "Net tax payable".into(),
            summary.net_tax_payable().to_string(),
        ],
    ];
    (vec!["Line", "Amount"], rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn to_rows_includes_the_net_payable_line() {
        let summary = TaxSummary {
            period_start: chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            period_end: chrono::NaiveDate::from_ymd_opt(2026, 1, 31).unwrap(),
            total_output_tax: dec!(200),
            total_input_tax: dec!(60),
        };
        let (_, rows) = to_rows(&summary);
        assert_eq!(rows[2][1], "140");
    }
}
