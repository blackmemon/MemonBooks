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

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else { return };
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

/// True if `arg` is a bare numeric literal (not a `spacing::`/
/// `typography::`-style constant reference -- those contain a letter)
/// with at least one nonzero digit. An all-zero literal (`0`, `0.0`,
/// `[0.0, 0.0]`) is the "no padding" case and needs no named token.
fn is_forbidden_numeric_literal(arg: &str) -> bool {
    if arg.chars().any(|character| character.is_alphabetic()) {
        return false;
    }
    arg.chars().any(|character| character.is_ascii_digit() && character != '0')
}

fn check_numeric_call(line: &str, needle: &str, reason: &'static str, violations: &mut Vec<&'static str>) {
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

    check_numeric_call(line, ".spacing(", "raw .spacing(N) literal -- reference a spacing.rs constant instead", &mut violations);
    check_numeric_call(line, ".padding(", "raw .padding(N) literal -- reference a spacing.rs constant instead", &mut violations);
    if !line.contains("checkbox(") {
        check_numeric_call(line, ".size(", "raw .size(N) font-size literal -- reference a typography.rs constant instead", &mut violations);
    }
    check_numeric_call(line, ".text_size(", "raw .text_size(N) font-size literal -- reference a typography.rs constant instead", &mut violations);

    if line.contains("Border {") && (line.contains("color:") || line.contains("width:")) {
        violations.push("hand-built Border { .. } with its own color/width -- use borders::card() (or another borders.rs helper) instead");
    }
    if line.contains("\\u{") {
        violations.push("raw \\u{...} glyph escape -- name it in styles/icons.rs and reference that constant instead");
    }

    violations
}

#[test]
fn views_and_components_source_all_styling_from_styles_rs() {
    let root = crate_root();
    let mut files = Vec::new();
    collect_rs_files(&root.join("frontend/views"), &mut files);
    collect_rs_files(&root.join("frontend/components"), &mut files);
    assert!(!files.is_empty(), "expected to find .rs files under frontend/views and frontend/components -- did the layout move?");

    let mut findings = Vec::new();
    for file in &files {
        let content = fs::read_to_string(file).unwrap_or_else(|error| panic!("failed to read {}: {error}", file.display()));
        for (index, line) in production_source(&content).lines().enumerate() {
            for reason in check_line(line) {
                findings.push(format!("{}:{}: {reason}\n    {}", file.strip_prefix(&root).unwrap_or(file).display(), index + 1, line.trim()));
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
