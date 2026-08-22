# Changelog

All notable changes to this project are documented in this file.
Format loosely follows [Keep a Changelog](https://keepachangelog.com/).

## [Unreleased]

### Fixed
- `SalesRepository::record_customer_payment` and
  `PurchaseRepository::record_vendor_payment` now reject a payment
  greater than the customer's/vendor's total open balance, before
  posting anything. Previously the GL transaction always posted for
  the payment's full nominal amount (debit cash, credit AR/AP) while
  the receivables/payables subledger only ever allocated up to what
  was actually owed (`allocate_collection`/`allocate_payment` silently
  drop any leftover by design) -- an overpayment left the AR/AP
  control account and its subledger permanently out of agreement, with
  the excess amount untracked anywhere. Found while writing the
  overpayment integration test called for in the post-step-20
  self-audit; confirmed with `tests/sales/main.rs`'s
  `overpaying_a_receivable_is_rejected_and_posts_nothing`.
- Credit notes and debit notes now actually post. `create_credit_note`/
  `create_debit_note` only ever inserted the header/line rows --
  `CreditNote::build_transaction`/`DebitNote::build_transaction`
  existed but were never called by anything, so a "posted" credit or
  debit note had zero effect on the general ledger or the linked
  receivable's/payable's balance, despite both modules' doc comments
  saying otherwise. Adds `SalesRepository::post_credit_note` and
  `PurchaseRepository::post_debit_note`, which build and post the
  transaction, and -- mirroring the overpayment fix above -- reject a
  note whose total exceeds its linked receivable's/payable's remaining
  balance before posting anything. Also adds
  `AccountingRepository::get_receivable`/`get_payable`, needed to check
  that balance. Found while writing the "crediting a partially-paid
  invoice" test called for in the post-step-20 self-audit.

### Changed
- Views now route all data access through `backend/services/*.rs`
  instead of calling repositories directly, closing an Architecture
  Rule violation (Section 3: "Iced View -> Service -> Repository ->
  Database"). Added `backend/services/auth_service.rs`, which was not
  in `PROJECT_SPEC.md` Section 2's frozen `services/` file list --
  every other domain had one, but there was no dedicated auth/user
  service for `auth/login_view.rs` to route through. Wraps
  `UserRepository` plus `core::auth::authentication`'s login/2FA
  decisions; kept separate from `organization_service.rs` since
  authentication is its own concern, not organization data. Each
  converted view also gained direct tests of its `update()` logic
  (state transitions per `Message` variant), closing what the
  post-step-20 self-audit found to be the largest coverage gap in the
  codebase -- previously zero views had any test coverage at all.

### Added
- Full project scaffold: folder/file structure from `PROJECT_SPEC.md`
  Section 2, wired through `mod.rs`/`lib.rs`, White Theme tokens
  (`frontend/styles/colors.rs`, `typography.rs`).
- Cargo/Iced application foundation (step 1) and the configuration,
  error, logging, and validation infrastructure plus the SQLite
  database and migration runner (step 2).
- Companies, branches, departments, and currencies/exchange rates
  (step 3); users, roles, permissions, sessions, and 2FA (step 4).
- The accounting core: chart of accounts and fiscal periods (step 5);
  the transaction engine and journal entries, enforcing the hard
  invariant that every posted transaction's debits equal its credits
  (step 6); the general ledger and trial balance, derived from posted
  transactions rather than stored separately (step 7); opening
  balances and period closing, including closing-entry generation
  (step 8).
- Customers, vendors, and the AR/AP subledger with aging (step 9);
  sales and purchases documents -- quotations, invoices, credit notes,
  purchase orders, bills, debit notes, and customer/vendor payments,
  all posting through the transaction engine (step 10).
- Inventory: products, categories, warehouses, and an append-only
  stock movement ledger with atomic warehouse transfers (step 11).
- Banking: bank accounts, reconciliation, and cash transfers (step
  12); direct expenses and employee expense claims, and fixed assets
  with depreciation (straight-line and declining balance) and disposal
  gain/loss (step 13).
- Tax/VAT: rates, transactions, and period summaries (step 14).
- Reports (trial balance, general ledger, AR/AP, aging, balance sheet,
  profit & loss, cash flow, retained earnings, account statements,
  sales/purchases/inventory/expenses/fixed assets/tax registers) and
  CSV/PDF/XLSX export (step 15).
- Point of sale: terminals, sessions, cash drawer reconciliation, and
  split-tender sales (step 16).
- HR/payroll: employees, salary structures, attendance, leave, and
  payroll runs posting to the ledger (step 17).
- Notifications (email/SMS/WhatsApp channels with templates),
  backup/restore (SQLite `VACUUM INTO`), and HMAC-signed offline
  license activation with a 30-day trial (step 18).
- Iced views for every module above, with sidebar navigation, a
  two-factor login flow, and a first-run bootstrap that creates a
  default company and administrator account (step 19).
- Integration tests exercising each business domain end to end (not
  just each module's own unit tests), and cross-platform packaging
  manifests for the Microsoft Store (MSIX), Mac App Store (Info.plist
  + entitlements), and Linux (Flatpak manifest, desktop entry,
  AppStream metainfo) (step 20).

### Known gaps
- No onboarding UI yet for creating additional companies, changing the
  bootstrap-generated admin password, or switching users.
- Real email/SMS/WhatsApp delivery needs a configured provider;
  `Unconfigured*Sender` validates fully and fails with a clear error
  until one is wired in.
- Packaging manifests are written to each platform's real schema but
  unverified end to end -- no macOS, Windows SDK, or Flatpak build
  environment was available to actually produce and sign a package in
  this environment.
- Icon assets are a placeholder (`packaging/linux/icon.svg`); a
  designed icon set should replace it before shipping.
- `LICENSE` is a placeholder; a license still needs to be chosen.
