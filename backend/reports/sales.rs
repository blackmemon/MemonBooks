//! Sales register: every posted invoice (Development Order step 10) in
//! a period.

use crate::backend::sales::invoices::Invoice;

pub fn to_rows(invoices: &[Invoice]) -> (Vec<&'static str>, Vec<Vec<String>>) {
    let rows = invoices
        .iter()
        .map(|invoice| {
            vec![
                invoice.id.map(|id| format!("INV-{id}")).unwrap_or_default(),
                invoice.invoice_date.to_string(),
                invoice.customer_id.to_string(),
                invoice.status.as_str().to_string(),
                invoice.total().to_string(),
            ]
        })
        .collect();
    (vec!["Invoice", "Date", "Customer", "Status", "Total"], rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::sales::invoices::InvoiceLine;
    use chrono::NaiveDate;
    use rust_decimal_macros::dec;

    #[test]
    fn to_rows_shapes_one_row_per_invoice() {
        let mut invoice = Invoice::new(
            1,
            1,
            NaiveDate::from_ymd_opt(2026, 1, 5).unwrap(),
            NaiveDate::from_ymd_opt(2026, 2, 4).unwrap(),
        );
        invoice.id = Some(7);
        invoice
            .lines
            .push(InvoiceLine::new(10, "Consulting", dec!(1), dec!(500)));

        let (headers, rows) = to_rows(&[invoice]);
        assert_eq!(headers.len(), 5);
        assert_eq!(rows[0][0], "INV-7");
        assert_eq!(rows[0][4], "500");
    }
}
