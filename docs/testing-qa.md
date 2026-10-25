# MemonBooks – Testing and QA

Related documents: [README](../README.md) · [Architecture](architecture.md) · [Design](design.md) · [Debugging](debugging.md) · [Feature development](feature-development.md)

## Table of contents

1. [Test strategy](#1-test-strategy)
2. [Test targets](#2-test-targets)
3. [Commands](#3-commands)
4. [What each suite covers](#4-what-each-suite-covers)
5. [The style-consistency guard](#5-the-style-consistency-guard)
6. [View-construction tests (iced)](#6-view-construction-tests-iced)
7. [Visual checks](#7-visual-checks)
8. [Manual QA checklist](#8-manual-qa-checklist)
9. [Quality gates before committing](#9-quality-gates-before-committing)
10. [Known gaps](#10-known-gaps)
11. [Open questions](#11-open-questions)

---

## 1. Test strategy

* Business rules are tested close to the code with **in-memory SQLite** (`Database::open_in_memory()`), so tests need no files and run in parallel.
* Unit tests live in `#[cfg(test)] mod tests` next to the code (about 850 `#[test]` functions across `backend/`, `frontend/`, `database/`, `configs/`, `src/`, counted by searching for the attribute).
* Integration tests live in `tests/` and use only the public crate API (`memonbooks::…`): services and repositories against an in-memory database.
* UI tests call each view's `update()` reducer with a `Ctx` pointing at a temporary directory; a few component tests build the widget tree (see section 6).
* A source-scanning test enforces the [UI rules](design.md#1-project-rules).
* There is no CI configuration in the repository; tests are run by hand.

## 2. Test targets

Defined in `Cargo.toml` (`[[test]]`) and auto-discovered (`style_consistency`):

| Target | File | Focus |
|--------|------|-------|
| `accounting` | `tests/accounting/main.rs` | chart of accounts, posting, trial balance, period rules |
| `banking` | `tests/banking/main.rs` | bank accounts, bank ledger |
| `expenses` | `tests/expenses/main.rs` | expenses and claims |
| `fixed_assets` | `tests/fixed_assets/main.rs` | depreciation |
| `inventory` | `tests/inventory/main.rs` | products, stock |
| `pos` | `tests/pos/main.rs` | POS sessions and sales |
| `purchases` | `tests/purchases/main.rs` | bills, payments, debit notes |
| `reports` | `tests/reports/main.rs` | report builders |
| `sales` | `tests/sales/main.rs` | invoices, payments, credit notes, overpayment rejection |
| `tax` | `tests/tax/main.rs` | tax rates |
| `workflows` | `tests/workflows.rs` | the 23 document kinds: save/edit/delete, conversion, stock effects, payroll, import |
| `style_consistency` | `tests/style_consistency/main.rs` | source scan for UI literals and structure rules |
| `lib` (`--lib`) | all in-crate unit tests | everything else |

Verify the list yourself with `cargo metadata --no-deps --format-version 1` (targets of kind `test`).

## 3. Commands

Run from the repository root. The full suite compiles the whole GUI stack and is memory hungry; on a small machine always use `-j 1` and run one target at a time.

```bash
cargo check -j 1                          # compile check only (fast sanity check)
cargo test --lib -j 1                     # unit tests of the crate
cargo test --test accounting -j 1         # one integration target
cargo test --test workflows -j 1
cargo test --test style_consistency -j 1  # UI rules guard
cargo test -j 1                           # everything (heavy)
cargo test -j 1 posts_a_balanced          # filter by test-name substring
cargo test -j 1 -- --nocapture            # show println!/log output
cargo clippy -j 1 --all-targets           # lints (clippy component required)
cargo fmt --check                         # formatting (rustfmt component required)
```

`cargo check -j 1` was run successfully against the current tree while writing this documentation (it finished in about 14 s with a warm cache). The test suite itself was **not** run for this documentation commit.

Tests that need a temp directory create one under `std::env::temp_dir()` and remove it at the end.

## 4. What each suite covers

| Area | Guarantees tested |
|------|-------------------|
| Ledger | unbalanced entries rejected; posting is atomic (rollback on failure); closed/locked period rejects postings; reversal links both transactions; trial balance totals equal |
| Sales/purchases | post invoice/bill creates receivable/payable; payments allocate FIFO; **overpayment rejected and nothing posts**; credit/debit notes post to the ledger |
| Inventory | signed stock movements; returns reverse stock in their location; document edits replace the previous movement |
| Workflows | all kinds round-trip through `DocumentService`; conversion chains (Quotation → Sale Order → Delivery Challan); import with reordered/case-insensitive headers; duplicate payroll protection |
| Auth | hash/verify, per-hash random salt, generic login errors, TOTP, session expiry, forced password change |
| Config | environment resolution, derived database/log paths |
| Database | migration runner: ordering, duplicate version rejection, version/name clash, rollback on bad SQL |
| Backup | back up a database with posted data, restore to a new path, compare trial balance and amounts |
| Fonts | embedded TTFs parse; font constants match the family names inside the files; Regular/Bold are real weights |
| Icons | every constant is a single code point present in the Lucide font |
| Colours/tables | light palette locked to original values; dark differs from light for each token; table/scrollbar light values locked; header corner-radius regression |
| Spacing | grid columns at 1920/1366/900/640; side-panel split behaviour |
| Typography | scale is strictly increasing; sizes never shrink as the window grows |
| Views | each view's `update()` flows (add/edit/delete, filters, export writes a file, attachments copy into `uploads/`) |

## 5. The style-consistency guard

`tests/style_consistency/main.rs` scans every `.rs` file under `frontend/views/` and `frontend/components/` (excluding the style modules themselves) and fails if it finds:

* raw `Color` construction (hex or channel values),
* raw `.size()` / `.text_size()` literals (also inside expressions),
* raw `.spacing()` / `.padding()` literals (bare `0` allowed),
* hand-built `Border { .. }`,
* `\u{…}` glyph escapes,
* a font family named outside `fonts.rs`,
* vendor column names, bank field names or warehouse field names written outside their single constant,
* vendors, bank accounts and warehouses not building their page with `panels::page_card`.

Allowed exceptions: `Color::WHITE`, `Color::BLACK`, `Color::TRANSPARENT`, `checkbox(..).size(16)`.

When this test fails, move the literal into `frontend/components/` as a named token or function; do not weaken the test.

## 6. View-construction tests (iced)

`cargo build`, `clippy` and `update()`-only tests do **not** run the widget constructors. iced contains `debug_assert!`s that fire only when a `view()` is built – for example `Scrollable` panics if its content is `Length::Fill` on the scrolling axis. Rule of thumb from past incidents (see also the project memory note): **whenever a change touches `Length`, sizing, scrollables or layout of a component, add a test that builds the `view()`**, as `data_table.rs` already does (`view_builds_without_panicking`, `view_with_frozen_header_builds_without_panicking`, …). Finally run the app once (`cargo run --bin memonbooks`) and open the changed screen at narrow, medium and wide widths.

## 7. Visual checks

* `cargo run --bin native_ui_check` renders real widgets in a native window against an in-memory database and never touches a user database (see `src/bin/native_ui_check.rs`).
* `docs/design-reference/` and `docs/screenshots/` hold reference images; they are inputs for comparison, not generated output.
* Check every screen in **both themes**, at the three layout classes (widths below 800, 800–1199, 1200 and above, plus 1700+), and with Font Size Small and Extra Large.

## 8. Manual QA checklist

Use a scratch database (see [debugging.md](debugging.md#6-using-a-throw-away-data-directory)) or `seed_demo_data`.

**Start and login**
- [ ] First run creates the company, fiscal period and the admin; admin password is in the log.
- [ ] Switch User → login with wrong password shows the generic message; correct password signs in.
- [ ] A user with `must_change_password` must change it before anything else is shown.

**Accounting**
- [ ] Create accounts (parent/child); duplicate code is rejected.
- [ ] Post a balanced journal entry; an unbalanced one is rejected; a date outside any open period is rejected.
- [ ] Reverse an entry; trial balance still balances.

**Maintain (Vendors, Customers, Bank Accounts, Warehouses)**
- [ ] Add, edit, search, filter by status, export CSV (file appears in `exports/`).
- [ ] Vendor attachment: add, save, reopen, remove; the file appears in `uploads/`.
- [ ] Card list scrolls whole rows only; no card is cut at top or bottom; resize through the breakpoints (3 → 2 → 1 columns; side panel replaces the list when narrow).
- [ ] Warehouse layout: set lines × positions, place stock in a bin, over-capacity handling.
- [ ] Deleting a record in use shows a friendly message (or archives, for bank accounts).

**Sales / purchases / POS / HR / expenses / assets / tax**
- [ ] Invoice → post → receive payment (partial, full, over-payment rejected) → void/credit note.
- [ ] Bill → post → pay; debit note.
- [ ] POS: open session, sell with two payment methods.
- [ ] Employee → payroll run → ledger entry.
- [ ] Expense claim: approve → reimburse.
- [ ] Fixed asset depreciation posts once per call.

**Reports**
- [ ] Balance sheet and P&L load, balance, and export CSV.

**UI**
- [ ] Light/dark toggle persists across restart; Font Size persists.
- [ ] No raw text overflow at Extra Large font on a 1024 px-wide window.

## 9. Quality gates before committing

1. `cargo check -j 1` and `cargo clippy -j 1 --all-targets` clean (no `#[allow]` added to silence a lint).
2. `cargo fmt --check` clean.
3. Relevant `cargo test --test <target> -j 1` and `cargo test --test style_consistency -j 1` pass.
4. For UI changes: the screen was opened and checked as in section 7.
5. `CHANGELOG.md` updated for user-visible or accounting-behaviour changes.

## 10. Known gaps

* No CI, no coverage measurement, no fuzzing, no performance or large-data tests.
* No end-to-end GUI automation; native file dialogs (`rfd`) are not exercised by tests.
* Import/XLSX/PDF export have library tests but no screen to test.
* Keyboard behaviour and accessibility are untested ([design.md](design.md#10-keyboard-support)).
* Windows and macOS were not tested for this documentation.

## 11. Open questions

1. Which CI system (if any) should run `check`, `clippy`, `fmt` and the test targets?
2. Should view-construction tests be required for every view, not only components?
3. Target coverage and the acceptance test list for a first release?
4. How is the Flatpak/MSIX/App Store build to be tested?
