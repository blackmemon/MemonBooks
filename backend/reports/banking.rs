//! Bank account summary: each account's book balance and how many
//! statement lines are still unreconciled (Development Order step 12).

use rust_decimal::Decimal;

use crate::backend::banking::bank_accounts::BankAccount;

pub struct BankAccountSummary {
    pub account: BankAccount,
    pub book_balance: Decimal,
    pub unreconciled_count: usize,
}

pub fn to_rows(summaries: &[BankAccountSummary]) -> (Vec<&'static str>, Vec<Vec<String>>) {
    let rows = summaries
        .iter()
        .map(|s| {
            vec![
                s.account.bank_name.clone(),
                s.account.account_number.clone(),
                s.book_balance.to_string(),
                s.unreconciled_count.to_string(),
            ]
        })
        .collect();
    (vec!["Bank", "Account Number", "Book Balance", "Unreconciled Items"], rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn to_rows_shapes_one_row_per_bank_account() {
        let summary = BankAccountSummary {
            account: BankAccount::new(1, 10, "First National", "123456789", "USD"),
            book_balance: dec!(5000),
            unreconciled_count: 2,
        };
        let (headers, rows) = to_rows(&[summary]);
        assert_eq!(headers.len(), 4);
        assert_eq!(rows[0][3], "2");
    }
}
