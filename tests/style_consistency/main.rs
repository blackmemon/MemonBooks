//! Phase 3 regression guard for the UI consistency audit (Batch 13):
//! scans every `.rs` file under `frontend/views/` and
//! `frontend/components/` for the raw-literal patterns Batches 3-12
//! spent this whole audit migrating away -- hand-built colors, raw
//! font-size/spacing/padding literals, hand-built borders, and inline
//! glyph escapes -- so a future edit can't silently reintroduce one.
//! `frontend/styles/` itself is exempt: it's the canonical source of
//! truth these values are supposed to come from, and legitimately
//! contains the literals this test forbids everywhere else.
//!
//! This only catches the mechanically-checkable categories the Phase 1
//! audit itemized: raw `Color` construction, raw `.size()`/
//! `.text_size()` literals, raw `.spacing()`/`.padding()` literals,
//! hand-built `Border { .. }` values, and raw `\u{...}` glyph escapes.
//! "Is this widget using the matching `styles/*.rs` function, or some
//! other one-off closure that happens to produce the same colors" is a
//! structural/semantic question a line-based text scan can't answer,
//! so it isn't attempted here -- this guards against *regressions*
//! (a new raw literal creeping back in), not against every way the
//! audit's spirit could be violated without one.
//!
//! Three literal shapes are deliberately allowed, matching the
//! exceptions the audit itself documented at each site:
//! - `Color::WHITE` / `Color::BLACK` / `Color::TRANSPARENT`: named
//!   constants, not hex literals -- `colors.rs` documents these as the
//!   standing exception for contrast text/icons on a fixed-color
//!   accent, and for structural transparency.
//! - A bare `0` / `0.0` (or an all-zero array/struct) in a
//!   `.spacing()`/`.padding()` call: "no padding" needs no named
//!   token.
//! - `checkbox(...).size(16)`: the checkbox *widget's* own box
//!   dimension, not a font size (see `typography.rs`'s doc comment).

use std::fs;
use std::path::{Path, PathBuf};

/// The former `frontend/styles/*.rs` modules, now flattened into
/// `frontend/components/`. They are the canonical source of the literals
/// this test forbids elsewhere, so they stay exempt (`toolbar.rs` is not
/// listed: it now holds real component code and is still checked).
const STYLE_MODULES: &[&str] = &[
    "badges.rs",
    "borders.rs",
    "button.rs",
    "calendar.rs",
    "checkbox.rs",
    "colors.rs",
    "combobox.rs",
    "datepicker.rs",
    "dialogs.rs",
    "dropdown.rs",
    "font_scale.rs",
    "fonts.rs",
    "forms.rs",
    "frames.rs",
    "headers.rs",
    "icons.rs",
    "inputs.rs",
    "labels.rs",
    "list.rs",
    "menu.rs",
    "modal.rs",
    "panels.rs",
    "radiobutton.rs",
    "scrollbar.rs",
    "separators.rs",
    "shadows.rs",
    "spacing.rs",
    "states.rs",
    "statusbar.rs",
    "table.rs",
    "tabs.rs",
    "textbox.rs",
    "theme.rs",
    "tooltip.rs",
    "tree.rs",
    "typography.rs",
];

fn is_style_module(path: &Path) -> bool {
    path.parent()
        .and_then(|dir| dir.file_name())
        .and_then(|name| name.to_str())
        == Some("components")
        && path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| STYLE_MODULES.contains(&name))
}

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_rs_files(&path, out);
        } else if path.extension().and_then(|extension| extension.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

/// Text before `#[cfg(test)]` -- every file in this codebase puts its
/// test module last, so cutting off there excludes test-only code
/// (which is free to construct arbitrary values for assertions)
/// without needing a real Rust parser.
fn production_source(content: &str) -> &str {
    match content.find("#[cfg(test)]") {
        Some(index) => &content[..index],
        None => content,
    }
}

/// The text between a call's opening paren (just after it) and the
/// next `)`. Every call site this test checks passes a simple numeric
/// literal, array, or single identifier path -- never a nested call --
/// so stopping at the first `)` is exact for all of them.
fn call_arg(line: &str, after_open_paren: usize) -> Option<&str> {
    let rest = &line[after_open_paren..];
    let close = rest.find(')')?;
    Some(&rest[..close])
}

/// True if a `.size(...)` argument builds a text size from a numeric
/// literal inside an expression (`size + 2.0`, `13.0 * scale`) instead of
/// naming a type-scale token -- only the typography module may do that.
fn is_literal_in_size_expression(arg: &str) -> bool {
    let has_literal = arg
        .split(|c: char| !(c.is_ascii_digit() || c == '.'))
        .any(|part| part.chars().any(|c| c.is_ascii_digit() && c != '0'));
    has_literal && !arg.contains("typography::") && !arg.contains("headers::")
}

/// True if `arg` is a bare numeric literal (not a `spacing::`/
/// `typography::`-style constant reference -- those contain a letter)
/// with at least one nonzero digit. An all-zero literal (`0`, `0.0`,
/// `[0.0, 0.0]`) is the "no padding" case and needs no named token.
fn is_forbidden_numeric_literal(arg: &str) -> bool {
    if arg.chars().any(|character| character.is_alphabetic()) {
        return false;
    }
    arg.chars()
        .any(|character| character.is_ascii_digit() && character != '0')
}

fn check_numeric_call(
    line: &str,
    needle: &str,
    reason: &'static str,
    violations: &mut Vec<&'static str>,
) {
    if let Some(position) = line.find(needle) {
        if let Some(arg) = call_arg(line, position + needle.len()) {
            if is_forbidden_numeric_literal(arg) {
                violations.push(reason);
            }
        }
    }
}

fn check_line(line: &str) -> Vec<&'static str> {
    let mut violations = Vec::new();
    let trimmed = line.trim_start();
    if trimmed.starts_with("//") {
        return violations;
    }

    if line.contains("Color::from_rgb") {
        violations.push("raw Color::from_rgb*() construction -- add/use a Theme method in styles/colors.rs instead");
    }
    if line.contains("Color {") && !line.contains("..theme.") {
        violations.push("hand-built Color { .. } literal not blended over a Theme color -- use a Theme method, or ..theme.X() for an alpha blend");
    }

    check_numeric_call(
        line,
        ".spacing(",
        "raw .spacing(N) literal -- reference a spacing.rs constant instead",
        &mut violations,
    );
    check_numeric_call(
        line,
        ".padding(",
        "raw .padding(N) literal -- reference a spacing.rs constant instead",
        &mut violations,
    );
    if !line.contains("checkbox(") {
        if let Some(position) = line.find(".size(") {
            if let Some(arg) = call_arg(line, position + ".size(".len()) {
                if is_literal_in_size_expression(arg) && !is_forbidden_numeric_literal(arg) {
                    violations.push("numeric literal inside a .size(..) expression -- use a typography.rs token (scale it with typography, not by hand)");
                }
            }
        }
        check_numeric_call(
            line,
            ".size(",
            "raw .size(N) font-size literal -- reference a typography.rs constant instead",
            &mut violations,
        );
    }
    check_numeric_call(
        line,
        ".text_size(",
        "raw .text_size(N) font-size literal -- reference a typography.rs constant instead",
        &mut violations,
    );

    if line.contains("Border {") && (line.contains("color:") || line.contains("width:")) {
        violations.push("hand-built Border { .. } with its own color/width -- use borders::card() (or another borders.rs helper) instead");
    }
    if line.contains("\\u{") {
        violations.push("raw \\u{...} glyph escape -- name it in styles/icons.rs and reference that constant instead");
    }

    violations
}

/// Patterns that name or pick a font family directly. Only `fonts.rs`
/// may do this; everything else references a `fonts::` constant.
const FONT_FAMILY_PATTERNS: &[&str] = &[
    "Font::with_name(",
    "Family::Name(",
    "Family::Monospace",
    "Family::Serif",
    "Family::SansSerif",
    "Font::MONOSPACE",
    "include_bytes!(",
];

#[test]
fn only_fonts_rs_names_a_font_family() {
    let root = crate_root();
    let mut files = Vec::new();
    collect_rs_files(&root.join("frontend/views"), &mut files);
    collect_rs_files(&root.join("frontend/components"), &mut files);
    assert!(!files.is_empty(), "expected to find .rs files under frontend/views and frontend/components -- did the layout move?");

    let fonts_rs = root.join("frontend/components/fonts.rs");
    let mut findings = Vec::new();
    for file in files.iter().filter(|file| **file != fonts_rs) {
        let content = fs::read_to_string(file)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", file.display()));
        for (index, line) in production_source(&content).lines().enumerate() {
            if line.trim_start().starts_with("//") {
                continue;
            }
            if FONT_FAMILY_PATTERNS
                .iter()
                .any(|pattern| line.contains(pattern))
            {
                findings.push(format!(
                    "{}:{}\n    {}",
                    file.strip_prefix(&root).unwrap_or(file).display(),
                    index + 1,
                    line.trim()
                ));
            }
        }
    }

    assert!(
        findings.is_empty(),
        "\n\nFound {} hard-coded font family/file reference(s) outside \
        frontend/components/fonts.rs -- use fonts::DEFAULT / fonts::BOLD / \
        fonts::ICONS instead:\n\n{}\n",
        findings.len(),
        findings.join("\n\n")
    );
}

/// Where the vendor table's columns are named, and the files that work
/// with vendor columns.
const VENDOR_VIEW: &str = "frontend/views/accounting/maintain/vendors_view.rs";
/// Where `VENDOR_HEADERS` is written.
const VENDOR_HEADERS_FILE: &str = "backend/accounting/vendors.rs";
const VENDOR_CODE: &[&str] = &[
    VENDOR_VIEW,
    "frontend/views/accounting/maintain/vendors_view/preview.rs",
    VENDOR_HEADERS_FILE,
    "backend/parties/vendors.rs",
    "backend/repositories/party_repository.rs",
];

/// The names written in `backend/accounting/vendors.rs`'s `VENDOR_HEADERS` array, and
/// the (zero-based) lines the array spans.
fn vendor_headers(content: &str) -> (Vec<String>, std::ops::Range<usize>) {
    let lines: Vec<&str> = content.lines().collect();
    let start = lines
        .iter()
        .position(|line| {
            line.trim_start_matches("pub(crate) ")
                .starts_with("const VENDOR_HEADERS")
        })
        .expect("backend/accounting/vendors.rs defines VENDOR_HEADERS");
    let end = start
        + lines[start..]
            .iter()
            .position(|line| line.trim() == "];")
            .expect("VENDOR_HEADERS is closed");
    let names = lines[start + 1..end]
        .iter()
        .filter_map(|line| line.trim().strip_prefix('"')?.strip_suffix("\","))
        .filter(|name| !name.is_empty())
        .map(String::from)
        .collect();
    (names, start..end + 1)
}

#[test]
fn vendor_column_names_are_written_only_in_vendor_headers() {
    let root = crate_root();
    let headers_file = fs::read_to_string(root.join(VENDOR_HEADERS_FILE))
        .expect("backend/accounting/vendors.rs is readable");
    let (names, header_lines) = vendor_headers(&headers_file);
    assert!(
        names.len() > 10,
        "expected to read VENDOR_HEADERS' names, got {names:?}"
    );

    let mut findings = Vec::new();
    for path in VENDOR_CODE {
        let content = fs::read_to_string(root.join(path))
            .unwrap_or_else(|error| panic!("failed to read {path}: {error}"));
        for (index, line) in production_source(&content).lines().enumerate() {
            if *path == VENDOR_HEADERS_FILE && header_lines.contains(&index) {
                continue;
            }
            if line.trim_start().starts_with("//") {
                continue;
            }
            for name in &names {
                if line.contains(&format!("\"{name}\"")) {
                    findings.push(format!(
                        "{path}:{}: \"{name}\"\n    {}",
                        index + 1,
                        line.trim()
                    ));
                }
            }
        }
    }

    assert!(
        findings.is_empty(),
        "\n\nFound {} vendor column name(s) written outside VENDOR_HEADERS -- refer \
        to the column by index (a COL_* constant, or VENDOR_HEADERS[COL_*]) instead:\n\n{}\n",
        findings.len(),
        findings.join("\n\n")
    );
}

/// Where the bank account fields are named, and the files that work with
/// bank account fields.
const BANK_VIEW: &str = "frontend/views/accounting/maintain/bank_of_accounts_view.rs";
const BANK_CODE: &[&str] = &[
    BANK_VIEW,
    "frontend/views/accounting/maintain/bank_account_card.rs",
    "frontend/views/accounting/maintain/bank_account_form.rs",
    "backend/banking/bank_accounts.rs",
    "backend/banking/bank_ledger.rs",
    "backend/repositories/banking_repository.rs",
    "backend/services/banking_service.rs",
];

/// The names written in the `const <name>` string array of `content`,
/// and the (zero-based) lines the array spans.
fn named_array(content: &str, name: &str) -> (Vec<String>, std::ops::Range<usize>) {
    let lines: Vec<&str> = content.lines().collect();
    let start = lines
        .iter()
        .position(|line| {
            line.trim_start_matches("pub(super) ")
                .starts_with(&format!("const {name}"))
        })
        .unwrap_or_else(|| panic!("expected a const {name}"));
    let end = start
        + lines[start..]
            .iter()
            .position(|line| line.trim() == "];")
            .unwrap_or_else(|| panic!("{name} is not closed"));
    let names = lines[start + 1..end]
        .iter()
        .filter_map(|line| line.trim().strip_prefix('"')?.strip_suffix("\","))
        .filter(|name| !name.is_empty())
        .map(String::from)
        .collect();
    (names, start..end + 1)
}

#[test]
fn bank_field_names_are_written_only_in_bank_fields() {
    assert_field_names_only_in_constant(BANK_VIEW, "BANK_FIELDS", BANK_CODE);
}

/// Where the warehouse fields are named, and the files that work with
/// warehouse, bin and bin-content fields.
const WAREHOUSE_VIEW: &str = "frontend/views/accounting/maintain/warehouse_view.rs";
const WAREHOUSE_CODE: &[&str] = &[
    WAREHOUSE_VIEW,
    "backend/warehouse_layout/layout.rs",
    "backend/warehouse_layout/repository.rs",
    "backend/warehouse_layout/service.rs",
];

#[test]
fn warehouse_field_names_are_written_only_in_warehouse_fields() {
    assert_field_names_only_in_constant(WAREHOUSE_VIEW, "WAREHOUSE_FIELDS", WAREHOUSE_CODE);
}

/// Fails when any name in the `constant` string array of `view` is
/// written as a string literal anywhere else in `files`' production code.
fn assert_field_names_only_in_constant(view_path: &str, constant: &str, files: &[&str]) {
    let root = crate_root();
    let view = fs::read_to_string(root.join(view_path))
        .unwrap_or_else(|error| panic!("failed to read {view_path}: {error}"));
    let (names, field_lines) = named_array(&view, constant);
    assert!(
        names.len() > 10,
        "expected to read {constant}'s names, got {names:?}"
    );

    let mut findings = Vec::new();
    for path in files {
        let content = fs::read_to_string(root.join(path))
            .unwrap_or_else(|error| panic!("failed to read {path}: {error}"));
        for (index, line) in production_source(&content).lines().enumerate() {
            if *path == view_path && field_lines.contains(&index) {
                continue;
            }
            if line.trim_start().starts_with("//") {
                continue;
            }
            for name in &names {
                if line.contains(&format!("\"{name}\"")) {
                    findings.push(format!(
                        "{path}:{}: \"{name}\"\n    {}",
                        index + 1,
                        line.trim()
                    ));
                }
            }
        }
    }

    assert!(
        findings.is_empty(),
        "\n\nFound {} field name(s) written outside {constant} -- refer to the field \
        by index (an F_* constant, or {constant}[F_*]) instead:\n\n{}\n",
        findings.len(),
        findings.join("\n\n")
    );
}

#[test]
fn the_field_name_guard_flags_a_name_written_twice() {
    let source = "const FIELDS: [&str; 2] = [\n    \"Bank Name\",\n    \"Type\",\n];\nfn label() -> &'static str { \"Type\" }\n";
    let (names, lines) = named_array(source, "FIELDS");
    assert_eq!(names, vec!["Bank Name".to_string(), "Type".to_string()]);
    let outside: Vec<usize> = source
        .lines()
        .enumerate()
        .filter(|(index, line)| !lines.contains(index) && line.contains("\"Type\""))
        .map(|(index, _)| index)
        .collect();
    assert_eq!(outside, vec![4]);
}

#[test]
fn views_and_components_source_all_styling_from_styles_rs() {
    let root = crate_root();
    let mut files = Vec::new();
    collect_rs_files(&root.join("frontend/views"), &mut files);
    collect_rs_files(&root.join("frontend/components"), &mut files);
    assert!(!files.is_empty(), "expected to find .rs files under frontend/views and frontend/components -- did the layout move?");

    let mut findings = Vec::new();
    for file in files.iter().filter(|file| !is_style_module(file)) {
        let content = fs::read_to_string(file)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", file.display()));
        for (index, line) in production_source(&content).lines().enumerate() {
            for reason in check_line(line) {
                findings.push(format!(
                    "{}:{}: {reason}\n    {}",
                    file.strip_prefix(&root).unwrap_or(file).display(),
                    index + 1,
                    line.trim()
                ));
            }
        }
    }

    assert!(
        findings.is_empty(),
        "\n\nFound {} raw-literal styling violation(s) outside frontend/styles/ -- \
        every color, font size, spacing/padding value, border, and icon glyph in \
        frontend/views/ and frontend/components/ must come from a frontend/styles/*.rs \
        constant or helper, not a literal written inline:\n\n{}\n",
        findings.len(),
        findings.join("\n\n")
    );
}

#[test]
fn raw_text_sizes_are_flagged_even_inside_expressions() {
    assert!(!check_line("text(\"a\").size(13.0)").is_empty());
    assert!(!check_line("text(\"a\").size(size + 2.0)").is_empty());
    assert!(!check_line("text(\"a\").size(size * 1.5)").is_empty());
    assert!(check_line("text(\"a\").size(typography::body())").is_empty());
    assert!(check_line("text(\"a\").size(size)").is_empty());
    assert!(check_line("checkbox(\"a\", true).size(16)").is_empty());
}

#[test]
fn vendors_bank_accounts_and_warehouses_build_their_page_with_the_shared_page_card() {
    let root = crate_root();
    for view in ["vendors_view", "bank_of_accounts_view", "warehouse_view"] {
        let path = root.join(format!("frontend/views/accounting/maintain/{view}.rs"));
        let content = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
        let source = production_source(&content);
        assert!(
            source.contains("panels::page_card("),
            "{view} must build its list page through panels::page_card"
        );
    }
}
