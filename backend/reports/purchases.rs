//! Purchase register: every posted bill (Development Order step 10) in
//! a period.

use crate::backend::purchases::bills::Bill;

pub fn to_rows(bills: &[Bill]) -> (Vec<&'static str>, Vec<Vec<String>>) {
    let rows = bills
        .iter()
        .map(|bill| {
            vec![
                bill.id.map(|id| format!("BILL-{id}")).unwrap_or_default(),
                bill.bill_date.to_string(),
                bill.vendor_id.to_string(),
                bill.status.as_str().to_string(),
                bill.total().to_string(),
            ]
        })
        .collect();
    (vec!["Bill", "Date", "Vendor", "Status", "Total"], rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::purchases::bills::BillLine;
    use chrono::NaiveDate;
    use rust_decimal_macros::dec;

    #[test]
    fn to_rows_shapes_one_row_per_bill() {
        let mut bill = Bill::new(
            1,
            1,
            NaiveDate::from_ymd_opt(2026, 1, 5).unwrap(),
            NaiveDate::from_ymd_opt(2026, 2, 4).unwrap(),
        );
        bill.id = Some(3);
        bill.lines
            .push(BillLine::new(50, "Office supplies", dec!(1), dec!(300)));

        let (headers, rows) = to_rows(&[bill]);
        assert_eq!(headers.len(), 5);
        assert_eq!(rows[0][0], "BILL-3");
        assert_eq!(rows[0][4], "300");
    }
}
