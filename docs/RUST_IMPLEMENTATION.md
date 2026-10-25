# MemonBooks Rust + Iced implementation

> **Notice (documentation review, 2026-10):** this file is a point-in-time checklist from an earlier review and is partly outdated. Since then the UI styles moved from `frontend/styles/` to `frontend/components/` (the sidebar is `frontend/components/sidebar.rs`), and the generic `documents_view.rs` editor referenced below no longer exists in `frontend/views/`; the typed Maintain, invoice and bill screens replaced it, while `DocumentService` and the `workflow_*` tables remain in the backend. The test counts and `cargo run` command below are historical – use `cargo run --bin memonbooks`. For current status see [project-implementation.md](project-implementation.md) and [prd.md](prd.md).

This desktop project implements the feature and workflow inventory in [Complete_Project_User_Guide.md](Complete_Project_User_Guide.md) under the Rust-only, native Iced constraints in [`../PROJECT_SPEC.md`](../PROJECT_SPEC.md). The supplied Markdown, PDF, DOCX guide and every supplied design-reference image are retained in this repository. No browser runtime or web frontend is used. Workflow and accounting data are stored in SQLite; user-selected documents and reports use native desktop file dialogs.

## Requirement checklist

Status means: **Complete** = the described user flow is implemented in the current code and has code/test or render evidence; **Partial** = a usable implementation exists with an identified behavior or verification gap; **Missing** = there is no corresponding implementation. The guide documents both working behavior and known gaps in its source application, so the checklist records documented behavior as well as guide features.

| Guide requirement | Status | Verification evidence |
|---|---|---|
| Native app shell, Home, navigation and visual theme | Partial | `src/main.rs`, `frontend/components/common/sidebar.rs`, `frontend/styles/`; native Iced harness renders 48 screens. The live app is bound to Wayland (`wl_seat` observed). The X11 window found by the test helper is stale/iconified XWayland (`WM_STATE=Iconic`), so its screenshots/events do not verify the live surface. Navigation click behavior is unverified. |
| Home totals, recent expenses and quick actions | Complete | `frontend/views/dashboard/dashboard_view.rs`; rendered by native UI harness; dashboard state comes from services. Quick access routes to expense/trip/receipt/report workflows. |
| Expenses and Expense Claims | Complete | `Kind::Expense` schema and claims route provide searchable records, save/validation and report fields; document renderer and workflow tests. |
| Trip list and trip create/edit workflow | Complete | `Kind::Trip` schema in `backend/transactions/documents.rs`; `documents_view.rs`; flight/status roundtrip, validation, date-range checks and on-disk reopen tests in `tests/workflows.rs`. |
| Purchase Order | Complete | Shared schema-driven editor/list in `documents.rs` and `documents_view.rs`; UI harness rendered list/form; document CRUD/conversion/export covered by service/view tests. |
| Purchase Invoice | Complete | Same shared editor, decimal totals and posting path; atomic posting and stock/accounting workflow tests. |
| Purchase Return | Complete | Posted purchase returns create an inbound-reversal stock movement; `tests/workflows.rs::posted_purchase_and_sale_returns_reverse_stock_in_their_location` asserts signed quantity and on-hand result. |
| Goods Receipt | Complete | Receipt quantities update the selected location; generic stock replacement removes the prior movement before applying edited document effects in `backend/repositories/document_repository.rs`. |
| Cash Receipt | Complete | Receipt schema, native account selectors, posting and FIFO receivable allocation; posting and payment tests. |
| Bank Receipt | Complete | Bank receipt schema, payment-mode controls, posting and FIFO allocation; transaction posting tests. |
| Cash Payment | Complete | Payment schema, WHT/account posting and payable allocation; posting tests. |
| Bank Payment | Complete | Bank payment schema, cheque/mode controls, WHT and payable allocation; posting tests. |
| Journal Voucher | Complete | Decimal/balanced journal validation is enforced below the UI; unbalanced journal rejection test and posting tests. |
| Quotation | Complete | Schema-driven quote editor/list and conversion to Sale Order; native renderer and workflow conversion tests. |
| Sale Order | Complete | Order editor/list and conversion to Delivery Challan; native renderer and workflow conversion tests. |
| Delivery Challan | Complete | Delivered quantities affect location stock; edits use the generic replacement path in `backend/repositories/document_repository.rs`. |
| Sale Invoice | Partial | Invoice editor, posting, AR integration, and stock handling exist; invoice tax liability and linked document edge cases remain limited. |
| Sale Return | Complete | Posted sale returns create an inbound stock movement; signed movement and resulting on-hand are asserted by `tests/workflows.rs::posted_purchase_and_sale_returns_reverse_stock_in_their_location`. |
| Chart of Accounts | Complete | Company-scoped create/edit/list, validation, parent-cycle rejection, GL and trial balance integration; edit action is wired in `frontend/views/accounting/maintain/chart_of_accounts_view.rs`. |
| Vendors | Complete | Existing party tables plus synchronized extended profiles; search/status, edit/versioning, import/export and attachment UI paths. Import/export backend and view tests exist; OS dialog interaction is unverified. |
| Items | Complete | Item maintenance, stock quantities, CSV/XLSX import/export, attachments and stock services; posted return and stock replacement paths are covered by workflow tests. |
| Customers | Complete | Customer master CRUD/extended profile, selectors, import/export and attachments; document selectors map profile mailing/shipping address fields into their corresponding form fields. |
| Inventory Locations | Complete | Location maintenance, selectors, CSV/XLSX import/export, stock views and movement services; exercised in inventory and workflow tests. |
| Shared Maintain search, filters, sort, columns, paging, bulk/archive/import/export/attachments | Partial | Implemented in `documents_view.rs` and import/export services; view/backend tests and 48 rendered screens. File-picker selection and dialog-driven end-to-end import/export remain unverified on this desktop session. |
| Employees | Complete | Employee/profile maintenance and reports; workflow test covers custom fields, leave date, payroll generation and duplicate protection. |
| Payroll runs | Complete | Payroll generation, calculations, draft/paid transitions, reports and posting; workflow tests cover duplicate handling, calculations and paid ledger posting. Role approval is part of the separate access-control limitation. |
| General Ledger | Complete | Native `ledger_view.rs`, ledger repository and ledger report tests; native harness rendered GL screen. |
| Trial Balance | Complete | Native report view and ledger-derived repository; accounting tests verify balanced totals. |
| AR/AP aging and statements | Complete | Native report view and FIFO subledger services; receivable allocation tests and accounting report tests. |
| Posting, fiscal-period checks, balanced entries and atomic writes | Complete | `backend/repositories/accounting_repository.rs`, transaction engine and purchase/sales/document services; tests reject unbalanced entries and verify atomic posting/rollback. |
| Settings, user/company administration, support and external integrations | Partial | Native settings/company/user routes exist, but settings and support integrations remain limited; no email, bank API, payment gateway or synchronization backend is configured. Authentication/role enforcement is not yet end-to-end. |
| Notifications, exports, templates, print/PDF and attachments | Partial | Native message/status feedback, CSV/XLSX/PDF output, templates, print and local attachment storage exist in code. Native OS file-dialog selection, dialog-driven export/import and attachment roundtrip remain unverified because desktop input/inspection is blocked in this environment. |

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

The native Iced render harness (`src/bin/native_ui_check.rs`) renders 48 screens. Rust workflow tests exercise the data/service workflows, including an on-disk close/reopen persistence test. Render output is not treated as proof of interactive behavior.

Current review checks:

| Check | Result |
|---|---|
| `cargo test --offline` | Passed: 780 tests (744 library and 36 integration), no failures. Includes stock return effects, custom employee profile values, validation/posting and close/reopen persistence workflows. |
| `cargo check --offline` | Passed. |
| `cargo fmt --check` | Passed after project-wide formatting was committed separately. |
| `cargo clippy --offline --all-targets --all-features -- -D warnings` | Passed with warnings denied. Narrow documented allowances retain the repository's module structure; all actionable findings were corrected. |
| `git diff --check` | Passed. |
| Native UI rendering | Native Iced harness renders 48 screens. This verifies screen construction/rendering only, not mouse or keyboard interaction. |
| Native mouse/keyboard workflows | Unverified. Root cause was a backend mismatch: the app bound to Wayland (`wl_seat` appeared in its startup trace) while the XTest helper found an old iconified XWayland window (`WM_STATE=Iconic`). XTest events therefore targeted the wrong surface. Winit 0.30 selects Wayland when the Wayland environment is present; setting `WINIT_UNIX_BACKEND=x11` did not switch it. A forced fake Wayland display fails with `WaylandError(Connection(NoCompositor))`. |
| Native desktop inspection/input access | Blocked by this desktop session. GNOME Shell `GetWindows` and `Screenshot` returned `AccessDenied`; the RemoteDesktop portal request remained pending and no usable input session was established. The live window cannot be introspected/captured through the available compositor APIs. This does not establish an application input-handler defect. |
| File dialogs, UI import/export and attachments | Unverified: the dialogs require native interactive selection, and the compositor input/inspection route above is unavailable. The CSV/XLSX/PDF, import/export and attachment code paths have Rust tests, but these results do not prove OS dialog interaction. |
| UI-created record across application restart | Unverified: no record was created through the live UI because native input could not be delivered. The Rust persistence workflow test closes and reopens the on-disk SQLite database and verifies the saved trip record. |

### Manual native verification checklist

Use a desktop session that can deliver input to the live Wayland or X11 surface and inspect it. Create a trip with the mouse and keyboard, submit it with a missing required field to check validation, save it, edit it, quit the app, relaunch it, and confirm the edited record remains. Create an unbalanced journal and confirm rejection, then post a balanced journal and verify the ledger/trial balance. In Maintain, import a template or CSV, export selected records, attach a local file to a record, reopen it, and remove the attachment. Open each file dialog, select a file/destination, and verify cancel with Escape. Record OS/backend and results before treating these flows as UI-verified.

No browser test applies to this native desktop application. The supplied image captures are design references rather than evidence of every interactive state.

## Remaining limitations

- No end-to-end mouse/keyboard desktop driver succeeded in this environment. XTest targeted a stale iconified XWayland window while the app ran on Wayland; compositor introspection and screenshots were denied, and RemoteDesktop input did not establish a session. Thus native navigation, keyboard entry, validation, save/edit, posting, import/export, attachment selection, file dialogs, and UI-created-record restart persistence remain unverified. The manual checklist above is the outstanding verification step.
- OS-level file picker behavior varies by platform and was not checked here.
- Roles/permissions are not fully enforced through every workflow; settings, support, outbound integrations and notification delivery are limited.
- Separate tax-liability posting is not implemented for invoice tax totals; the appropriate tax accounts and jurisdiction rules are not specified by the guide. Review this accounting policy before relying on tax-bearing posted invoices.
- Platform packaging/signing, cross-platform accessibility and OS theme switching remain outside this verification.
