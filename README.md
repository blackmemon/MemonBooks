# MemonBooks

A double-entry accounting / ERP desktop application, built with Rust and
[Iced](https://iced.rs/) (no Electron, Tauri, Flutter, or browser runtime).

Targets: macOS App Store, Microsoft Store (Windows), Linux Flatpak.

## Status

Under active development. `PROJECT_SPEC.md` is the frozen specification and
single source of truth for structure, architecture rules, and build order —
read it before making structural changes.

## Development

The project follows a fixed Development Order (see `PROJECT_SPEC.md`
Section 5), starting from the Cargo/Iced foundation and White Theme, then
the accounting core (chart of accounts -> journal entries -> general ledger
-> trial balance) before any other module.

Money values use `rust_decimal` everywhere; monetary fields are never
`f32`/`f64`.
