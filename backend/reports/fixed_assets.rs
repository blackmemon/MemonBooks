//! Fixed asset register: every asset with its cost, accumulated
//! depreciation, and current book value (Development Order step 13).

use crate::backend::fixed_assets::assets::Asset;

pub fn to_rows(assets: &[Asset]) -> (Vec<&'static str>, Vec<Vec<String>>) {
    let rows = assets
        .iter()
        .map(|asset| {
            vec![
                asset.name.clone(),
                asset.acquisition_date.to_string(),
                asset.acquisition_cost.to_string(),
                asset.accumulated_depreciation.to_string(),
                asset.book_value().to_string(),
                asset.is_disposed.to_string(),
            ]
        })
        .collect();
    (
        vec![
            "Asset",
            "Acquired",
            "Cost",
            "Accum. Depreciation",
            "Book Value",
            "Disposed",
        ],
        rows,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use rust_decimal_macros::dec;

    #[test]
    fn to_rows_shapes_one_row_per_asset() {
        let mut asset = Asset::new(
            1,
            1,
            "Office Laptop",
            NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            dec!(2000),
            dec!(200),
            5,
        );
        asset.accumulated_depreciation = dec!(360);
        let (headers, rows) = to_rows(&[asset]);
        assert_eq!(headers.len(), 6);
        assert_eq!(rows[0][4], "1640");
    }
}
