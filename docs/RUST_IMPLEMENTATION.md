# MemonBooks Rust + Iced implementation

This desktop project implements the feature and workflow inventory in [Complete_Project_User_Guide.md](Complete_Project_User_Guide.md) under the Rust-only, native Iced constraints in [`../PROJECT_SPEC.md`](../PROJECT_SPEC.md). The supplied Markdown, PDF, DOCX guide and every supplied design-reference image are retained in this repository. No browser runtime or web frontend is used. Workflow and accounting data are stored in SQLite; user-selected documents and reports use native desktop file dialogs.

## Requirement checklist

Status means: **Complete** = the described user flow is implemented in the current code and has code/test or render evidence; **Partial** = a usable implementation exists with an identified behavior or verification gap; **Missing** = there is no corresponding implementation. The guide documents both working behavior and known gaps in its source application, so the checklist records documented behavior as well as guide features.

| Guide requirement | Status | Verification evidence |
|---|---|---|
| Native app shell, Home, navigation and visual theme | Partial | `src/main.rs`, `frontend/components/common/sidebar.rs`, `frontend/styles/`; native Iced renderer produced 48 screen captures (see test results). Actual XWayland app opened. Shell screenshot exposed fixed-nav overlap; moved both nav groups into a shared scroll area. Input-driven confirmation of this fix is unverified. |
| Home totals, recent expenses and quick actions | Complete | `frontend/views/dashboard/dashboard_view.rs`; rendered by native UI harness; dashboard state comes from services. Quick access routes to expense/trip/receipt/report workflows. |
| Expenses and Expense Claims | Complete | `Kind::Expense` schema and claims route provide searchable records, save/validation and report fields; document renderer and workflow tests. |
| Trip list and trip create/edit workflow | Complete | `Kind::Trip` schema in `backend/transactions/documents.rs`; `documents_view.rs`; flight/status roundtrip, validation, date-range checks and on-disk reopen tests in `tests/workflows.rs`. |
| Purchase Order | Complete | Shared schema-driven editor/list in `documents.rs` and `documents_view.rs`; UI harness rendered list/form; document CRUD/conversion/export covered by service/view tests. |
| Purchase Invoice | Complete | Same shared editor, decimal totals and posting path; atomic posting and stock/accounting workflow tests. |
| Purchase Return | Partial | Form/list and conversion from invoice exist; source guide notes stock is not reversed by a return. |
| Goods Receipt | Partial | Form/list and receipt stock movement exist; edits and return/reversal cases have limits covered in the guide. |
| Cash Receipt | Complete | Receipt schema, native account selectors, posting and FIFO receivable allocation; posting and payment tests. |
| Bank Receipt | Complete | Bank receipt schema, payment-mode controls, posting and FIFO allocation; transaction posting tests. |
| Cash Payment | Complete | Payment schema, WHT/account posting and payable allocation; posting tests. |
| Bank Payment | Complete | Bank payment schema, cheque/mode controls, WHT and payable allocation; posting tests. |
| Journal Voucher | Complete | Decimal/balanced journal validation is enforced below the UI; unbalanced journal rejection test and posting tests. |
| Quotation | Complete | Schema-driven quote editor/list and conversion to Sale Order; native renderer and workflow conversion tests. |
| Sale Order | Complete | Order editor/list and conversion to Delivery Challan; native renderer and workflow conversion tests. |
| Delivery Challan | Partial | Editor/list and delivery stock movement exist; edit-time stock replacement and some return/reversal paths remain limited. |
| Sale Invoice | Partial | Invoice editor, posting, AR integration, and stock handling exist; invoice tax liability and linked document edge cases remain limited. |
| Sale Return | Partial | Return editor and conversion from invoice exist; returns do not adjust stock. |
| Chart of Accounts | Partial | Company-scoped account CRUD/list, validation, parent-cycle rejection, GL and trial balance integration. A dedicated account edit flow and complete cycle-safe hierarchy UX are not available. |
| Vendors | Complete | Existing party tables plus synchronized extended profiles; search/status, edit/versioning, import/export and attachment UI paths. Import/export backend and view tests exist; OS dialog interaction is unverified. |
| Items | Partial | Item maintenance, stock quantities, CSV/XLSX import/export, attachments and stock services. Transaction return/edit stock edge cases remain. |
| Customers | Partial | Customer master CRUD/extended profile, selectors, import/export and attachments exist. Customer address autofill has a field-name mismatch in some document flows. |
| Inventory Locations | Complete | Location maintenance, selectors, CSV/XLSX import/export, stock views and movement services; exercised in inventory and workflow tests. |
| Shared Maintain search, filters, sort, columns, paging, bulk/archive/import/export/attachments | Partial | Implemented in `documents_view.rs` and import/export services; view/backend tests and 48 rendered screens. File-picker selection and dialog-driven end-to-end import/export remain unverified on this desktop session. |
| Employees | Partial | Employee/profile maintenance and reports exist. User guide notes custom-field persistence and some status/edit behavior limitations. |
| Payroll runs | Partial | Payroll generation, calculations, draft/paid transitions, reports and posting. Workflow tests cover duplicate handling, calculations and paid ledger posting. Role approvals and report-filter selection limitations remain. |
| General Ledger | Complete | Native `ledger_view.rs`, ledger repository and ledger report tests; native harness rendered GL screen. |
| Trial Balance | Complete | Native report view and ledger-derived repository; accounting tests verify balanced totals. |
| AR/AP aging and statements | Complete | Native report view and FIFO subledger services; receivable allocation tests and accounting report tests. |
| Posting, fiscal-period checks, balanced entries and atomic writes | Complete | `backend/repositories/accounting_repository.rs`, transaction engine and purchase/sales/document services; tests reject unbalanced entries and verify atomic posting/rollback. |
| Settings, user/company administration, support and external integrations | Partial | Native settings/company/user routes exist, but settings and support integrations remain limited; no email, bank API, payment gateway or synchronization backend is configured. Authentication/role enforcement is not yet end-to-end. |
| Notifications, exports, templates, print/PDF and attachments | Partial | Native message/status feedback, CSV/XLSX/PDF output, templates, print and local attachment storage exist in code. Native OS file-dialog selection, successful dialog-driven export/import, and attachment roundtrip were not driven end-to-end here. |

## Design-reference review

Every image in `docs/` and its subdirectories supplied for this project was reviewed. The accounting screenshots (`dashboard.png`, `general-ledger.png`, `maintain-vendors.png`, `payroll.png`, `purchase-invoice-form.png`, plus six timestamped screenshots) informed the light accounting theme, compact document toolbar, master-data grids, form sections and report layouts. The Koru healthcare screenshots (`A3`, `A4`, `A5`, `B1`, `B1 (1)`, `C1`, `D1`) informed dense searchable tables, visible filters, clear grouped controls, restrained cards, and a side-detail pattern where appropriate; healthcare terminology/content was not copied into the accounting product. All 18 reference images are versioned under `docs/design-reference/`.

The supplied requirements guide is versioned as Markdown, PDF and DOCX under `docs/`. Its Python/PyQt6 launch/storage/UI references are implemented with Rust, Iced, SQLite, and native dialogs in accordance with the project specification.

## Setup and usage

Install a Rust toolchain and the host libraries needed by Iced/winit and `rfd` (GTK3 on Linux). From the project root:

```sh
cargo check
cargo run
```

Cargo can use cached dependencies with `--offline`. On first launch the app creates its local database, a company, fiscal period and initial administrator. Follow the startup log guidance to obtain the generated initial credential and change it. Use the left navigation to open Home, Transactions, Accounting, Trips, Payroll, Expense Claims, or Maintain. Choose a document/master type, select Add, fill the form and Save. To post accounting documents, create the required accounts first and choose Posted; failed posting reports a warning and preserves the prior/draft status. Use row selection and the page actions for bulk maintenance, imports, exports, PDF, templates and attachments. Payroll can be generated for a month and active employees, then reviewed and marked Paid to post the payroll entry.

## Test and native UI results

The prior commit included 744 library tests and 34 integration tests (778 passing), plus `src/bin/native_ui_check.rs`, which renders 48 native Iced screens. This review added an on-disk persistence regression test and fixed the sidebar overflow found in the actual app shell.

Current review checks:

| Check | Result |
|---|---|
| `cargo test --offline --test workflows` | Passed: 19 tests, including close database / reopen / verify the exact saved trip row. |
| Full `cargo test --offline` | Passed: 779 tests (744 library tests and 35 integration tests), no failures. |
| `cargo check --offline` | Passed. |
| `cargo fmt --check` | Fails on extensive pre-existing formatting differences across unrelated files. `rustfmt --check tests/workflows.rs` passed; the sidebar edit follows the surrounding file's existing formatting, and unrelated files are intentionally left unchanged. |
| `cargo clippy --offline` | Passed with 10 existing warnings (9 library, 1 `native_ui_check`): possible missing else, module inception, empty doc-comment spacing, needless update, single match, and manual `is_multiple_of`. |
| `git diff --check` | Passed. |
| Native UI render and screen inspection | Passed for 48 Iced screens at 1440×935 via native renderer harness. The actual app launched in an isolated XDG data directory and its home shell was visually inspected at 1330×887. The navigation overlap was fixed; a post-fix screenshot confirms both route groups now scroll together without covering each other. |
| Mouse/keyboard end-to-end workflows | Unverified. A temporary Rust X11/XTest helper successfully captured the running window, but synthesized events did not change the UI, so no create/edit/post/import/export/attachment action is claimed as performed through desktop input. |
| Native file dialogs | Unverified. GTK3 dialogs were not navigated/selected in this session. |
| Process restart persistence | Verified by Rust test that closes the SQLite connection and reopens the same on-disk database; an app-level quit/relaunch with a UI-created record was not verified because native input did not work. |

No browser test applies to this native desktop application. The supplied image captures are design references rather than evidence of every interactive state.

## Remaining limitations

- No end-to-end mouse/keyboard desktop driver succeeded in this environment, so native form validation, save/edit, posting, import/export, attachment selection, and file dialogs were not confirmed through actual user input. The same workflows have service/view tests where noted, but those do not replace desktop interaction checks.
- OS-level file picker behavior varies by platform and was not checked here.
- Roles/permissions are not fully enforced through every workflow; settings, support, outbound integrations and notification delivery are limited.
- Account hierarchy editing, customer address autofill, some return-stock effects, and stock recalculation after edits retain the specific gaps in the checklist.
- Platform packaging/signing, cross-platform accessibility and OS theme switching remain outside this verification.
