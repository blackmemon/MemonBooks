//! The cash book: `general_ledger.rs`'s report, specifically for
//! cash/bank accounts -- every posting to them, in date order, with a
//! running balance.

use crate::backend::accounting::general_ledger::GeneralLedgerEntry;

pub struct CashBookAccount {
    pub account_name: String,
    pub entries: Vec<GeneralLedgerEntry>,
}

pub fn to_rows(accounts: &[CashBookAccount]) -> (Vec<&'static str>, Vec<Vec<String>>) {
    let mut rows = Vec::new();
    for account in accounts {
        rows.push(vec![
            account.account_name.clone(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
        ]);
        for entry in &account.entries {
            rows.push(vec![
                entry.transaction_date.to_string(),
                entry.description.clone().unwrap_or_default(),
                entry.debit.to_string(),
                entry.credit.to_string(),
                entry.running_balance.to_string(),
            ]);
        }
    }
    (
        vec!["Date", "Description", "Debit", "Credit", "Balance"],
        rows,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn to_rows_groups_entries_under_an_account_header_row() {
        let accounts = vec![CashBookAccount {
            account_name: "Cash".into(),
            entries: vec![GeneralLedgerEntry {
                transaction_id: 1,
                transaction_date: chrono::NaiveDate::from_ymd_opt(2026, 1, 5).unwrap(),
                description: Some("Opening balance".into()),
                debit: dec!(1000),
                credit: dec!(0),
                running_balance: dec!(1000),
            }],
        }];
        let (_, rows) = to_rows(&accounts);
        assert_eq!(rows[0][0], "Cash");
        assert_eq!(rows[1][2], "1000");
    }
}
