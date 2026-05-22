# MemonBook ERP — Complete Project Build Instructions

## Project Info
- Name: MemonBook ERP (Offline Accounting + HR System)
- Path: /var/home/memon/Projects/MemonBooks
- OS: Fedora Silverblue — Toolbox dev container
- Doc: MemonBook_Complete_Project_v1.0.docx (reference document)

## Tech Stack
- Backend: Python FastAPI + Uvicorn
- Database: SQLite + SQLCipher (AES-256 encrypted)
- Frontend: HTML + CSS + Vanilla JS (NO React, NO Vue, NO Angular)
- Desktop/Mobile: Tauri v2
- Bundler: PyInstaller (FastAPI → .exe)
- Encryption: Python cryptography library (AES-256 + PBKDF2)
- Auth: bcrypt + JWT
- Hardware ID: WMI + uuid (Python)
- PDF: ReportLab

## Brand Colors
- Primary: #0072B7
- Dark Blue: #0063AA
- Light Blue: #0099DF
- Accent: #007DC3
- Success: #16A34A
- Danger: #DC2626
- Warning: #D97706

## Theme
- System default: dark mode ON = dark theme, OFF = light theme
- Manual toggle in sidebar bottom
- BOTH themes required in every CSS file

## Design Rules
- Dashboard style: clean cards (KPI stats) + charts + sidebar — jaise shared image
- Sirf EK components.js file — sab Button, Input, Table, Modal, Toast, Validator yahaan se import
- Sirf 6 button types: primary, secondary, success, danger, warning, info
- Har input field ki validation zaroori hai (email, CNIC, IBAN, amount, percent etc.)
- Sidebar: collapsible (260px → 64px), search box, role-based visibility, dark/light

## Security Architecture
- 2 DB files: master.db (admin, full) + user.db (user, limited)
- master.db Key = PBKDF2(SHA256, DEVICE_HASH + APP_SECRET, DB_SALT, 600000 iterations)
- user.db Key = PBKDF2(SHA256, DEVICE_HASH + SECRET_TOKEN + USER_HASH, DB_SALT, 600000)
- master.db remove = user.db bhi band
- Doosre PC par copy = DEVICE_HASH wrong = dono band
- Hardware fingerprint: Windows(CPU+MB+HDD+MAC+GUID), Linux(/proc+/sys+machine-id), Mac(IOPlatform+UUID+MAC), Android(AndroidID+IMEI+MAC), iOS(IDFV+HWMachine)
- Field-level AES-256 encryption: cnic, passport_no, bank_account, bank_iban, imei_1, imei_2, mac_address, audit old_values, audit new_values
- Mobile: IMEI + MAC must be in approved_devices table — same WiFi/hotspot required

## Code Rules
- Production-ready code — no placeholders, no TODOs
- Python: type hints everywhere, proper error handling, docstrings
- JS: ES2020+, async/await, no var (only const/let)
- Every API endpoint: JWT check + permission check + device check
- Auto document numbering: PO-2024-0001, GRN-2024-0001 etc.
- Auto Journal Voucher on every transaction post
- Stock ledger update on every GRN accept and DC deliver

## Database Pattern
- Every transaction = Header table + Lines table
- All IDs = UUID (TEXT in SQLite)
- All timestamps = ISO 8601 TEXT
- Status flow: draft → submitted → approved → posted → closed/cancelled

## Folder Structure
MemonBooks/
├── src-tauri/src/main.rs + commands.rs
├── src-tauri/tauri.conf.json + Cargo.toml
├── frontend/index.html
├── frontend/pages/ (har module ka html)
├── frontend/assets/css/theme.css + components.css + sidebar.css
├── frontend/assets/js/core/ (app.js, router.js, api.js, auth.js, security.js)
├── frontend/assets/js/components/components.js (SINGLE FILE — sab components)
├── frontend/assets/js/modules/ (purchases/, sales/, payments/, accounts/, hr/, masters/)
├── backend/main.py + config.py
├── backend/security/ (device_fingerprint.py, encryption.py, auth.py, permissions.py)
├── backend/database/ (connection.py, master_db.py, user_db.py)
├── backend/models/ (config/, masters/, purchases/, sales/, payments/, accounts/, hr/, system/)
├── backend/routers/ (auth, device, masters/, purchases/, sales/, payments/, accounts/, hr/, reports/)
├── backend/services/ (auto_jv.py, stock_ledger.py, payroll_calculator.py, numbering.py, sync.py)
├── backend/utils/ (helpers.py, validators.py, pdf_generator.py)
├── data/db/ (master.db, user.db)
├── data/backups/ + exports/ + logs/
├── scripts/ (setup_db.py, build_backend.py, generate_keys.py)
├── requirements.txt + package.json + CLAUDE.md

## Modules To Build (In This Order)
PHASE 1 — Security Foundation:
1. scripts/generate_keys.py — APP_SECRET + DB_SALT generate
2. backend/security/device_fingerprint.py — Windows + Linux + Mac
3. src-tauri/src/commands.rs — Android + iOS device info
4. backend/security/encryption.py — AES-256 + PBKDF2 SQLCipher keys
5. backend/database/connection.py — master.db + user.db SQLCipher open
6. backend/security/auth.py — login, JWT, 2-DB token
7. backend/security/permissions.py — role + module access check
8. scripts/setup_db.py — all tables create + admin user
9. backend/main.py + config.py — FastAPI entry + all routers register

PHASE 2 — Frontend Foundation:
10. frontend/assets/css/theme.css — CSS variables, dark+light theme
11. frontend/assets/css/components.css — all UI components CSS
12. frontend/assets/css/sidebar.css — sidebar dark+light
13. frontend/assets/js/components/components.js — Button, Input, Badge, Table, Modal, Toast, Validator (ALL in one file)
14. frontend/assets/js/core/app.js + router.js + api.js + auth.js
15. frontend/pages/login.html — login page
16. frontend/pages/dashboard.html + frontend/assets/js/modules/dashboard.js — KPI cards + charts
17. src-tauri/tauri.conf.json — desktop + mobile config

PHASE 3 — Masters:
18. COA (Chart of Accounts) — hierarchy tree, CRUD
19. Customers — full form, NTN, STRN, credit limit
20. Vendors — WHT settings, bank IBAN
21. Items — price lists, UOM conversion
22. Inventory Locations, UOM, Tax Config, Payment Terms, Currencies, Fiscal Years
23. backend/services/numbering.py — auto document numbers

PHASE 4 — Purchases:
24. Purchase Order (PO) — header + lines form + approval
25. GRN — received/accepted/rejected qty + stock update
26. Purchase Invoice (PI) — WHT + auto JV
27. Purchase Return (PR)

PHASE 5 — Sales:
28. Quotation + Sale Order
29. Delivery Challan + stock outward
30. Sale Invoice + auto JV + aging
31. Sale Return

PHASE 6 — Payments + Accounts:
32. Cash/Bank Receipt (CR/BR)
33. Cash/Bank Payment (CP/BP) + WHT
34. Journal Voucher (manual + auto)
35. Ledger view + Trial Balance

PHASE 7 — HR & Payroll:
36. Employees — full profile (personal, job, salary, bank)
37. Attendance + Leave Management
38. Payroll Calculator + Payroll Runs + Payslip PDF

PHASE 8 — Reports:
39. Sales, Purchase, Stock, Payroll, Financial reports
40. PDF export for all documents

PHASE 9 — Mobile + Build:
41. Device registration API (IMEI + MAC)
42. Mobile sync queue system
43. Tauri v2 Android + iOS build
44. PyInstaller bundle (FastAPI → exe)
45. Final integrity check

## Start Command
Pehle yeh karo:
1. Poori folder structure banao (empty files ke saath)
2. requirements.txt + package.json likho
3. Phir Phase 1 se ek ek file banao

Har file complete aur production-ready honi chahiye.
Ek file mukammal karo phir agla shuru karo.
Koi placeholder nahi, koi TODO nahi.
