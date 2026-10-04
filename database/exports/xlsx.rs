//! Generic XLSX export via `rust_xlsxwriter`: the same (headers, rows)
//! shape as `csv.rs`, written to a single worksheet.

use std::path::Path;

use rust_xlsxwriter::{Format, Workbook};

use crate::backend::core::errors::{AppError, AppResult};

pub fn export_xlsx(
    sheet_name: &str,
    headers: &[&str],
    rows: &[Vec<String>],
    path: &Path,
) -> AppResult<()> {
    let mut workbook = Workbook::new();
    let bold = Format::new().set_bold();
    let sheet = workbook
        .add_worksheet()
        .set_name(sheet_name)
        .map_err(|err| AppError::Internal(format!("failed to name worksheet: {err}")))?;

    for (col, header) in headers.iter().enumerate() {
        sheet
            .write_string_with_format(0, col as u16, *header, &bold)
            .map_err(|err| AppError::Internal(format!("failed to write XLSX header: {err}")))?;
    }

    for (row_index, row) in rows.iter().enumerate() {
        for (col, value) in row.iter().enumerate() {
            sheet
                .write_string(row_index as u32 + 1, col as u16, value)
                .map_err(|err| AppError::Internal(format!("failed to write XLSX cell: {err}")))?;
        }
    }

    workbook.save(path).map_err(|source| AppError::Io {
        path: path.to_path_buf(),
        source: std::io::Error::other(source),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_a_readable_xlsx_file() {
        let dir = std::env::temp_dir().join(format!("memonbooks-xlsx-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("report.xlsx");

        export_xlsx(
            "Trial Balance",
            &["Account", "Balance"],
            &[vec!["Cash".into(), "100.00".into()]],
            &path,
        )
        .unwrap();

        assert!(path.exists());
        assert!(std::fs::metadata(&path).unwrap().len() > 0);

        std::fs::remove_dir_all(&dir).ok();
    }
}
