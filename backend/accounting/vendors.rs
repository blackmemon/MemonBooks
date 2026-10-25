//! Vendors, without any screen: the vendor table's column names
//! (`VENDOR_HEADERS`) and the `COL_*` indexes into it, which columns are
//! shown or locked, the CSV export's columns and rows, the sort order and
//! the cell text of each column, the payment-method and type lists, and
//! the small formatting helpers the vendor page uses.
//!
//! This is the non-screen side of the vendors view
//! (`vendors_view.rs` in the maintain views). It must stay free of UI: no
//! widgets, no styles, nothing imported from the view layer. Add a new
//! column here (name, index, visibility); add anything that draws or
//! reacts to a click in the view.

use chrono::NaiveDate;
use rust_decimal::Decimal;

use crate::backend::parties::vendors::{Vendor, VendorAttachment};

/// Every column of the vendor table, in display order. This is the only
/// place a column's name is written: everything else refers to a column
/// by its index, through the `COL_*` constants below.
pub(crate) const VENDOR_HEADERS: [&str; 21] = [
    "S.No",
    "",
    "Vendor ID",
    "Vendor Name",
    "Company Name",
    "Phone",
    "Email",
    "Website",
    "Mailing Address",
    "Shipping Address",
    "Payment Terms",
    "Debit",
    "Credit",
    "Balance",
    "Payment Method",
    "Attachments",
    "Description",
    "Status",
    "Create Timestamp",
    "Modified Timestamp",
    "Custom Fields",
];
pub(crate) const COL_SERIAL: usize = 0;
/// The unnamed select-box column.
pub(crate) const COL_SELECT: usize = 1;
pub(crate) const COL_VENDOR_ID: usize = 2;
pub(crate) const COL_VENDOR_NAME: usize = 3;
pub(crate) const COL_COMPANY_NAME: usize = 4;
pub(crate) const COL_PHONE: usize = 5;
pub(crate) const COL_EMAIL: usize = 6;
pub(crate) const COL_WEBSITE: usize = 7;
pub(crate) const COL_MAILING_ADDRESS: usize = 8;
pub(crate) const COL_SHIPPING_ADDRESS: usize = 9;
pub(crate) const COL_PAYMENT_TERMS: usize = 10;
pub(crate) const COL_DEBIT: usize = 11;
pub(crate) const COL_CREDIT: usize = 12;
pub(crate) const COL_BALANCE: usize = 13;
pub(crate) const COL_PAYMENT_METHOD: usize = 14;
pub(crate) const COL_ATTACHMENTS: usize = 15;
pub(crate) const COL_DESCRIPTION: usize = 16;
pub(crate) const COL_STATUS: usize = 17;
pub(crate) const COL_CREATE_TIMESTAMP: usize = 18;
pub(crate) const COL_MODIFIED_TIMESTAMP: usize = 19;
pub(crate) const COL_CUSTOM_FIELDS: usize = 20;
/// The first column after the serial number and the select box.
pub(crate) const FIRST_DATA_COLUMN: usize = COL_VENDOR_ID;

/// Columns the user can never hide.
pub(crate) const VENDOR_ALWAYS_SELECTED: [usize; 10] = [
    COL_SERIAL,
    COL_VENDOR_ID,
    COL_VENDOR_NAME,
    COL_COMPANY_NAME,
    COL_PHONE,
    COL_EMAIL,
    COL_MAILING_ADDRESS,
    COL_DEBIT,
    COL_CREDIT,
    COL_BALANCE,
];
pub(crate) const VENDOR_DEFAULT_VISIBLE: [usize; 10] = [
    COL_SERIAL,
    COL_VENDOR_ID,
    COL_VENDOR_NAME,
    COL_COMPANY_NAME,
    COL_PHONE,
    COL_EMAIL,
    COL_MAILING_ADDRESS,
    COL_DEBIT,
    COL_CREDIT,
    COL_BALANCE,
];
/// Columns shown before the user changes anything.
pub(crate) const VENDOR_VISIBLE_COLUMNS: [usize; 11] = [
    COL_SERIAL,
    COL_SELECT,
    COL_VENDOR_ID,
    COL_VENDOR_NAME,
    COL_COMPANY_NAME,
    COL_PHONE,
    COL_EMAIL,
    COL_MAILING_ADDRESS,
    COL_DEBIT,
    COL_CREDIT,
    COL_BALANCE,
];
/// Columns the picker can show or hide, in the picker's order.
pub(crate) const VENDOR_OPTIONAL_COLUMNS: [usize; 10] = [
    COL_WEBSITE,
    COL_SHIPPING_ADDRESS,
    COL_PAYMENT_TERMS,
    COL_PAYMENT_METHOD,
    COL_ATTACHMENTS,
    COL_DESCRIPTION,
    COL_STATUS,
    COL_CREATE_TIMESTAMP,
    COL_MODIFIED_TIMESTAMP,
    COL_CUSTOM_FIELDS,
];
/// The picker's labels, looked up in `VENDOR_HEADERS` at compile time.
pub(crate) const VENDOR_OPTIONAL_LABELS: [&str; 10] = header_names(VENDOR_OPTIONAL_COLUMNS);

/// `VENDOR_HEADERS[column]` for each of `columns`.
pub(crate) const fn header_names<const N: usize>(columns: [usize; N]) -> [&'static str; N] {
    let mut names = [""; N];
    let mut index = 0;
    while index < N {
        names[index] = VENDOR_HEADERS[columns[index]];
        index += 1;
    }
    names
}

/// A CSV export column: a table column, or the vendor type, which the
/// table offers only as a filter.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum ExportColumn {
    Table(usize),
    VendorType,
}
pub(crate) const VENDOR_TYPE_LABEL: &str = "Vendor Type";
/// What the CSV export writes, in order: the original export's columns
/// first, then the profile fields.
pub(crate) const VENDOR_EXPORT_COLUMNS: [ExportColumn; 14] = [
    ExportColumn::Table(COL_VENDOR_NAME),
    ExportColumn::Table(COL_COMPANY_NAME),
    ExportColumn::Table(COL_EMAIL),
    ExportColumn::Table(COL_PHONE),
    ExportColumn::VendorType,
    ExportColumn::Table(COL_PAYMENT_TERMS),
    ExportColumn::Table(COL_STATUS),
    ExportColumn::Table(COL_WEBSITE),
    ExportColumn::Table(COL_MAILING_ADDRESS),
    ExportColumn::Table(COL_SHIPPING_ADDRESS),
    ExportColumn::Table(COL_PAYMENT_METHOD),
    ExportColumn::Table(COL_DESCRIPTION),
    ExportColumn::Table(COL_CUSTOM_FIELDS),
    ExportColumn::Table(COL_ATTACHMENTS),
];

pub(crate) const VENDOR_TYPE_OPTIONS: [&str; 6] = [
    "All",
    "Supplier",
    "Contractor",
    "Distributor",
    "Manufacturer",
    "Other",
];
pub(crate) const VENDOR_STATUS_OPTIONS: [&str; 2] = ["Active", "Inactive"];
pub(crate) const PAGE_SIZES: [usize; 4] = [10, 25, 50, 100];
pub(crate) const DEFAULT_PAGE_SIZE: usize = 50;

pub(crate) fn export_headers() -> Vec<&'static str> {
    VENDOR_EXPORT_COLUMNS
        .iter()
        .map(|column| match column {
            ExportColumn::Table(column) => VENDOR_HEADERS[*column],
            ExportColumn::VendorType => VENDOR_TYPE_LABEL,
        })
        .collect()
}

pub(crate) fn export_row(vendor: &Vendor) -> Vec<String> {
    VENDOR_EXPORT_COLUMNS
        .iter()
        .map(|column| match *column {
            ExportColumn::VendorType => vendor.vendor_type.clone(),
            ExportColumn::Table(COL_PAYMENT_TERMS) => vendor.payment_terms_days.to_string(),
            ExportColumn::Table(COL_STATUS) => (if vendor.is_active {
                "Active"
            } else {
                "Inactive"
            })
            .to_string(),
            ExportColumn::Table(column) => column_text(vendor, column).unwrap_or_default(),
        })
        .collect()
}

pub(crate) fn optional_input(value: &str) -> Option<String> {
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_string())
}

pub(crate) fn default_column_visibility() -> Vec<bool> {
    (0..VENDOR_HEADERS.len())
        .map(|column| {
            column == COL_SELECT
                || VENDOR_DEFAULT_VISIBLE.contains(&column)
                    && VENDOR_VISIBLE_COLUMNS.contains(&column)
        })
        .collect()
}

/// `value`, or `--` when it is empty.
pub(crate) fn dash(value: &str) -> String {
    if value.is_empty() {
        "--".to_string()
    } else {
        value.to_string()
    }
}

/// The attachments' display names, comma-separated.
pub(crate) fn attachment_names(attachments: &[VendorAttachment]) -> String {
    attachments
        .iter()
        .map(|attachment| attachment.display_name.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

/// `1,234.50`, with a leading `-` when negative.
pub(crate) fn money(amount: Decimal) -> String {
    let fixed = format!("{:.2}", amount.abs());
    let (whole, cents) = fixed.split_once('.').unwrap_or((&fixed, "00"));
    let mut grouped = String::new();
    for (i, digit) in whole.chars().enumerate() {
        if i > 0 && (whole.len() - i) % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(digit);
    }
    let sign = if amount.is_sign_negative() && !amount.is_zero() {
        "-"
    } else {
        ""
    };
    format!("{sign}{grouped}.{cents}")
}

/// A debit/credit column cell: blank rather than `0.00` for the empty side.
pub(crate) fn side(amount: Decimal) -> String {
    if amount.is_zero() {
        String::new()
    } else {
        money(amount)
    }
}

pub(crate) fn day(date: NaiveDate) -> String {
    date.format("%d %b %Y").to_string()
}

pub(crate) fn compare_by_column(a: &Vendor, b: &Vendor, column: usize) -> std::cmp::Ordering {
    match column {
        COL_VENDOR_NAME => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        COL_PAYMENT_TERMS => a.payment_terms_days.cmp(&b.payment_terms_days),
        COL_STATUS => a.is_active.cmp(&b.is_active),
        _ => vendor_column_value(a, column)
            .to_lowercase()
            .cmp(&vendor_column_value(b, column).to_lowercase()),
    }
}

/// The vendor's own value in `column`, `None` when it has none (or the
/// column holds no per-vendor value yet).
pub(crate) fn column_text(vendor: &Vendor, column: usize) -> Option<String> {
    let custom_fields = || {
        vendor
            .custom_fields
            .iter()
            .map(|field| format!("{}: {}", field.name, field.value))
            .collect::<Vec<_>>()
            .join("; ")
    };
    match column {
        COL_VENDOR_ID => vendor
            .reference_code
            .clone()
            .or_else(|| vendor.id.map(|id| id.to_string())),
        COL_VENDOR_NAME => Some(vendor.name.clone()),
        COL_COMPANY_NAME => vendor.company_name.clone(),
        COL_PHONE => vendor.phone.clone(),
        COL_EMAIL => vendor.email.clone(),
        COL_WEBSITE => vendor.website.clone(),
        COL_MAILING_ADDRESS => vendor.mailing_address.clone(),
        COL_SHIPPING_ADDRESS => vendor.shipping_address.clone(),
        COL_PAYMENT_METHOD => vendor.payment_method.clone(),
        COL_DESCRIPTION => vendor.description.clone(),
        COL_PAYMENT_TERMS => Some(format!("{} days", vendor.payment_terms_days)),
        COL_STATUS => Some(
            if vendor.is_active {
                "Active"
            } else {
                "Archived"
            }
            .into(),
        ),
        COL_ATTACHMENTS => {
            (!vendor.attachments.is_empty()).then(|| attachment_names(&vendor.attachments))
        }
        COL_CUSTOM_FIELDS => (!vendor.custom_fields.is_empty()).then(custom_fields),
        _ => None,
    }
}

/// A table cell's text: the vendor's value, or `--` for an empty
/// profile or accounting column (company, phone and email stay blank).
pub(crate) fn vendor_column_value(vendor: &Vendor, column: usize) -> String {
    column_text(vendor, column).unwrap_or_else(|| match column {
        COL_COMPANY_NAME | COL_PHONE | COL_EMAIL => String::new(),
        _ => "--".into(),
    })
}

/// Whether `column` is the sorted one, and if so its direction.
pub(crate) fn sort_of(sort: Option<(usize, bool)>, column: usize) -> Option<bool> {
    match sort {
        Some((current, ascending)) if current == column => Some(ascending),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn money_groups_thousands_and_keeps_the_sign() {
        use rust_decimal_macros::dec;
        assert_eq!(money(dec!(1234567.5)), "1,234,567.50");
        assert_eq!(money(dec!(-999)), "-999.00");
        assert_eq!(money(dec!(0)), "0.00");
    }

    #[test]
    fn every_column_list_holds_distinct_indexes_into_the_headers() {
        let lists: [&[usize]; 4] = [
            &VENDOR_ALWAYS_SELECTED,
            &VENDOR_DEFAULT_VISIBLE,
            &VENDOR_VISIBLE_COLUMNS,
            &VENDOR_OPTIONAL_COLUMNS,
        ];
        for list in lists {
            for (position, column) in list.iter().enumerate() {
                assert!(*column < VENDOR_HEADERS.len(), "{column} is out of range");
                assert!(
                    !list[..position].contains(column),
                    "{column} is listed twice"
                );
            }
        }
        for column in VENDOR_ALWAYS_SELECTED {
            assert!(VENDOR_DEFAULT_VISIBLE.contains(&column));
        }
        for column in VENDOR_EXPORT_COLUMNS {
            if let ExportColumn::Table(column) = column {
                assert!(column < VENDOR_HEADERS.len());
            }
        }
    }
}
