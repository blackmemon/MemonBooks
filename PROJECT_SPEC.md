# MemomBooks — Frozen Project Specification (v1.0)

**Status:** FROZEN — do not restructure folders without explicit approval
**Stack:** Rust + Iced (desktop only)
**Last updated:** 2026-08-21

> **Note on naming:** kept as "MemomBooks" exactly as provided. If this is meant to be "MemonBooks", say so and it should be renamed consistently across `Cargo.toml`, `README.md`, package identifiers, and packaging manifests.

---

## 0. Instructions for the Coding Agent (Claude Code)

You are implementing this project end-to-end. Treat this document as the single source of truth.

1. **Do not re-architect the folder structure.** It is frozen (Section 2). If a file is genuinely missing for something you're implementing, add it in the most consistent existing location and note it — don't invent new top-level modules.
2. **Scaffold first.** Create every directory and file listed in Section 2 as empty stub modules (with `// TODO` and correct `pub mod` wiring through `mod.rs`/`lib.rs`) before writing real logic.
3. **Follow the Development Order in Section 5 sequentially.** Do not jump ahead to POS or HR before the accounting core (chart of accounts → journal entries → general ledger → trial balance) is working and tested.
4. **Money is never `f32`/`f64`.** Use a fixed-point/decimal-safe type for every monetary field (e.g. the `rust_decimal` crate — this is a suggestion, not a mandate; pick one and use it everywhere consistently).
5. **Apply the White Theme (Section 6) from the start.** Wire the tokens into `frontend/styles/colors.rs` and `frontend/styles/typography.rs` during foundation setup (Development Order step 1) — no hardcoded hex colors anywhere else in the frontend.
6. **Every posted journal entry must satisfy Total Debits = Total Credits** — enforce this at the transaction engine level, not just in the UI.
7. **Write tests as you go**, under `tests/`, matching the module you just built — not as a final pass at the end.
8. **Run `cargo build` and `cargo test` after each completed phase** before moving to the next.
9. **Keep packaging code (`packaging/`) free of business logic.**
10. Ask for clarification only if something is genuinely ambiguous or contradictory — not for style preferences already decided in this doc.

---

## 1. Technology Constraints

- Rust only.
- Iced only for the desktop UI.
- Targets: macOS App Store, Microsoft Store (Windows), and Linux Flatpak.
- No Electron, Tauri, Flutter, React, web frontend, or browser runtime.
- Business logic runs inside the Rust desktop application.
- Double-entry accounting is mandatory.
- Use fixed-point/decimal-safe money handling; never binary floating point for monetary values.

---

## 2. Final Frozen Project Structure

```
MemomBooks/
├── Cargo.toml
├── Cargo.lock
├── README.md
├── LICENSE
├── CHANGELOG.md
├── Project_Structure.txt
├── rust-toolchain.toml
├── .gitignore
│
├── src/
│   ├── main.rs
│   └── lib.rs
│
├── backend/
│   ├── accounting/
│   │   ├── accounting_rules.rs
│   │   ├── chart_of_accounts.rs
│   │   ├── fiscal_periods.rs
│   │   ├── general_ledger.rs
│   │   ├── journal_entries.rs
│   │   ├── opening_balances.rs
│   │   ├── period_closing.rs
│   │   ├── trial_balance.rs
│   │   ├── accounts_payable/
│   │   │   ├── aging.rs
│   │   │   ├── payables.rs
│   │   │   └── payments.rs
│   │   └── accounts_receivable/
│   │       ├── aging.rs
│   │       ├── collections.rs
│   │       └── receivables.rs
│   │
│   ├── audit/
│   │   ├── activity_logs.rs
│   │   ├── audit_logs.rs
│   │   └── change_history.rs
│   │
│   ├── backup/
│   │   ├── backup.rs
│   │   ├── backup_schedule.rs
│   │   └── restore.rs
│   │
│   ├── banking/
│   │   ├── bank_accounts.rs
│   │   ├── bank_reconciliation.rs
│   │   ├── bank_transactions.rs
│   │   └── cash_management.rs
│   │
│   ├── expenses/
│   │   ├── expenses.rs
│   │   ├── expense_categories.rs
│   │   └── expense_claims.rs
│   │
│   ├── fixed_assets/
│   │   ├── assets.rs
│   │   ├── asset_categories.rs
│   │   ├── depreciation.rs
│   │   ├── disposals.rs
│   │   └── transfers.rs
│   │
│   ├── inventory/
│   │   ├── adjustments.rs
│   │   ├── categories.rs
│   │   ├── products.rs
│   │   ├── stock.rs
│   │   ├── stock_movements.rs
│   │   ├── transfers.rs
│   │   └── warehouses.rs
│   │
│   ├── purchases/
│   │   ├── bills.rs
│   │   ├── debit_notes.rs
│   │   ├── payments.rs
│   │   └── purchase_orders.rs
│   │
│   ├── reports/
│   │   ├── accounts_payable.rs
│   │   ├── accounts_receivable.rs
│   │   ├── account_statement.rs
│   │   ├── aging_summary.rs
│   │   ├── balance_sheet.rs
│   │   ├── banking.rs
│   │   ├── cash_book.rs
│   │   ├── cash_flow.rs
│   │   ├── expenses.rs
│   │   ├── fixed_assets.rs
│   │   ├── general_ledger.rs
│   │   ├── inventory.rs
│   │   ├── profit_loss.rs
│   │   ├── purchases.rs
│   │   ├── retained_earnings.rs
│   │   ├── sales.rs
│   │   ├── tax.rs
│   │   ├── trial_balance.rs
│   │   └── vat_summary.rs
│   │
│   ├── sales/
│   │   ├── credit_notes.rs
│   │   ├── invoices.rs
│   │   ├── payments.rs
│   │   └── quotations.rs
│   │
│   ├── tax/
│   │   ├── tax_rates.rs
│   │   ├── tax_reports.rs
│   │   ├── tax_transactions.rs
│   │   └── vat.rs
│   │
│   ├── core/
│   │   ├── errors.rs
│   │   ├── logging.rs
│   │   ├── permissions.rs
│   │   ├── roles.rs
│   │   ├── telemetry.rs
│   │   ├── users.rs
│   │   ├── validation.rs
│   │   └── auth/
│   │       ├── activation.rs
│   │       ├── authentication.rs
│   │       ├── licensing.rs
│   │       ├── passwords.rs
│   │       ├── sessions.rs
│   │       └── two_factor.rs
│   │
│   ├── currency/
│   │   ├── currencies.rs
│   │   ├── currency_conversions.rs
│   │   └── exchange_rates.rs
│   │
│   ├── hr/
│   │   ├── attendance.rs
│   │   ├── employees.rs
│   │   ├── leave.rs
│   │   ├── payroll.rs
│   │   └── salary.rs
│   │
│   ├── notifications/
│   │   ├── email.rs
│   │   ├── notifications.rs
│   │   ├── reminders.rs
│   │   ├── sms.rs
│   │   ├── templates.rs
│   │   └── whatsapp.rs
│   │
│   ├── organization/
│   │   ├── branches.rs
│   │   ├── companies.rs
│   │   └── departments.rs
│   │
│   ├── parties/
│   │   ├── addresses.rs
│   │   ├── contacts.rs
│   │   ├── customers.rs
│   │   └── vendors.rs
│   │
│   ├── pos/
│   │   ├── barcode.rs
│   │   ├── cash_drawer.rs
│   │   ├── payments.rs
│   │   ├── receipts.rs
│   │   ├── sales.rs
│   │   ├── sessions.rs
│   │   └── terminals.rs
│   │
│   ├── repositories/
│   │   ├── accounting_repository.rs
│   │   ├── banking_repository.rs
│   │   ├── expense_repository.rs
│   │   ├── fixed_asset_repository.rs
│   │   ├── inventory_repository.rs
│   │   ├── organization_repository.rs
│   │   ├── party_repository.rs
│   │   ├── purchase_repository.rs
│   │   ├── sales_repository.rs
│   │   ├── tax_repository.rs
│   │   └── user_repository.rs
│   │
│   ├── services/
│   │   ├── accounting_service.rs
│   │   ├── api.rs
│   │   ├── banking_service.rs
│   │   ├── commands.rs
│   │   ├── expense_service.rs
│   │   ├── fixed_asset_service.rs
│   │   ├── inventory_service.rs
│   │   ├── organization_service.rs
│   │   ├── party_service.rs
│   │   ├── purchase_service.rs
│   │   ├── report_service.rs
│   │   ├── sales_service.rs
│   │   ├── tax_service.rs
│   │   └── transaction_service.rs
│   │
│   ├── settings/
│   │   ├── accounting_settings.rs
│   │   ├── company_settings.rs
│   │   ├── invoice_settings.rs
│   │   ├── localization.rs
│   │   ├── numbering.rs
│   │   ├── sync.rs
│   │   └── tax_settings.rs
│   │
│   └── transactions/
│       ├── transaction.rs
│       ├── transaction_engine.rs
│       ├── transaction_lines.rs
│       ├── transaction_status.rs
│       └── transaction_types.rs
│
├── configs/
│   ├── app.rs
│   └── environment.rs
│
├── database/
│   ├── database.rs
│   ├── exports/
│   │   ├── csv.rs
│   │   ├── pdf.rs
│   │   └── xlsx.rs
│   ├── migrations/
│   └── seeds/
│
├── frontend/
│   ├── assets/
│   ├── components/
│   │   └── common/
│   │       ├── button.rs
│   │       ├── calendar.rs
│   │       ├── confirmation.rs
│   │       ├── data_table.rs
│   │       ├── dropdown.rs
│   │       ├── input_field.rs
│   │       ├── loading.rs
│   │       ├── modal.rs
│   │       ├── notification.rs
│   │       ├── pagination.rs
│   │       └── search.rs
│   │
│   ├── i18n/
│   │   ├── ar.rs
│   │   ├── en.rs
│   │   └── ur.rs
│   │
│   ├── resources/
│   │   ├── calendar.svg
│   │   └── close.svg
│   │
│   ├── styles/
│   │   ├── application.rs
│   │   ├── badges.rs
│   │   ├── borders.rs
│   │   ├── button.rs
│   │   ├── calendar.rs
│   │   ├── checkbox.rs
│   │   ├── colors.rs
│   │   ├── combobox.rs
│   │   ├── datepicker.rs
│   │   ├── dialogs.rs
│   │   ├── dropdown.rs
│   │   ├── fonts.rs
│   │   ├── forms.rs
│   │   ├── frames.rs
│   │   ├── headers.rs
│   │   ├── icons.rs
│   │   ├── inputs.rs
│   │   ├── labels.rs
│   │   ├── list.rs
│   │   ├── menu.rs
│   │   ├── modal.rs
│   │   ├── panels.rs
│   │   ├── radiobutton.rs
│   │   ├── scrollbar.rs
│   │   ├── separators.rs
│   │   ├── shadows.rs
│   │   ├── spacing.rs
│   │   ├── states.rs
│   │   ├── statusbar.rs
│   │   ├── table.rs
│   │   ├── tabs.rs
│   │   ├── textbox.rs
│   │   ├── theme.rs
│   │   ├── toolbar.rs
│   │   ├── tooltip.rs
│   │   ├── tree.rs
│   │   └── typography.rs
│   │
│   ├── uploads/
│   └── views/
│       ├── accounting/
│       │   ├── chart_of_accounts_view.rs
│       │   ├── journal_entry_view.rs
│       │   └── trial_balance_view.rs
│       ├── auth/
│       ├── dashboard/
│       ├── hr/
│       ├── inventory/
│       ├── pos/
│       ├── reports/
│       ├── sales/
│       └── settings/
│
├── tests/
│   ├── accounting/
│   ├── banking/
│   ├── expenses/
│   ├── fixed_assets/
│   ├── inventory/
│   ├── pos/
│   ├── purchases/
│   ├── reports/
│   ├── sales/
│   └── tax/
│
└── packaging/
    ├── macos/
    ├── windows/
    └── linux/
        └── flatpak/
```

### Cleanup applied in this revision
1. **`backend/accountings/accounting/...`** (double-nested, plural outer / singular inner) → flattened to a single **`backend/accounting/`**.
2. **`transaction_types.rs`** was missing → added under `backend/transactions/`.
3. `calendar.rs` naming confirmed consistent (no `calender.rs` typo present).
4. `departments.rs` confirmed under `organization/` only (not duplicated in `hr/`).
5. Removed a duplicate `src/` listing that appeared twice in the source document — now listed once, at the top level.

**Structure is frozen as of this revision.** Any further changes should go through an explicit update to this document, not ad-hoc edits during coding.

---

## 3. Architecture Rules

- Iced View → Service → Repository → Database.
- Frontend must never directly manipulate database tables.
- Sales, purchases, POS, expenses, banking, inventory, and payroll must post accounting effects through the transaction engine.
- Every posted journal entry must satisfy Total Debits = Total Credits.
- Posted accounting entries are immutable; corrections use reversals/corrective transactions.
- Fiscal periods support open and closed/locked states with permission checks.
- Audit records cover posting, reversal, security events, configuration changes, and other important actions.
- Financial reports derive from ledger data instead of storing duplicated totals.
- Multi-table financial posting must be atomic: all changes commit or none commit.
- Keep platform-specific packaging code isolated under `packaging/`.

---

## 4. Core Accounting Flow

```
Business Event → Transaction Service → Transaction Engine → Journal Entry → General Ledger → Trial Balance → Financial Reports
```

**Hard invariant:** Total Debits = Total Credits for every posted journal entry.

---

## 5. Development Order

1. Cargo/Iced application foundation — **including White Theme setup (Section 6): `styles/colors.rs`, `styles/fonts.rs`, `styles/typography.rs` wired before any view is built.**
2. Configuration, errors, logging, validation, and database.
3. Companies, branches, departments, currencies.
4. Users, authentication, roles, permissions, sessions, 2FA.
5. Chart of accounts and fiscal periods.
6. Journal entries and transaction engine.
7. General ledger and trial balance.
8. Opening balances and period closing.
9. Customers/vendors and AR/AP.
10. Sales and purchases.
11. Inventory and stock movements.
12. Banking and reconciliation.
13. Expenses and fixed assets.
14. Tax/VAT.
15. Reports and CSV/PDF/XLSX exports.
16. POS.
17. HR/payroll.
18. Notifications, backup/restore, licensing and activation.
19. Iced views/dashboards module by module.
20. Automated tests and cross-platform packaging.

---

## 6. UI Design System — White Theme

Single theme for now (light/white). Token naming uses a `white_` prefix so a future `dark_` set can be added later without renaming anything.

### Typography
- **Font:** system default (no bundled/custom font — use the OS-provided UI font stack).
- **Base size:** 12.

### Color Tokens

| Token | Hex | Role |
|---|---|---|
| `white_primary` | `#51A2FF` | Primary accent — buttons, active nav item, links, focus rings |
| `white_positive_bg` | `#C6F1CE` | Success / positive background (badges, toasts, confirmations) |
| `white_positive_front` | `#006400` | Success / positive text & icon color |
| `white_negative_bg` | `#FFC8CE` | Error / negative background (validation errors, destructive confirmations) |
| `white_negative_front` | `#9C000A` | Error / negative text & icon color |
| `white_warning_bg` | `#FFEBA0` | Warning background (pending states, unsaved-changes notices) |
| `white_warning_front` | `#9C5A00` | Warning text & icon color |

> **Assumption:** the last two hex values (`#FFEBA0`, `#9C5A00`) weren't explicitly labeled in the source — mapped as a warning background/front pair following the same bg/front convention as positive and negative. Correct this table if that assumption is wrong.

### Suggested implementation (`frontend/styles/colors.rs`)

```rust
use iced::Color;

// Typography
pub const BASE_FONT_SIZE: f32 = 12.0;

// Primary
pub const WHITE_PRIMARY: Color = Color::from_rgb8(0x51, 0xA2, 0xFF);

// Positive / success
pub const WHITE_POSITIVE_BG: Color = Color::from_rgb8(0xC6, 0xF1, 0xCE);
pub const WHITE_POSITIVE_FRONT: Color = Color::from_rgb8(0x00, 0x64, 0x00);

// Negative / error
pub const WHITE_NEGATIVE_BG: Color = Color::from_rgb8(0xFF, 0xC8, 0xCE);
pub const WHITE_NEGATIVE_FRONT: Color = Color::from_rgb8(0x9C, 0x00, 0x0A);

// Warning
pub const WHITE_WARNING_BG: Color = Color::from_rgb8(0xFF, 0xEB, 0xA0);
pub const WHITE_WARNING_FRONT: Color = Color::from_rgb8(0x9C, 0x5A, 0x00);
```

Adjust the exact API (`Color::from_rgb8` etc.) to match whichever Iced version is pinned in `Cargo.toml`.

**Usage rule:** no hex literals anywhere outside `styles/colors.rs`. Every component and view imports these constants.

---

## 7. Platform Packaging

- **macOS:** native `.app` and Mac App Store packaging/signing configuration.
- **Windows:** MSIX packaging for Microsoft Store.
- **Linux:** Flatpak manifest, desktop entry, icons, permissions and metadata.
- No business logic inside packaging scripts.

---

## 8. Foundation Definition of Done

- App starts on macOS, Windows and Linux.
- Iced UI runs without a web runtime.
- Fresh database can be created through migrations.
- Company and chart of accounts can be created/seeded.
- Fiscal period can be opened and closed.
- Balanced journal can be posted.
- Unbalanced journal is rejected.
- Posted entries appear in General Ledger and Trial Balance.
- Reports derive from ledger data.
- Important accounting/security actions are audited.
- Accounting invariants have automated tests.
- White Theme tokens render consistently across all built views (no hardcoded colors).
