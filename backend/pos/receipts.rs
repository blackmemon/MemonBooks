//! Rendering a completed `PosSale` as a printable plain-text receipt.

use crate::backend::pos::sales::PosSale;

const RECEIPT_WIDTH: usize = 32; // typical 80mm thermal printer, ~32 chars/line

fn line_of(ch: char) -> String {
    ch.to_string().repeat(RECEIPT_WIDTH)
}

fn justify(left: &str, right: &str) -> String {
    let padding = RECEIPT_WIDTH.saturating_sub(left.len() + right.len()).max(1);
    format!("{left}{}{right}", " ".repeat(padding))
}

/// `company_name` and `payment_method_labels` are supplied by the caller
/// so this module stays free of any dependency on the party/organization
/// or payments modules' exact enum shapes.
pub fn render_receipt(company_name: &str, sale: &PosSale, payment_method_labels: &[(&str, String)]) -> String {
    let mut out = String::new();
    out.push_str(company_name);
    out.push('\n');
    out.push_str(&sale.sale_date.to_string());
    out.push('\n');
    out.push_str(&line_of('-'));
    out.push('\n');

    for line in &sale.lines {
        out.push_str(&line.description);
        out.push('\n');
        let qty_price = format!("{} x {}", line.quantity, line.unit_price);
        out.push_str(&justify(&qty_price, &line.amount().to_string()));
        out.push('\n');
    }

    out.push_str(&line_of('-'));
    out.push('\n');
    out.push_str(&justify("TOTAL", &sale.total().to_string()));
    out.push('\n');

    for (method, label) in payment_method_labels {
        if let Some(payment) = sale.payments.iter().find(|p| p.method.as_str() == *method) {
            out.push_str(&justify(label, &payment.amount.to_string()));
            out.push('\n');
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::pos::payments::{PaymentMethod, PosPayment};
    use crate::backend::pos::sales::PosSaleLine;
    use chrono::NaiveDate;
    use rust_decimal_macros::dec;

    #[test]
    fn render_receipt_includes_every_line_item_and_the_total() {
        let mut sale = PosSale::new(1, 1, 1, NaiveDate::from_ymd_opt(2026, 1, 15).unwrap());
        sale.lines.push(PosSaleLine::new(10, 40, "Widget", dec!(2), dec!(50)));
        sale.payments.push(PosPayment::new(PaymentMethod::Cash, dec!(100)));

        let receipt = render_receipt("Acme Retail", &sale, &[("cash", "Cash".to_string())]);
        assert!(receipt.contains("Widget"));
        assert!(receipt.contains("TOTAL"));
        assert!(receipt.contains("100"));
        assert!(receipt.contains("Cash"));
    }
}
