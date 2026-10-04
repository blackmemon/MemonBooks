//! CSV and XLSX input for master-data templates. No spreadsheet evaluation.
use crate::backend::core::errors::{AppError, AppResult};
use std::{io::Read, path::Path};
fn error(s: impl Into<String>) -> AppError {
    AppError::Validation(s.into())
}
pub fn csv(input: &str) -> AppResult<Vec<Vec<String>>> {
    let mut rows = vec![];
    let mut row = vec![];
    let mut field = String::new();
    let mut quoted = false;
    let mut chars = input.trim_start_matches('\u{feff}').chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '"' if quoted => {
                if chars.peek() == Some(&'"') {
                    chars.next();
                    field.push('"');
                } else {
                    quoted = false;
                }
            }
            '"' if field.is_empty() => quoted = true,
            ',' if !quoted => row.push(std::mem::take(&mut field)),
            '\n' if !quoted => {
                row.push(std::mem::take(&mut field));
                rows.push(std::mem::take(&mut row));
            }
            '\r' if !quoted => {}
            _ => field.push(ch),
        }
    }
    if quoted {
        return Err(error("Unclosed CSV quote"));
    }
    if !field.is_empty() || !row.is_empty() {
        row.push(field);
        rows.push(row);
    }
    Ok(rows)
}
pub fn read(path: &Path) -> AppResult<Vec<Vec<String>>> {
    let bytes = std::fs::read(path).map_err(|source| AppError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    if bytes.len() > 20_000_000 {
        return Err(error("Import exceeds 20 MB"));
    }
    if path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("csv"))
    {
        return csv(std::str::from_utf8(&bytes).map_err(|_| error("CSV must use UTF-8"))?);
    }
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes))
        .map_err(|_| error("Choose a CSV or XLSX file"))?;
    let mut xml = |name: &str| -> AppResult<String> {
        let f = archive
            .by_name(name)
            .map_err(|_| error(format!("Missing worksheet component {name}")))?;
        let mut s = String::new();
        f.take(20_000_001)
            .read_to_string(&mut s)
            .map_err(|_| error("Cannot decode XLSX XML"))?;
        if s.len() > 20_000_000 {
            return Err(error("Worksheet exceeds 20 MB"));
        }
        Ok(s)
    };
    let strings = match xml("xl/sharedStrings.xml") {
        Ok(s) => shared_strings(&s)?,
        Err(_) => Vec::new(),
    };
    let sheet = xml("xl/worksheets/sheet1.xml")?;
    worksheet(&sheet, &strings)
}
fn text(e: quick_xml::events::BytesText<'_>) -> AppResult<String> {
    let decoded = e.decode().map_err(|_| error("Invalid XML text"))?;
    quick_xml::escape::unescape(&decoded)
        .map(|s| s.into_owned())
        .map_err(|_| error("Invalid XML entity"))
}
fn reference(e: quick_xml::events::BytesRef<'_>) -> AppResult<String> {
    if let Some(ch) = e
        .resolve_char_ref()
        .map_err(|_| error("Invalid XML character reference"))?
    {
        return Ok(ch.to_string());
    }
    let name = e.decode().map_err(|_| error("Invalid XML entity name"))?;
    quick_xml::escape::resolve_predefined_entity(&name)
        .map(str::to_string)
        .ok_or_else(|| error("Unsupported XML entity"))
}
fn shared_strings(xml: &str) -> AppResult<Vec<String>> {
    use quick_xml::events::Event;
    let mut reader = quick_xml::Reader::from_str(xml);
    let mut strings = vec![];
    let mut value = String::new();
    let mut inside = false;
    loop {
        match reader
            .read_event()
            .map_err(|_| error("Malformed shared strings"))?
        {
            Event::Start(e) if e.name().as_ref() == b"si" => value.clear(),
            Event::Start(e) if e.name().as_ref() == b"t" => inside = true,
            Event::Text(e) if inside => value.push_str(&text(e)?),
            Event::GeneralRef(e) if inside => value.push_str(&reference(e)?),
            Event::End(e) if e.name().as_ref() == b"t" => inside = false,
            Event::End(e) if e.name().as_ref() == b"si" => strings.push(value.clone()),
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(strings)
}
fn worksheet(xml: &str, strings: &[String]) -> AppResult<Vec<Vec<String>>> {
    use quick_xml::events::Event;
    let mut r = quick_xml::Reader::from_str(xml);
    let mut rows = vec![];
    let mut row = vec![];
    let mut value = String::new();
    let mut col = 0;
    let mut shared = false;
    let mut inside = false;
    loop {
        match r.read_event().map_err(|_| error("Malformed worksheet"))? {
            Event::Start(e) if e.name().as_ref() == b"row" => row.clear(),
            Event::Start(e) if e.name().as_ref() == b"c" => {
                value.clear();
                shared = false;
                for a in e.attributes().flatten() {
                    if a.key.as_ref() == b"t" {
                        shared = a.value.as_ref() == b"s";
                    }
                    if a.key.as_ref() == b"r" {
                        col = 0;
                        for ch in a.value.iter().take_while(|c| c.is_ascii_alphabetic()) {
                            col = col * 26 + usize::from(ch.to_ascii_uppercase() - b'A' + 1);
                        }
                        col = col.saturating_sub(1);
                        if col > 1000 {
                            return Err(error("Too many columns"));
                        }
                    }
                }
            }
            Event::Start(e) if e.name().as_ref() == b"v" || e.name().as_ref() == b"t" => {
                inside = true
            }
            Event::Text(e) if inside => value.push_str(&text(e)?),
            Event::GeneralRef(e) if inside => value.push_str(&reference(e)?),
            Event::End(e) if e.name().as_ref() == b"v" || e.name().as_ref() == b"t" => {
                inside = false
            }
            Event::End(e) if e.name().as_ref() == b"c" => {
                while row.len() <= col {
                    row.push(String::new());
                }
                row[col] = if shared {
                    strings
                        .get(
                            value
                                .parse::<usize>()
                                .map_err(|_| error("Invalid shared string"))?,
                        )
                        .cloned()
                        .ok_or_else(|| error("Missing shared string"))?
                } else {
                    value.clone()
                };
            }
            Event::End(e) if e.name().as_ref() == b"row" => rows.push(row.clone()),
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(rows)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reads_bom_quotes_commas_newlines() {
        assert_eq!(
            csv("\u{feff}Name,Note\r\n\"A, B\",\"two\nlines\"\r\n").unwrap(),
            vec![vec!["Name", "Note"], vec!["A, B", "two\nlines"]]
        );
        assert!(csv("\"broken").is_err());
    }
    #[test]
    fn imports_exported_xlsx() {
        let path = std::env::temp_dir().join(format!(
            "memonbooks-xlsx-roundtrip-{}.xlsx",
            std::process::id()
        ));
        super::super::xlsx::export_xlsx(
            "Template",
            &["Customer Name", "Email"],
            &[vec!["A & B".into(), "a@example.test".into()]],
            &path,
        )
        .unwrap();
        assert_eq!(read(&path).unwrap()[1], vec!["A & B", "a@example.test"]);
        std::fs::remove_file(path).unwrap();
    }
}
