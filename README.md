# MemonBooks

A double-entry accounting and small-business ERP **desktop application**, written in Rust with the [iced](https://iced.rs/) GUI toolkit and an embedded SQLite database. It runs fully offline on one computer: no server, no web view, no Electron/Tauri/Flutter, no HTTP API.

* Targets (per `PROJECT_SPEC.md`): Linux (Flatpak), Windows (Microsoft Store/MSIX), macOS. Only Linux builds were verified for this documentation.
* Money is `rust_decimal::Decimal` everywhere – never `f32`/`f64`.
* Every posted journal entry balances (debits = credits); corrections are reversals.
* Status: under active development (version 0.1.0, no release yet). `PROJECT_SPEC.md` is the frozen specification and development order.

## Contents

1. [Features](#features)
2. [Prerequisites](#prerequisites)
3. [Build, run and test](#build-run-and-test)
4. [Configuration](#configuration)
5. [Database location and backup](#database-location-and-backup)
6. [First run and usage](#first-run-and-usage)
7. [Project layout](#project-layout)
8. [Documentation](#documentation)
9. [Known limitations](#known-limitations)

## Features

Status: **Implemented**, **Partial** or **Planned**. Full table with evidence: [docs/prd.md](docs/prd.md#5-feature-inventory-and-status).

| Area | Status |
|------|--------|
| Chart of accounts, journal entries (post / reverse), general ledger, trial balance | Implemented |
| Multi-company, fiscal periods (open/closed/locked rules) | Implemented (no screen to close periods → Partial) |
| Vendors, customers, bank accounts, warehouses with bin layout, products | Implemented |
| Sales invoices, purchase bills, payments, credit/debit notes, receivables/payables | Implemented |
| Point of sale (terminals, sessions, cash drawer, barcode) | Implemented |
| Expenses and claims, fixed assets and depreciation, tax rates | Implemented |
| Employees | Implemented |
| Payroll posting, attendance, leave | Partial (backend only) |
| Balance sheet, profit & loss, dashboard, CSV export | Implemented |
| More reports, XLSX/PDF export, CSV/XLSX import | Partial (library code, no screens) |
| Users, roles, password change, Argon2 hashing, TOTP | Partial (login is skipped at start-up; only `users.manage` is enforced) |
| Backup/restore, licensing, notifications | Partial (backend only) |
| Light/dark theme, font-size setting, responsive layout | Implemented |
| Urdu/Arabic UI, keyboard shortcuts | Planned |

## Prerequisites

| | Linux | Windows |
|---|-------|---------|
| Rust | stable toolchain via [rustup](https://rustup.rs) (`rust-toolchain.toml` selects `stable`) | same; use the default MSVC toolchain |
| C toolchain | `gcc`/`cc`, `pkg-config` | Visual Studio Build Tools ("Desktop development with C++") |
| System libraries | GTK 3 development files (the native file dialog uses `rfd` with `gtk3`), plus the usual windowing libraries (Wayland/X11, xkbcommon) | none beyond the Windows SDK |
| GPU | Vulkan or OpenGL capable driver (iced renders through wgpu) | DirectX 12 / Vulkan capable driver |

Linux package examples: Fedora `sudo dnf install gcc pkgconf-pkg-config gtk3-devel libxkbcommon-devel wayland-devel`; Debian/Ubuntu `sudo apt install build-essential pkg-config libgtk-3-dev libxkbcommon-dev libwayland-dev`.

SQLite is compiled into the program (`rusqlite` with `bundled`); nothing else needs installing.

## Build, run and test

Run from the repository root. Add `-j 1` on a low-memory machine (the GUI dependency tree is large).

```bash
# compile check (fast sanity check)
cargo check -j 1

# debug build and run the application
cargo run --bin memonbooks

# optimised build; binary is target/release/memonbooks (memonbooks.exe on Windows)
cargo build --release
./target/release/memonbooks            # Linux
.\target\release\memonbooks.exe        # Windows (PowerShell)
```

> `cargo run` alone fails with *could not determine which binary to run*, because the package has four binaries; always pass `--bin memonbooks`.

Other binaries:

```bash
cargo run --bin seed_demo_data      # fill the configured database with a DEMO- company
cargo run --bin clear_demo_data     # remove it again
cargo run --bin native_ui_check     # render widgets against an in-memory database (visual check)
```

Tests (details and per-target commands in [docs/testing-qa.md](docs/testing-qa.md)):

```bash
cargo test --lib -j 1                      # unit tests
cargo test --test accounting -j 1          # one integration target (also banking, expenses, fixed_assets,
                                           # inventory, pos, purchases, reports, sales, tax, workflows)
cargo test --test style_consistency -j 1   # UI rules guard
cargo clippy -j 1 --all-targets
cargo fmt --check
```

## Configuration

There is no configuration file. Behaviour is controlled by:

| Setting | How |
|---------|-----|
| Environment | `MEMONBOOKS_ENV` = `development` / `dev`, `test`, `production` / `prod` (case-insensitive). Default: development for debug builds, production for release builds. Controls log verbosity only (Debug vs Info). |
| Theme | sidebar sun/moon button; first launch follows the OS theme; saved to `theme_preference.txt` |
| Font size | Settings → Font Size (Small, Medium, Large, Extra Large); saved to `font_scale_preference.txt` |
| Data directory | chosen by the OS conventions below; on Linux `XDG_DATA_HOME` moves it |

```bash
MEMONBOOKS_ENV=production ./target/release/memonbooks
XDG_DATA_HOME=/tmp/mb-scratch cargo run --bin memonbooks     # throw-away data (Linux)
```

## Database location and backup

SQLite file `memonbooks.sqlite3` in the per-user data directory:

| OS | Data directory |
|----|----------------|
| Linux | `~/.local/share/memonbooks/` |
| Windows | `%APPDATA%\MemonBooks\MemonBooks\data\` |
| macOS | `~/Library/Application Support/com.MemonBooks.MemonBooks/` |

Next to it: `logs/memonbooks.log`, `exports/` (CSV output), `uploads/` (attachments). The schema is created and upgraded automatically at start-up (29 migrations).

**Back up the database file and the `uploads/` folder** while the app is closed. The backup and restore code exists but has no button yet. See [docs/database.md](docs/database.md#8-backup-and-restore). The database is not encrypted.

## First run and usage

1. Start the app. On an empty database it creates the company *My Company* (USD), an open fiscal year, an *Administrator* role and the user `admin`.
2. The generated one-time `admin` password is written to `logs/memonbooks.log` (`Password: …`). Note that the app currently opens directly as the first active user; use **Switch User** to sign in as another user. The first login requires a new password (minimum 10 characters).
3. Use the sidebar: **Home**, **Maintain** (Chart of Accounts, Customers, Vendors, Bank Accounts, Warehouses), **New Journal Entry** / **Journal Entries**, **Trial Balance**, **Ledger Invoices / Bills**, **Products**, **Employees**, **Expenses**, **Point of Sale**, **Reports**, **Fixed Assets**, **Tax Rates**, **Settings**, **Companies**, **Users**.
4. Typical order: create accounts → customers/vendors → post an invoice or bill → record the payment → check Trial Balance and Reports → export CSV (written to `exports/`).
5. To explore with sample data: `cargo run --bin seed_demo_data`.

## Project layout

```
backend/     domain, services, repositories (SQL + migrations)
database/    SQLite connection, migration runner, CSV/XLSX/PDF import-export
configs/     paths, environment, preferences
frontend/    components/ (all styles + widgets), views/ (screens)
src/         main.rs (application), lib.rs, bin/ (demo and UI-check tools)
tests/       integration tests + style_consistency guard
assets/      bundled fonts (Pragati Narrow, Lucide)
packaging/   Flatpak, Windows (MSIX) and macOS manifests
docs/        documentation (below)
```

## Documentation

| Document | Content |
|----------|---------|
| [docs/prd.md](docs/prd.md) | Product requirements and feature status |
| [docs/design.md](docs/design.md) | UI standard, colours, type scale, layout, accessibility |
| [docs/architecture.md](docs/architecture.md) | Layers, start-up, data flow, module map |
| [docs/database.md](docs/database.md) | Schema, migrations, ER diagrams, backup |
| [docs/api.md](docs/api.md) | Internal service/repository API and file formats (there is no HTTP API) |
| [docs/security.md](docs/security.md) | Authentication, permissions, gaps |
| [docs/project-implementation.md](docs/project-implementation.md) | What is built, what remains |
| [docs/debugging.md](docs/debugging.md) | Logs, common errors, inspecting the database |
| [docs/testing-qa.md](docs/testing-qa.md) | Test targets, commands, manual QA checklist |
| [docs/feature-development.md](docs/feature-development.md) | How to add a feature |
| [PROJECT_SPEC.md](PROJECT_SPEC.md) | Frozen specification and development order |
| [CHANGELOG.md](CHANGELOG.md) | Behaviour changes |
| [docs/Complete_Project_User_Guide.md](docs/Complete_Project_User_Guide.md) | Guide to the earlier Python prototype whose features are being rebuilt |
| [docs/RUST_IMPLEMENTATION.md](docs/RUST_IMPLEMENTATION.md) | Older requirement checklist (partly outdated) |

## Known limitations

* No login enforcement at start-up, no encryption at rest, permissions mostly unenforced ([docs/security.md](docs/security.md)).
* The bundled text font has no Urdu/Arabic glyphs and no rupee sign; the UI is English only ([docs/design.md](docs/design.md#11-known-limitations)).
* No keyboard shortcuts.
* Windows and macOS builds and the store packages have not been verified.

Licence: not yet chosen (`license = "UNLICENSED"` in `Cargo.toml`; `LICENSE` is a TODO placeholder). Bundled fonts: Pragati Narrow (SIL OFL 1.1), Lucide (ISC) under `assets/fonts/`.
