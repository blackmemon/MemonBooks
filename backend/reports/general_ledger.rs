//! Export formatting for `accounting::general_ledger::GeneralLedgerEntry`
//! (Development Order step 7).

use crate::backend::accounting::general_ledger::GeneralLedgerEntry;

pub fn to_rows(entries: &[GeneralLedgerEntry]) -> (Vec<&'static str>, Vec<Vec<String>>) {
    let rows = entries
        .iter()
        .map(|entry| {
            vec![
                entry.transaction_date.to_string(),
                entry.description.clone().unwrap_or_default(),
                entry.debit.to_string(),
                entry.credit.to_string(),
                entry.running_balance.to_string(),
            ]
        })
        .collect();
    (vec!["Date", "Description", "Debit", "Credit", "Balance"], rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn to_rows_shapes_one_row_per_entry() {
        let entries = vec![GeneralLedgerEntry {
            transaction_id: 1,
            transaction_date: chrono::NaiveDate::from_ymd_opt(2026, 1, 5).unwrap(),
            description: Some("Opening balance".into()),
            debit: dec!(1000),
            credit: dec!(0),
            running_balance: dec!(1000),
        }];
        let (headers, rows) = to_rows(&entries);
        assert_eq!(headers.len(), 5);
        assert_eq!(rows.len(), 1);
    }
}
