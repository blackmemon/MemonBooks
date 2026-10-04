//! Export formatting for `accounting::trial_balance::TrialBalance`
//! (Development Order step 7) -- this module adds no new computation,
//! just the row shape the CSV/PDF/XLSX writers need.

use crate::backend::accounting::trial_balance::TrialBalance;

pub fn to_rows(report: &TrialBalance) -> (Vec<&'static str>, Vec<Vec<String>>) {
    let mut rows: Vec<Vec<String>> = report
        .lines
        .iter()
        .map(|line| {
            vec![
                line.account_code.clone(),
                line.account_name.clone(),
                line.debit.to_string(),
                line.credit.to_string(),
            ]
        })
        .collect();
    rows.push(vec![
        "".into(),
        "Total".into(),
        report.total_debits().to_string(),
        report.total_credits().to_string(),
    ]);
    (vec!["Code", "Account", "Debit", "Credit"], rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::accounting::trial_balance::TrialBalanceLine;
    use rust_decimal_macros::dec;

    #[test]
    fn to_rows_appends_a_totals_row() {
        let report = TrialBalance {
            as_of: chrono::NaiveDate::from_ymd_opt(2026, 1, 31).unwrap(),
            lines: vec![TrialBalanceLine::from_net_movement(
                1,
                "1000",
                "Cash",
                dec!(100),
            )],
        };
        let (headers, rows) = to_rows(&report);
        assert_eq!(headers.len(), 4);
        assert_eq!(rows.last().unwrap()[1], "Total");
    }
}
