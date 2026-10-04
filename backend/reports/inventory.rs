//! Stock-on-hand report: one row per (product, warehouse) with its
//! current quantity (`inventory::stock`, Development Order step 11).

use rust_decimal::Decimal;

pub struct InventoryLine {
    pub product_sku: String,
    pub product_name: String,
    pub warehouse_name: String,
    pub quantity_on_hand: Decimal,
}

pub fn to_rows(lines: &[InventoryLine]) -> (Vec<&'static str>, Vec<Vec<String>>) {
    let rows = lines
        .iter()
        .map(|line| {
            vec![
                line.product_sku.clone(),
                line.product_name.clone(),
                line.warehouse_name.clone(),
                line.quantity_on_hand.to_string(),
            ]
        })
        .collect();
    (vec!["SKU", "Product", "Warehouse", "On Hand"], rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn to_rows_shapes_one_row_per_line() {
        let line = InventoryLine {
            product_sku: "WIDGET-1".into(),
            product_name: "Widget".into(),
            warehouse_name: "Main Warehouse".into(),
            quantity_on_hand: dec!(42),
        };
        let (headers, rows) = to_rows(&[line]);
        assert_eq!(headers.len(), 4);
        assert_eq!(rows[0][3], "42");
    }
}
