//! Barcode scanning support: looking a scanned code up against a
//! product list. `InventoryRepository::get_product_by_barcode` does the
//! same lookup against the database; this pure version exists so the
//! matching rule itself (exact, trimmed) is unit-testable without one.

use crate::backend::inventory::products::Product;

/// Scanners sometimes send a trailing newline/whitespace; trim before
/// comparing.
pub fn find_by_barcode<'a>(products: &'a [Product], scanned_code: &str) -> Option<&'a Product> {
    let scanned_code = scanned_code.trim();
    if scanned_code.is_empty() {
        return None;
    }
    products.iter().find(|p| p.barcode.as_deref().map(str::trim) == Some(scanned_code))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn product_with_barcode(barcode: &str) -> Product {
        let mut product = Product::new(1, "WIDGET-1", "Widget");
        product.barcode = Some(barcode.to_string());
        product
    }

    #[test]
    fn finds_a_product_by_exact_barcode() {
        let products = vec![product_with_barcode("012345678905")];
        assert!(find_by_barcode(&products, "012345678905").is_some());
    }

    #[test]
    fn trims_whitespace_from_the_scanned_code() {
        let products = vec![product_with_barcode("012345678905")];
        assert!(find_by_barcode(&products, "012345678905\n").is_some());
    }

    #[test]
    fn returns_none_for_an_unknown_code() {
        let products = vec![product_with_barcode("012345678905")];
        assert!(find_by_barcode(&products, "999999999999").is_none());
    }

    #[test]
    fn products_without_a_barcode_never_match() {
        let products = vec![Product::new(1, "SERVICE-1", "Consulting Hour")];
        assert!(find_by_barcode(&products, "").is_none());
    }
}
