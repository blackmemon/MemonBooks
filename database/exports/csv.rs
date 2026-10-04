//! Generic CSV export: any report becomes column headers + string rows
//! (see each `backend/reports/*.rs` module's `to_rows()`), and this turns
//! that into RFC 4180-compliant CSV text. No dependency needed for
//! something this small.

/// Quotes a field only when it needs it (contains a comma, quote, or
/// newline), doubling any embedded quotes -- the RFC 4180 escaping rule.
fn escape_field(field: &str) -> String {
    if field.contains(',') || field.contains('"') || field.contains('\n') || field.contains('\r') {
        format!("\"{}\"", field.replace('"', "\"\""))
    } else {
        field.to_string()
    }
}

fn write_row(fields: &[String]) -> String {
    fields
        .iter()
        .map(|f| escape_field(f))
        .collect::<Vec<_>>()
        .join(",")
}

/// Renders `headers` and `rows` as CSV text, CRLF line endings per RFC 4180.
pub fn export_csv(headers: &[&str], rows: &[Vec<String>]) -> String {
    let mut out = String::new();
    out.push_str(&write_row(
        &headers.iter().map(|h| h.to_string()).collect::<Vec<_>>(),
    ));
    out.push_str("\r\n");
    for row in rows {
        out.push_str(&write_row(row));
        out.push_str("\r\n");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_headers_and_rows_with_crlf_line_endings() {
        let csv = export_csv(
            &["Account", "Balance"],
            &[vec!["Cash".into(), "100.00".into()]],
        );
        assert_eq!(csv, "Account,Balance\r\nCash,100.00\r\n");
    }

    #[test]
    fn quotes_fields_containing_commas_or_quotes() {
        let csv = export_csv(
            &["Description"],
            &[vec!["Widgets, Inc.".into()], vec!["He said \"hi\"".into()]],
        );
        assert_eq!(
            csv,
            "Description\r\n\"Widgets, Inc.\"\r\n\"He said \"\"hi\"\"\"\r\n"
        );
    }

    #[test]
    fn empty_rows_still_produces_a_header_line() {
        let csv = export_csv(&["A", "B"], &[]);
        assert_eq!(csv, "A,B\r\n");
    }
}
