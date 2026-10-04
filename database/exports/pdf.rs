//! Generic PDF export via `printpdf`: a simple one-column-per-field
//! tabular render of the same (headers, rows) shape as csv.rs/xlsx.rs --
//! no styling beyond a bold title and header row, which is enough for a
//! printable report.

use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

use printpdf::*;

use crate::backend::core::errors::{AppError, AppResult};

const PAGE_WIDTH_MM: f32 = 210.0; // A4
const PAGE_HEIGHT_MM: f32 = 297.0;
const MARGIN_MM: f32 = 15.0;
const ROW_HEIGHT_MM: f32 = 6.0;
const FONT_SIZE: f32 = 10.0;

pub fn export_pdf(
    title: &str,
    headers: &[&str],
    rows: &[Vec<String>],
    path: &Path,
) -> AppResult<()> {
    let (doc, page1, layer1) =
        PdfDocument::new(title, Mm(PAGE_WIDTH_MM), Mm(PAGE_HEIGHT_MM), "Layer 1");
    let font = doc
        .add_builtin_font(BuiltinFont::Helvetica)
        .map_err(|err| AppError::Internal(format!("failed to load PDF font: {err}")))?;
    let bold_font = doc
        .add_builtin_font(BuiltinFont::HelveticaBold)
        .map_err(|err| AppError::Internal(format!("failed to load PDF bold font: {err}")))?;

    let mut layer = doc.get_page(page1).get_layer(layer1);
    let mut y = PAGE_HEIGHT_MM - MARGIN_MM;

    layer.use_text(title, 16.0, Mm(MARGIN_MM), Mm(y), &bold_font);
    y -= ROW_HEIGHT_MM * 2.0;

    let column_width = (PAGE_WIDTH_MM - 2.0 * MARGIN_MM) / headers.len().max(1) as f32;

    let write_row =
        |layer: &PdfLayerReference, values: &[String], font: &IndirectFontRef, y: f32| {
            for (col, value) in values.iter().enumerate() {
                layer.use_text(
                    value.as_str(),
                    FONT_SIZE,
                    Mm(MARGIN_MM + column_width * col as f32),
                    Mm(y),
                    font,
                );
            }
        };

    write_row(
        &layer,
        &headers.iter().map(|h| h.to_string()).collect::<Vec<_>>(),
        &bold_font,
        y,
    );
    y -= ROW_HEIGHT_MM;

    for row in rows {
        if y < MARGIN_MM {
            let (new_page, new_layer) =
                doc.add_page(Mm(PAGE_WIDTH_MM), Mm(PAGE_HEIGHT_MM), "Layer 1");
            layer = doc.get_page(new_page).get_layer(new_layer);
            y = PAGE_HEIGHT_MM - MARGIN_MM;
        }
        write_row(&layer, row, &font, y);
        y -= ROW_HEIGHT_MM;
    }

    let file = File::create(path).map_err(|source| AppError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    doc.save(&mut BufWriter::new(file))
        .map_err(|err| AppError::Internal(format!("failed to save PDF: {err}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_a_readable_pdf_file() {
        let dir = std::env::temp_dir().join(format!("memonbooks-pdf-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("report.pdf");

        export_pdf(
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

    #[test]
    fn handles_many_rows_by_paginating() {
        let dir = std::env::temp_dir().join(format!(
            "memonbooks-pdf-test-paginate-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("report.pdf");

        let rows: Vec<Vec<String>> = (0..100)
            .map(|i| vec![format!("Account {i}"), "0.00".into()])
            .collect();
        export_pdf("Big Report", &["Account", "Balance"], &rows, &path).unwrap();
        assert!(path.exists());

        std::fs::remove_dir_all(&dir).ok();
    }
}
