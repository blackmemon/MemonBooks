//! Expense register: every direct expense (Development Order step 13)
//! in a period.

use crate::backend::expenses::expenses::Expense;

pub fn to_rows(expenses: &[Expense]) -> (Vec<&'static str>, Vec<Vec<String>>) {
    let rows = expenses
        .iter()
        .map(|expense| {
            vec![
                expense.expense_date.to_string(),
                expense.description.clone(),
                expense.amount.to_string(),
            ]
        })
        .collect();
    (vec!["Date", "Description", "Amount"], rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use rust_decimal_macros::dec;

    #[test]
    fn to_rows_shapes_one_row_per_expense() {
        let expense = Expense::new(
            1,
            5,
            NaiveDate::from_ymd_opt(2026, 1, 10).unwrap(),
            dec!(45),
            "Taxi fare",
            10,
        );
        let (headers, rows) = to_rows(&[expense]);
        assert_eq!(headers.len(), 3);
        assert_eq!(rows[0][2], "45");
    }
}
