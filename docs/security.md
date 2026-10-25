# MemonBooks – Security

This document records the security controls that **exist in the code today**, and states the gaps honestly. MemonBooks is an offline, single-machine desktop application: there is no network listener, no web session and no remote API (see [api.md](api.md)), so the main threats are a local attacker or other local user, loss or theft of the machine, malicious import/attachment files, and operator mistakes.

Related documents: [README](../README.md) · [Architecture](architecture.md) · [Database](database.md) · [Testing & QA](testing-qa.md)

## Table of contents

1. [Summary](#1-summary)
2. [Authentication](#2-authentication)
3. [Authorisation (roles and permissions)](#3-authorisation-roles-and-permissions)
4. [Password handling](#4-password-handling)
5. [Sessions](#5-sessions)
6. [Input validation](#6-input-validation)
7. [SQL injection and data integrity](#7-sql-injection-and-data-integrity)
8. [Files: attachments, imports, exports](#8-files-attachments-imports-exports)
9. [Secrets, logs and local data](#9-secrets-logs-and-local-data)
10. [Audit trail](#10-audit-trail)
11. [Dependencies and supply chain](#11-dependencies-and-supply-chain)
12. [Packaging and sandboxing](#12-packaging-and-sandboxing)
13. [Gaps and risks](#13-gaps-and-risks)
14. [Open questions](#14-open-questions)

---

## 1. Summary

| Control | Status |
|---------|--------|
| Password hashing (Argon2, random salt) | Implemented |
| Login with generic failure message | Implemented |
| TOTP two-factor (backend + login screen field) | Implemented (no enrolment screen found – see 2) |
| Forced password change on first login / after reset | Implemented |
| Role-based permissions | Partial (catalogue of 6 keys; only `users.manage` is enforced) |
| Login required at start-up | **Not enforced** – start-up opens as the first active user (development shortcut) |
| Parameterised SQL | Implemented |
| Balanced-ledger and closed-period enforcement | Implemented |
| Audit log | Partial (ledger postings only) |
| Account lockout / rate limiting | Not implemented |
| Database or attachment encryption | Not implemented |
| Session persistence / idle timeout | Not implemented (sessions are modelled, not used by the UI) |
| Licence / activation | Code exists, not wired in |

## 2. Authentication

* **Login screen:** `frontend/views/auth/login_view.rs` → `AuthService::login_with_credentials(username, password)` → `core/auth/authentication.rs::authenticate`.
* Unknown user, inactive user and wrong password return the **same** message, `invalid username or password`, so usernames cannot be enumerated.
* **Two-factor:** if `users.two_factor_enabled`, `authenticate` returns `TwoFactorRequired(user)` and the caller must pass a code to `verify_two_factor`. TOTP is SHA-1, 6 digits, 30 s step, ±1 step skew (`two_factor.rs`). The login view has a code field. No screen to enable 2FA or show the secret was found in `frontend/`; the secret is stored in `users.two_factor_secret` **in plain base32 text**.
* **Start-up bypass (important):** `src/main.rs` calls `load_startup_user()`, documented as a *"Temporary development startup session: open the app directly as the first active user while authentication UI is being finalized"*. Anyone who can launch the app is therefore signed in as the first active user (normally `admin`) without a password. The login screen appears only after Switch User / sign-out. If the first user has `must_change_password`, the flag is cleared for this session. This must be removed before real use.
* **First-run admin:** when no roles exist, `ensure_default_admin` creates the `Administrator` role (all known permissions) and user `admin` with a random 16-character alphanumeric password and `must_change_password = true`. The password is written to the log with `log::warn!` (see 9).
* **Password reset:** `request_password_reset(username)` sets a flag silently (no information leak); an administrator then calls `reset_password`, which returns a new random 16-character temporary password and forces a change. There is no e-mail or SMS delivery (no network crates).

## 3. Authorisation (roles and permissions)

* Tables `roles`, `role_permissions`, `users.role_id` ([database.md](database.md#62-users-and-audit-400403)).
* Permission keys are an open catalogue (`core/permissions.rs`): `users.manage`, `roles.manage`, `organization.manage`, `reports.view`, `accounting.chart_of_accounts.manage`, `accounting.fiscal_periods.manage`. `Role::validate` rejects unknown keys.
* **Enforced:** `AuthService::create_user`, `update_user` and `reset_password` call `require_permission(.., "users.manage", ..)`; the Users screen is shown only when the session user has `users.manage` (`src/main.rs`).
* **Not enforced:** the other five keys exist but no service or view checks them. Any signed-in user can use accounting, sales, purchases, inventory, HR, payroll and settings functions. There is only the Administrator role by default; no screen for editing roles was found.
* Denials use `AppError::Validation("only an administrator can …")`.

## 4. Password handling

| Aspect | Implementation (`core/auth/passwords.rs`) |
|--------|-------------------------------------------|
| Algorithm | Argon2 (`argon2` 0.5, `Argon2::default()` = Argon2id) |
| Salt | fresh `SaltString` from `OsRng` per hash (unit-tested to differ) |
| Storage | PHC string in `users.password_hash` |
| Minimum length | 10 characters (`MIN_PASSWORD_LENGTH`); no complexity, breach or history checks |
| Comparison | `PasswordVerifier::verify_password` (constant-time inside the crate) |
| Change | requires the current password, also on the forced first change |
| Temporary passwords | 16 random alphanumeric characters, shown once to the administrator |

Argon2 parameters are the crate defaults and have not been tuned or reviewed.

## 5. Sessions

`core/auth/sessions.rs` defines `Session` (32 random bytes, hex token, 12-hour validity, `revoked_at`) and the table `sessions` exists, but the UI keeps the signed-in `User` in memory (`App.session`) and does not create or check session rows. There is no idle timeout and no automatic lock. Closing the app ends the session.

## 6. Input validation

* Domain types implement `Validate`; helpers `require_non_empty`, `require_max_length`, `require_non_negative`, `require_positive` (`core/validation.rs`). Example limits: role name 100 characters, description 500.
* **Ledger rules** (`transaction_engine`): balanced debits = credits, draft-only posting, same company, date inside an **open** fiscal period. Posted entries are corrected by reversal only. Overpayments of receivables/payables are rejected before posting.
* **Money** is `Decimal`, never float; decimals are stored as TEXT.
* **Unique keys** at database level: usernames, role names, account codes per company, SKUs per company, reference codes, etc. ([database.md](database.md#7-indexes-and-constraints-summary)).
* Foreign keys are on (`PRAGMA foreign_keys = ON`).
* Views validate form input before calling services; errors are shown inline.

## 7. SQL injection and data integrity

* All values are bound with `params![...]` / `?` placeholders in `rusqlite`.
* A few queries use `format!` to insert **table or column identifiers** (`document_repository.rs`, `master_profiles.rs`, `banking_repository.rs`). These identifiers are internal constants (for example `customers`/`vendors`, column lists), not user input; values stay bound. Reviewers should keep it that way.
* Multi-table writes run in one SQLite transaction; migrations run in individual transactions and are recorded only on success.
* `schema_migrations` rejects duplicate version numbers or a version re-used under a different name.

## 8. Files: attachments, imports, exports

**Attachments** (vendor attachments in `vendors_view.rs`; also referenced by accounts and workflow documents):

* The user picks a file with the native `rfd` dialog; it is **copied** (`std::fs::copy`) into `<data dir>/uploads/` under the name `<timestamp-nanos>-<index>-<original file name>`; the database stores `file_name`, `display_name` and `size_bytes`.
* The stored name embeds the original file name taken from `Path::file_name()` of the user's own selection, so it cannot contain path separators.
* **Gaps:** no file-type allow-list, no size limit, no malware scanning, no encryption at rest, files are not opened by the app (so no in-app parsing risk), and removal of an attachment record behaviour was not audited for deleting the file on disk.

**Import** (`database/exports/import.rs`): 20 MB cap on the file and on each XLSX XML part; UTF-8 CSV only; XLSX read with `zip` + `quick-xml` with only predefined XML entities allowed, no formulas evaluated, no macros executed; only the first worksheet is read. `DocumentService::import` validates all rows before saving any.

**Export:** CSV with RFC 4180 quoting. **No spreadsheet-formula neutralisation** is applied: a text value that starts with `=`, `+`, `-` or `@` is written as is, so a malicious value stored in a name field could execute as a formula when the CSV is opened in a spreadsheet application. This is a known gap.

## 9. Secrets, logs and local data

| Item | Where | Risk |
|------|-------|------|
| Database | `memonbooks.sqlite3` in the user data directory | Not encrypted; protected only by OS file permissions. Contains all financial data, `password_hash` and `two_factor_secret`. |
| First-run admin password | `logs/memonbooks.log` (WARN) | Anyone who can read the log can read the initial password until it is changed. Remove the line after the first login. |
| 2FA secret | `users.two_factor_secret` | plain text in the database |
| Licence signing secret | `sign_license_key(secret, …)` takes a caller-supplied `&[u8]`; no secret is stored in the repository | Licensing is not wired in. |
| Source repository | git history | Two files with unprintable names (one with a `.pub` companion, so they look like an SSH key pair) are **tracked in git** (commit `e1d8afc` "ssh"). Their contents were not opened for this document. **If one is a private key, treat it as compromised: rotate it and remove it from the history.** `Apis.txt` (a personal utility-bill URL with a contract number) and two image files in the root are also tracked and are not part of the application. See [Open questions](#14-open-questions). |
| Environment | `MEMONBOOKS_ENV` | selects log verbosity only (Debug vs Info) |

Debug builds also print log lines to the terminal.

## 10. Audit trail

`audit_logs(user_id, action, entity_type, entity_id, detail, occurred_at)` is written inside the posting transaction by `AccountingRepository::post_transaction`. Other create/update/delete operations, logins, failed logins, password changes and permission changes are **not** logged. `audit/activity_logs.rs` and `audit/change_history.rs` are empty placeholders. Workflow documents keep a `revision` counter and `modified_at`, not a history. Vendor/account rows have no `modified_by`.

## 11. Dependencies and supply chain

* Direct dependencies are listed in `Cargo.toml`; versions are locked by `Cargo.lock` (committed).
* No `cargo audit` / `cargo deny` configuration or CI was found in the repository.
* `rusqlite` uses the `bundled` feature (SQLite compiled in), so the SQLite version is pinned by the crate, not by the OS.
* `rfd` with `gtk3` links system GTK3 on Linux.
* `license = "UNLICENSED"`, `publish = false` in `Cargo.toml`.

## 12. Packaging and sandboxing

* **Flatpak manifest** (`packaging/linux/flatpak/*.yml`): permissions are `--share=ipc`, `--socket=wayland`, `--socket=fallback-x11`, `--device=dri`; **no network, no host filesystem**. Because attachments are picked through `rfd`, a sandboxed build may need the portal; this was not tested.
* macOS: `packaging/macos/entitlements.plist`; Windows: `AppxManifest.xml`. Neither was reviewed or built for this document.

## 13. Gaps and risks

Ordered by severity:

| # | Gap | Impact |
|---|-----|--------|
| 1 | Start-up opens as the first active user without a password (`load_startup_user`) | Authentication is bypassed for anyone with local access to the app |
| 2 | Permissions other than `users.manage` are not enforced | Any signed-in user can post, edit and delete financial data |
| 3 | Database, attachments and backups are unencrypted | Theft of the laptop or backup exposes everything |
| 4 | First-run admin password and 2FA secret stored in clear (log / DB) | Local disclosure |
| 5 | No lockout or rate limiting on login | Offline guessing is limited only by Argon2 cost |
| 6 | Audit log covers postings only | Limited accountability |
| 7 | CSV formula injection not neutralised | Spreadsheet-side code execution risk |
| 8 | No attachment type or size limits | Disk filling, odd file types |
| 9 | Sessions not persisted or expired in the UI | No idle lock |
| 10 | No automated dependency vulnerability scanning | Unknown advisories |
| 11 | Possible private key committed to git (see section 9) | Credential exposure if the repository is ever shared or pushed |
| 12 | No built-in backup (see [database.md](database.md#8-backup-and-restore)) | Data-loss risk is also a security (availability) risk |

## 14. Open questions

1. When will `load_startup_user` be removed so that login is mandatory?
2. Which permission keys must be enforced per screen/service, and who edits roles?
3. Is encryption at rest (SQLCipher, OS keychain for secrets) required for the target stores?
4. Should the first-run password be shown on screen once instead of being logged?
5. The tracked root files with unprintable names (one with a `.pub` companion, added in commit `e1d8afc`) must be inspected by the owner; if they are key material, rotate the key and purge it from git history. This documentation commit does not touch them.
6. Is a security review of the Flatpak/MSIX/App Store sandbox permissions planned?
7. Required retention and detail level of the audit log?
