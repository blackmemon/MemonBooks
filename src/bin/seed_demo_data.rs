//! Admin tool: populates the configured database with a realistic demo
//! dataset for manual testing, entirely through `backend/services/*.rs`
//! (never raw SQL inserts -- every posting goes through the real
//! transaction engine, so debit = credit and every other invariant
//! holds exactly like real data would). Never run as part of normal
//! app startup; run explicitly:
//!
//!     cargo run --bin seed_demo_data
//!
//! Safe to re-run: if a `DEMO-`-tagged company already exists, this
//! clears it (via the same logic `clear_demo_data` uses) and
//! regenerates from scratch, rather than piling up duplicates on every
//! run. Chosen over "skip if present" because this script is expected
//! to be tweaked and re-run during manual testing -- a stale dataset
//! from three edits ago isn't useful, and re-running being a genuine
//! reset makes that safe to rely on.
//!
//! Dependency order: company/fiscal periods -> chart of accounts, tax
//! rates -> customers/vendors/products/warehouses/employees/bank
//! accounts/terminals -> transactional records that reference all of
//! the above (invoices, bills, POS sales, journal entries, expense
//! claims, payroll runs) last, since posting any of them needs valid
//! foreign keys and an open fiscal period.
//!
//! See `src/demo_data.rs` for the `DEMO-` tagging convention this
//! relies on, and the module-level doc comment there for why deleting
//! by company scope is safe.

use std::collections::HashMap;

use chrono::{Datelike, NaiveDate, Utc};
use rand::seq::SliceRandom;
use rand::Rng;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

use memonbooks::backend::accounting::accounting_rules::AccountType;
use memonbooks::backend::accounting::chart_of_accounts::Account;
use memonbooks::backend::accounting::fiscal_periods::FiscalPeriod;
use memonbooks::backend::core::auth::passwords::hash_password;
use memonbooks::backend::core::errors::AppResult;
use memonbooks::backend::core::roles::Role;
use memonbooks::backend::core::users::User;
use memonbooks::backend::expenses::expense_categories::ExpenseCategory;
use memonbooks::backend::expenses::expense_claims::ExpenseClaim;
use memonbooks::backend::hr::employees::Employee;
use memonbooks::backend::hr::payroll::{PayrollEntry, PayrollRun};
use memonbooks::backend::hr::salary::{SalaryComponent, SalaryStructure};
use memonbooks::backend::inventory::products::Product;
use memonbooks::backend::inventory::warehouses::Warehouse;
use memonbooks::backend::parties::customers::Customer;
use memonbooks::backend::parties::vendors::Vendor;
use memonbooks::backend::pos::payments::{PaymentMethod, PosPayment};
use memonbooks::backend::pos::sales::{PosSale, PosSaleLine};
use memonbooks::backend::pos::sessions::PosSession;
use memonbooks::backend::pos::terminals::Terminal;
use memonbooks::backend::purchases::bills::{Bill, BillLine};
use memonbooks::backend::repositories::user_repository::UserRepository;
use memonbooks::backend::sales::invoices::{Invoice, InvoiceLine};
use memonbooks::backend::services::accounting_service::AccountingService;
use memonbooks::backend::services::banking_service::BankingService;
use memonbooks::backend::services::expense_service::ExpenseService;
use memonbooks::backend::services::inventory_service::InventoryService;
use memonbooks::backend::services::organization_service::OrganizationService;
use memonbooks::backend::services::party_service::PartyService;
use memonbooks::backend::services::purchase_service::PurchaseService;
use memonbooks::backend::services::sales_service::SalesService;
use memonbooks::backend::services::tax_service::TaxService;
use memonbooks::backend::tax::tax_rates::TaxDirection;
use memonbooks::backend::banking::bank_accounts::BankAccount;
use memonbooks::configs::app::AppConfig;
use memonbooks::database::database::Database;
use memonbooks::demo_data::{self, DEMO_CASHIER_DISPLAY_NAME, DEMO_CASHIER_USERNAME, DEMO_COMPANY_NAME, DEMO_ROLE_NAME};

// -- counts -----------------------------------------------------------
// See the run summary printed at the end (and the commit message) for
// the reasoning behind each of these.
const CUSTOMER_COUNT: usize = 100;
const VENDOR_COUNT: usize = 100;
const PRODUCT_COUNT: usize = 100;
const WAREHOUSE_COUNT: usize = 100;
const EMPLOYEE_COUNT: usize = 15;
const BANK_ACCOUNT_COUNT: usize = 100;
const TERMINAL_COUNT: usize = 3;
const INVOICE_COUNT: usize = 100;
const BILL_COUNT: usize = 100;
const POS_SESSION_COUNT: usize = 10;
const POS_SALES_PER_SESSION: usize = 10; // 10 * 10 = 100 POS sales
const JOURNAL_ENTRY_COUNT: usize = 100;
const EXPENSE_CLAIM_COUNT: usize = 20;
const PAYROLL_RUN_COUNT: usize = 3;

// -- name pools (obviously-fake but realistic-looking) -----------------

const FIRST_NAMES: &[&str] = &[
    "James", "Maria", "Ahmed", "Wei", "Fatima", "Carlos", "Aisha", "Liam", "Sofia", "Kenji", "Priya", "Omar", "Elena",
    "David", "Noor", "Lucas", "Amara", "Ravi", "Grace", "Hassan", "Mei", "Daniel", "Zara", "Samuel", "Layla", "Ethan",
    "Nadia", "Miguel", "Chloe", "Yusuf",
];
const LAST_NAMES: &[&str] = &[
    "Khan", "Garcia", "Chen", "Smith", "Ali", "Muller", "Silva", "Nguyen", "Kowalski", "Haddad", "Patel", "Rossi",
    "Ibrahim", "Anderson", "Tanaka", "Osei", "Costa", "Bakr", "Novak", "Reyes", "Lindqvist", "Abbas", "Fischer",
    "Cohen", "Diallo", "Park", "Moreau", "Santos", "Weber", "Farooq",
];

const PRODUCT_MATERIALS: &[&str] = &[
    "Steel", "Aluminum", "Cotton", "Plastic", "Ceramic", "Wooden", "Glass", "Leather", "Rubber", "Bamboo", "Copper",
    "Canvas", "Nylon", "Stainless", "Recycled", "Brass", "Wool", "Carbon Fiber", "Porcelain", "Woven",
];
const PRODUCT_ITEMS: &[&str] = &[
    "Widget", "Bracket", "Container", "Fastener", "Panel", "Bottle", "Tray", "Hinge", "Cable", "Adapter", "Frame",
    "Mug", "Chair", "Table", "Lamp", "Shelf", "Box", "Bag", "Tool Kit", "Gadget",
];

fn full_name(rng: &mut impl Rng) -> String {
    format!("{} {}", FIRST_NAMES.choose(rng).unwrap(), LAST_NAMES.choose(rng).unwrap())
}

fn product_name(rng: &mut impl Rng) -> String {
    format!("{} {}", PRODUCT_MATERIALS.choose(rng).unwrap(), PRODUCT_ITEMS.choose(rng).unwrap())
}

fn slugify_for_email(name: &str) -> String {
    name.to_ascii_lowercase().chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '.' }).collect::<String>()
}

fn random_date_between(rng: &mut impl Rng, start: NaiveDate, end: NaiveDate) -> NaiveDate {
    let span_days = (end - start).num_days().max(0);
    start + chrono::Days::new(rng.gen_range(0..=span_days as u64))
}

// -- Pakistan-specific data pools, used only by the Maintain sections that
// carry no address/city field on their model (Vendor, Customer, Warehouse,
// BankAccount) -- so "Pakistani business data" here means realistic names,
// phone numbers, NTNs, cities, and bank names, not fields the model doesn't
// have.

const PK_NAME_PREFIXES: &[&str] = &[
    "Al-Habib", "Al-Noor", "Al-Rehman", "Al-Fateh", "Al-Madina", "Al-Karam", "Crescent", "Indus", "Ravi", "Chenab",
    "Sindh", "Punjab", "National", "United", "Metro", "Continental", "Superior", "Pearl", "Diamond", "Silk Route",
    "Zam Zam", "Khan", "Malik", "Sheikh", "Chaudhry", "Qureshi", "Ansari", "Butt", "Rajput", "Siddiqui",
];
const PK_VENDOR_NOUNS: &[&str] = &[
    "Traders", "Suppliers", "Distributors", "Trading Co", "Impex", "Mills", "Textiles", "Industries", "Exports",
    "Enterprises", "Agencies", "Corporation", "International", "Trading Company", "Group", "Associates",
    "Manufacturing Co", "Wholesalers", "Trading House", "Brothers",
];
const PK_CUSTOMER_NOUNS: &[&str] = &[
    "Retail Store", "Departmental Store", "General Store", "Super Store", "Mart", "Cash & Carry", "Enterprises",
    "Traders", "Retailers", "Wholesale Mart", "Shopping Center", "Emporium", "Plaza", "Outlets", "Trading Co",
    "& Sons", "Garments House", "Electronics Store", "Hardware Store", "Grocers",
];
const PK_MOBILE_PREFIXES: &[&str] = &[
    "0300", "0301", "0302", "0303", "0304", "0305", "0306", "0307", "0308", "0309", "0311", "0312", "0313", "0314",
    "0315", "0316", "0317", "0318", "0319", "0320", "0321", "0322", "0323", "0324", "0325", "0330", "0331", "0332",
    "0333", "0334", "0335", "0336", "0337", "0340", "0341", "0342", "0343", "0344", "0345", "0346", "0347", "0348",
    "0349",
];
const PK_CITIES: &[&str] = &[
    "Karachi", "Lahore", "Faisalabad", "Islamabad", "Rawalpindi", "Multan", "Peshawar", "Quetta", "Sialkot",
    "Gujranwala", "Hyderabad", "Sukkur", "Bahawalpur", "Sargodha", "Sheikhupura", "Mardan", "Gujrat", "Kasur",
    "Rahim Yar Khan", "Sahiwal", "Okara", "Wah Cantt", "Dera Ghazi Khan", "Mingora", "Nawabshah", "Jhelum",
    "Sadiqabad", "Abbottabad", "Mirpur Khas", "Chiniot",
];
const WAREHOUSE_TYPES: &[&str] = &[
    "Main Warehouse", "Distribution Center", "Storage Facility", "Central Depot", "Cold Storage Unit",
    "Regional Warehouse", "Logistics Hub", "Fulfillment Center", "Bonded Warehouse", "Stockyard",
];
const PK_INDUSTRIAL_AREAS: &[&str] = &[
    "Industrial Estate", "Industrial Area", "Trade Centre", "Export Processing Zone", "Free Trade Zone",
    "Commercial Area", "Logistics Park", "Cargo Village", "Business Park", "Trading Estate",
];

/// Pool the 100 demo bank accounts cycle through: (display bank name, the
/// chart-of-accounts key for the GL asset account it posts to, its
/// illustrative 4-letter IBAN bank code, its currency). Multiple bank
/// account rows legitimately share a GL key here -- `BankAccount` has no
/// stored balance of its own (see `book_balance` in
/// `banking_repository.rs`, which derives it from the linked GL account's
/// ledger), so this models several physical accounts a company holds at
/// the same bank rolling up into that bank's one ledger account, same as
/// a real chart of accounts would.
const PK_BANKS: &[(&str, &str, &str, &str)] = &[
    ("Habib Bank Limited", "bank_hbl", "HABB", "PKR"),
    ("United Bank Limited", "bank_ubl", "UNIL", "PKR"),
    ("MCB Bank Limited", "bank_mcb", "MUCB", "PKR"),
    ("Meezan Bank Limited", "bank_meezan", "MEZN", "PKR"),
    ("Bank Alfalah Limited", "bank_alfalah", "ALFH", "PKR"),
    ("Allied Bank Limited", "bank_abl", "ABPA", "PKR"),
    ("National Bank of Pakistan", "bank_nbp", "NBPA", "PKR"),
    ("Faysal Bank Limited", "bank_faysal", "FAYS", "PKR"),
    ("Askari Bank Limited", "bank_askari", "ASCM", "PKR"),
    ("Bank Al Habib Limited", "bank_alhabib", "BAHL", "PKR"),
    ("Standard Chartered Bank (Pakistan) Limited", "bank_scb", "SCBL", "PKR"),
    ("JS Bank Limited", "bank_js", "JSBL", "PKR"),
    ("Soneri Bank Limited", "bank_soneri", "SONE", "PKR"),
    ("Habib Bank Limited", "bank_fcy_usd", "HABB", "USD"),
];

/// Builds `count` guaranteed-distinct "`prefix` `noun`" combinations by
/// shuffling the full prefix x noun cartesian product and taking a
/// prefix of it, rather than drawing independently at random -- with
/// only ~30 prefixes independent draws would collide often enough (100
/// draws from a few hundred combos) to violate the "unique name per
/// record" expectation vendors/customers carry here.
fn unique_business_names(rng: &mut impl Rng, count: usize, nouns: &[&str]) -> Vec<String> {
    let mut combos: Vec<String> =
        PK_NAME_PREFIXES.iter().flat_map(|prefix| nouns.iter().map(move |noun| format!("{prefix} {noun}"))).collect();
    combos.shuffle(rng);
    combos.truncate(count);
    combos
}

fn pk_phone_number(rng: &mut impl Rng) -> String {
    let prefix = PK_MOBILE_PREFIXES.choose(rng).unwrap();
    let suffix: u32 = rng.gen_range(1_000_000..9_999_999);
    format!("{prefix}-{suffix}")
}

/// A plausible-looking Pakistani NTN (National Tax Number): 7 digits, a
/// dash, then a single digit -- not a real checksum algorithm, just
/// enough shape to look right. `seed` (a 7-digit number) must be unique
/// per call to guarantee every generated NTN is unique.
fn pk_ntn(seed: u32) -> String {
    let check = (seed % 9) + 1;
    format!("{seed:07}-{check}")
}

/// A plausible-looking Pakistani IBAN: "PK" + 2-digit check + 4-letter
/// bank code + 16-digit account number -- illustrative formatting only
/// (not a real mod-97 checksum). `seed` must be unique per call to
/// guarantee every generated IBAN is unique.
fn pk_iban(bank_code: &str, seed: usize) -> String {
    let check = 10 + (seed % 89);
    let account_digits = 1_000_000_000_000u64 + seed as u64 * 97;
    format!("PK{check:02}{bank_code}{account_digits:016}")
}

/// Bundles every service the seeder needs, constructed once from the
/// same `Database` handle -- mirrors how `src/main.rs` builds one
/// `Ctx` per view rather than reopening the database repeatedly.
struct Services {
    organization: OrganizationService,
    accounting: AccountingService,
    party: PartyService,
    inventory: InventoryService,
    sales: SalesService,
    purchase: PurchaseService,
    banking: BankingService,
    tax: TaxService,
    expense: ExpenseService,
}

impl Services {
    fn new(db: Database) -> Self {
        Self {
            organization: OrganizationService::new(db.clone()),
            accounting: AccountingService::new(db.clone()),
            party: PartyService::new(db.clone()),
            inventory: InventoryService::new(db.clone()),
            sales: SalesService::new(db.clone()),
            purchase: PurchaseService::new(db.clone()),
            banking: BankingService::new(db.clone()),
            tax: TaxService::new(db.clone()),
            expense: ExpenseService::new(db),
        }
    }
}

fn main() {
    if let Err(err) = run() {
        eprintln!("seed_demo_data failed: {err}");
        std::process::exit(1);
    }
}

fn run() -> AppResult<()> {
    let config = AppConfig::load()?;
    println!("database: {}", config.database_path.display());

    let db = Database::open(&config.database_path)?;
    db.run_migrations(&memonbooks::all_migrations())?;

    let cleared = demo_data::clear_demo_data(&db)?;
    if cleared.companies_removed > 0 {
        println!("Existing demo data found -- cleared it before regenerating (re-run is a full reset, not an append).");
    }

    let mut rng = rand::thread_rng();
    let services = Services::new(db.clone());

    let today = Utc::now().date_naive();
    let current_year_start = NaiveDate::from_ymd_opt(today.year(), 1, 1).unwrap();
    let prior_year_start = NaiveDate::from_ymd_opt(today.year() - 1, 1, 1).unwrap();
    let prior_year_end = NaiveDate::from_ymd_opt(today.year() - 1, 12, 31).unwrap();
    let data_range_start = prior_year_start;
    let data_range_end = today;

    // -- 1. company + fiscal periods -----------------------------------
    let company = services.organization.create_company(DEMO_COMPANY_NAME, "USD")?;
    let company_id = company.id.expect("just-created company must have an id");
    services.accounting.create_fiscal_period(&FiscalPeriod::new(company_id, "DEMO- FY prior year", prior_year_start, prior_year_end))?;
    // create_company already made the current-year period.
    println!("Company: {DEMO_COMPANY_NAME} (id {company_id}), 2 fiscal periods (prior year + current year)");

    // -- 2. demo system user (for POS sessions -- users/roles aren't company-scoped) --
    // No permissions granted -- nothing this script calls checks them,
    // and a real cashier role's permission set is an app-configuration
    // concern, not something demo data needs to model.
    let user_repo = UserRepository::new(db.clone());
    let role = user_repo.create_role(&Role::new(DEMO_ROLE_NAME))?;
    let cashier = user_repo.create_user(&User::new(DEMO_CASHIER_USERNAME, DEMO_CASHIER_DISPLAY_NAME, hash_password("Demo-Cashier-Password-1")?, role.id.unwrap()))?;
    println!("Demo system user: {DEMO_CASHIER_USERNAME} (for POS sessions only)");

    // -- 3. chart of accounts --------------------------------------------
    let accounts = create_chart_of_accounts(&services.accounting, company_id)?;
    println!("Chart of accounts: {} accounts", accounts.len());

    // -- 4. tax rates ------------------------------------------------------
    let tax_rate_count = create_tax_rates(&services.tax, company_id, &accounts)?;
    println!("Tax rates: {tax_rate_count}");

    // -- 5. opening balance (setup, not counted as one of the 100 journal entries) --
    services.accounting.post_transaction(
        &memonbooks::backend::transactions::transaction::Transaction::new(
            company_id,
            memonbooks::backend::transactions::transaction_types::TransactionType::OpeningBalance,
            prior_year_start,
        )
        .with_line(memonbooks::backend::transactions::transaction_lines::TransactionLine::debit(accounts["cash"], dec!(50000)))
        .with_line(memonbooks::backend::transactions::transaction_lines::TransactionLine::credit(accounts["owners_capital"], dec!(50000))),
    )?;
    println!("Opening balance posted: 50,000 cash against owner's capital");

    // -- 6. customers / vendors / products / warehouses / employees / bank accounts / terminals --
    let customer_ids = create_customers(&services.party, &mut rng, company_id)?;
    println!("Customers: {}", customer_ids.len());

    let vendor_ids = create_vendors(&services.party, &mut rng, company_id)?;
    println!("Vendors: {}", vendor_ids.len());

    let product_ids = create_products(&services.inventory, &mut rng, company_id)?;
    println!("Products: {}", product_ids.len());

    let warehouse_ids = create_warehouses(&services.inventory, &mut rng, company_id)?;
    println!("Warehouses: {}", warehouse_ids.len());

    let employee_ids = create_employees_with_salaries(&services.organization, &mut rng, company_id, data_range_start)?;
    println!("Employees: {} (each with a salary structure)", employee_ids.len());

    let bank_account_ids = create_bank_accounts(&services.banking, &mut rng, company_id, &accounts)?;
    println!("Bank accounts: {}", bank_account_ids.len());

    let terminal_ids = create_terminals(&services.sales, company_id, &warehouse_ids)?;
    println!("POS terminals: {}", terminal_ids.len());

    let expense_category_ids = create_expense_categories(&services.expense, company_id, &accounts)?;

    // -- 7. transactional records -------------------------------------
    let invoices_posted = create_invoices(&services.sales, &mut rng, company_id, &customer_ids, &accounts, data_range_start, data_range_end)?;
    println!("Invoices: {invoices_posted} (created and posted)");

    let bills_posted = create_bills(&services.purchase, &mut rng, company_id, &vendor_ids, &accounts, data_range_start, data_range_end)?;
    println!("Purchase bills: {bills_posted} (created and posted)");

    let pos_sales_posted = create_pos_sales(
        &services.sales,
        &mut rng,
        company_id,
        cashier.id.unwrap(),
        &terminal_ids,
        &product_ids,
        &accounts,
        data_range_start,
        data_range_end,
    )?;
    println!("POS sales: {pos_sales_posted} (across {POS_SESSION_COUNT} sessions)");

    let journal_entries_posted = create_journal_entries(&services.accounting, &mut rng, company_id, &accounts, data_range_start, data_range_end)?;
    println!("Journal entries: {journal_entries_posted}");

    let claims_created = create_expense_claims(&services.expense, &mut rng, company_id, &accounts, &expense_category_ids, data_range_start, data_range_end)?;
    println!("Expense claims: {claims_created} (mixed submitted/approved/rejected/reimbursed)");

    let payroll_runs_posted = create_payroll_runs(&services.organization, company_id, &employee_ids, &accounts, current_year_start)?;
    println!("Payroll runs: {payroll_runs_posted}");

    println!("\nDone. Log in with the bootstrap admin account and switch to \"{DEMO_COMPANY_NAME}\" via the Companies screen to browse the demo data.");
    Ok(())
}

// -- chart of accounts --------------------------------------------------

/// A full 100-account Pakistani-business chart of accounts. Ordered so
/// every account named as a `parent` key already exists earlier in the
/// list (`create_chart_of_accounts` looks parents up in the `accounts`
/// map it's still building, in one forward pass) -- e.g. `cash` (1) is
/// declared before `petty_cash`/`cash_in_transit` (5, 6), which parent
/// to it.
#[rustfmt::skip]
const CHART_OF_ACCOUNTS_SPECS: &[(&str, &str, &str, AccountType, Option<&str>)] = &[
    // -- Assets: cash & bank --------------------------------------------
    ("cash", "1000", "Cash", AccountType::Asset, None),
    ("bank_operating", "1010", "Operating Bank Account", AccountType::Asset, None),
    ("bank_payroll", "1020", "Payroll Bank Account", AccountType::Asset, None),
    ("bank_savings", "1030", "Savings Account", AccountType::Asset, None),
    ("petty_cash", "1040", "Petty Cash", AccountType::Asset, Some("cash")),
    ("cash_in_transit", "1050", "Cash in Transit", AccountType::Asset, Some("cash")),
    // Dedicated GL leaf per major Pakistani bank -- backs the 100 demo
    // BankAccount records (see PK_BANKS).
    ("bank_hbl", "1060", "Bank - Habib Bank Limited", AccountType::Asset, None),
    ("bank_ubl", "1061", "Bank - United Bank Limited", AccountType::Asset, None),
    ("bank_mcb", "1062", "Bank - MCB Bank Limited", AccountType::Asset, None),
    ("bank_meezan", "1063", "Bank - Meezan Bank Limited", AccountType::Asset, None),
    ("bank_alfalah", "1064", "Bank - Bank Alfalah Limited", AccountType::Asset, None),
    ("bank_abl", "1065", "Bank - Allied Bank Limited", AccountType::Asset, None),
    ("bank_nbp", "1066", "Bank - National Bank of Pakistan", AccountType::Asset, None),
    ("bank_faysal", "1067", "Bank - Faysal Bank Limited", AccountType::Asset, None),
    ("bank_askari", "1068", "Bank - Askari Bank Limited", AccountType::Asset, None),
    ("bank_alhabib", "1069", "Bank - Bank Al Habib Limited", AccountType::Asset, None),
    ("bank_scb", "1070", "Bank - Standard Chartered Bank (Pakistan) Limited", AccountType::Asset, None),
    ("bank_js", "1071", "Bank - JS Bank Limited", AccountType::Asset, None),
    ("bank_soneri", "1072", "Bank - Soneri Bank Limited", AccountType::Asset, None),
    ("bank_fcy_usd", "1073", "Bank - Foreign Currency Account (USD)", AccountType::Asset, None),
    // -- Assets: receivables & other current -----------------------------
    ("ar", "1100", "Accounts Receivable", AccountType::Asset, None),
    ("prepaid_expenses", "1110", "Prepaid Expenses", AccountType::Asset, None),
    ("advances_to_suppliers", "1120", "Advances to Suppliers", AccountType::Asset, None),
    ("advances_to_employees", "1130", "Advances to Employees", AccountType::Asset, None),
    ("security_deposits", "1140", "Security Deposits", AccountType::Asset, None),
    ("sales_tax_recoverable", "1150", "Sales Tax Recoverable (Input)", AccountType::Asset, None),
    ("wht_recoverable", "1160", "Withholding Tax Recoverable", AccountType::Asset, None),
    ("inventory", "1200", "Inventory", AccountType::Asset, None),
    ("inventory_raw_material", "1210", "Inventory - Raw Material", AccountType::Asset, Some("inventory")),
    ("inventory_finished_goods", "1220", "Inventory - Finished Goods", AccountType::Asset, Some("inventory")),
    ("inventory_wip", "1230", "Inventory - Work in Progress", AccountType::Asset, Some("inventory")),
    // -- Assets: fixed assets ----------------------------------------------
    ("land", "1300", "Land", AccountType::Asset, None),
    ("building", "1310", "Building", AccountType::Asset, None),
    ("plant_machinery", "1320", "Plant & Machinery", AccountType::Asset, None),
    ("furniture_fixtures", "1330", "Furniture & Fixtures", AccountType::Asset, None),
    ("vehicles", "1340", "Vehicles", AccountType::Asset, None),
    ("office_equipment", "1350", "Office Equipment", AccountType::Asset, None),
    ("computers", "1360", "Computers & IT Equipment", AccountType::Asset, None),
    ("accum_dep_building", "1370", "Accumulated Depreciation - Building", AccountType::Asset, Some("building")),
    ("accum_dep_vehicles", "1380", "Accumulated Depreciation - Vehicles", AccountType::Asset, Some("vehicles")),
    ("accum_dep_equipment", "1390", "Accumulated Depreciation - Equipment", AccountType::Asset, Some("office_equipment")),
    // -- Liabilities ---------------------------------------------------------
    ("ap", "2000", "Accounts Payable", AccountType::Liability, None),
    ("tax_payable", "2100", "Tax Payable", AccountType::Liability, None),
    ("sales_tax_payable", "2110", "Sales Tax Payable", AccountType::Liability, Some("tax_payable")),
    ("income_tax_payable", "2120", "Income Tax Payable", AccountType::Liability, Some("tax_payable")),
    ("wht_payable", "2130", "Withholding Tax Payable", AccountType::Liability, Some("tax_payable")),
    ("accrued_expenses", "2140", "Accrued Expenses", AccountType::Liability, None),
    ("salaries_payable", "2150", "Salaries Payable", AccountType::Liability, None),
    ("unearned_revenue", "2160", "Unearned Revenue", AccountType::Liability, None),
    ("deductions_payable", "2200", "Payroll Deductions Payable", AccountType::Liability, None),
    ("net_pay_payable", "2300", "Net Pay Payable", AccountType::Liability, None),
    ("short_term_loan", "2400", "Short-Term Loan", AccountType::Liability, None),
    ("long_term_loan", "2500", "Long-Term Loan", AccountType::Liability, None),
    ("lease_liability", "2510", "Lease Liability", AccountType::Liability, Some("long_term_loan")),
    ("provision_gratuity", "2520", "Provision for Gratuity", AccountType::Liability, None),
    // -- Equity ---------------------------------------------------------------
    ("owners_capital", "3000", "Owner's Capital", AccountType::Equity, None),
    ("owners_drawings", "3100", "Owner's Drawings", AccountType::Equity, Some("owners_capital")),
    ("retained_earnings", "3900", "Retained Earnings", AccountType::Equity, None),
    ("general_reserve", "3200", "General Reserve", AccountType::Equity, Some("retained_earnings")),
    // -- Revenue ----------------------------------------------------------------
    ("sales_revenue", "4000", "Sales Revenue", AccountType::Revenue, None),
    ("service_revenue", "4010", "Service Revenue", AccountType::Revenue, None),
    ("wholesale_revenue", "4020", "Wholesale Revenue", AccountType::Revenue, None),
    ("export_sales_revenue", "4030", "Export Sales Revenue", AccountType::Revenue, None),
    ("discount_received", "4040", "Discount Received", AccountType::Revenue, None),
    ("other_income", "4050", "Other Income", AccountType::Revenue, None),
    ("interest_income", "4060", "Interest Income", AccountType::Revenue, None),
    ("rental_income", "4070", "Rental Income", AccountType::Revenue, None),
    ("commission_income", "4080", "Commission Income", AccountType::Revenue, None),
    // -- Cost of goods sold ---------------------------------------------------
    ("cogs", "5000", "Cost of Goods Sold", AccountType::Expense, None),
    ("purchase_discount", "5010", "Purchase Discount", AccountType::Expense, Some("cogs")),
    ("freight_inward", "5020", "Freight Inward", AccountType::Expense, Some("cogs")),
    // -- Operating expenses ------------------------------------------------------
    ("rent_expense", "6000", "Rent Expense", AccountType::Expense, None),
    ("utilities_expense", "6010", "Utilities Expense", AccountType::Expense, None),
    ("office_supplies_expense", "6020", "Office Supplies Expense", AccountType::Expense, None),
    ("salaries_expense", "6030", "Salaries Expense", AccountType::Expense, None),
    ("marketing_expense", "6040", "Marketing Expense", AccountType::Expense, None),
    ("travel_expense", "6050", "Travel Expense", AccountType::Expense, None),
    ("insurance_expense", "6060", "Insurance Expense", AccountType::Expense, None),
    ("bank_fees_expense", "6070", "Bank Fees Expense", AccountType::Expense, None),
    ("depreciation_expense", "6080", "Depreciation Expense", AccountType::Expense, None),
    ("misc_expense", "6090", "Miscellaneous Expense", AccountType::Expense, None),
    ("telecom_expense", "6100", "Telephone & Internet Expense", AccountType::Expense, None),
    ("printing_stationery_expense", "6110", "Printing & Stationery", AccountType::Expense, None),
    ("repairs_maintenance_expense", "6120", "Repairs & Maintenance", AccountType::Expense, None),
    ("legal_professional_expense", "6130", "Legal & Professional Fees", AccountType::Expense, None),
    ("fuel_expense", "6140", "Fuel Expense", AccountType::Expense, None),
    ("vehicle_maintenance_expense", "6150", "Vehicle Maintenance", AccountType::Expense, None),
    ("entertainment_expense", "6160", "Entertainment Expense", AccountType::Expense, None),
    ("donations_expense", "6170", "Donations & Charity", AccountType::Expense, None),
    ("electricity_expense", "6180", "Electricity Expense", AccountType::Expense, Some("utilities_expense")),
    ("gas_expense", "6190", "Gas Expense", AccountType::Expense, Some("utilities_expense")),
    ("water_expense", "6200", "Water Expense", AccountType::Expense, Some("utilities_expense")),
    ("it_software_expense", "6210", "IT & Software Expense", AccountType::Expense, None),
    ("postage_courier_expense", "6220", "Postage & Courier", AccountType::Expense, None),
    ("audit_fee_expense", "6230", "Audit Fee", AccountType::Expense, None),
    ("zakat_expense", "6240", "Zakat Expense", AccountType::Expense, None),
    ("medical_expense", "6250", "Medical Expense", AccountType::Expense, None),
    ("bad_debts_expense", "6260", "Provision for Bad Debts", AccountType::Expense, None),
    ("commission_expense", "6270", "Commission Expense", AccountType::Expense, None),
    ("warehouse_rent_expense", "6280", "Warehouse Rent", AccountType::Expense, None),
];

fn create_chart_of_accounts(accounting: &AccountingService, company_id: i64) -> AppResult<HashMap<&'static str, i64>> {
    let mut accounts = HashMap::new();
    for (key, code, name, account_type, parent_key) in CHART_OF_ACCOUNTS_SPECS {
        let mut account = Account::new(company_id, *code, *name, *account_type);
        if let Some(parent_key) = parent_key {
            account.parent_account = Some(accounts[*parent_key]);
        }
        let created = accounting.create_account(&account)?;
        accounts.insert(*key, created.id.unwrap());
    }
    Ok(accounts)
}

fn create_tax_rates(tax: &TaxService, company_id: i64, accounts: &HashMap<&str, i64>) -> AppResult<usize> {
    let specs: &[(&str, &str, Decimal, TaxDirection)] = &[
        ("DEMO- Standard VAT (Output)", "tax_payable", dec!(15), TaxDirection::Output),
        ("DEMO- Reduced VAT (Output)", "tax_payable", dec!(5), TaxDirection::Output),
        ("DEMO- Zero-Rated (Output)", "tax_payable", dec!(0), TaxDirection::Output),
        ("DEMO- Standard VAT (Input)", "tax_payable", dec!(15), TaxDirection::Input),
    ];
    for (name, gl_key, rate, direction) in specs {
        tax.create_tax_rate(&memonbooks::backend::tax::tax_rates::TaxRate::new(company_id, *name, *rate, *direction, accounts[gl_key]))?;
    }
    Ok(specs.len())
}

// -- master data ----------------------------------------------------------

fn create_customers(party: &PartyService, rng: &mut impl Rng, company_id: i64) -> AppResult<Vec<i64>> {
    let names = unique_business_names(rng, CUSTOMER_COUNT, PK_CUSTOMER_NOUNS);
    let mut ids = Vec::with_capacity(CUSTOMER_COUNT);
    for (i, base_name) in names.into_iter().enumerate() {
        let name = format!("DEMO- {base_name}");
        let mut customer = Customer::new(company_id, &name);
        customer.email = Some(format!("ap+{}{}@example-demo.test", slugify_for_email(&name), i));
        customer.phone = Some(pk_phone_number(rng));
        customer.tax_id = Some(pk_ntn(2_000_000 + i as u32));
        // PKR 50,000 - 2,000,000 credit limit, stored in cents.
        customer.credit_limit = Decimal::new(rng.gen_range(5_000_000..200_000_000), 2);
        customer.payment_terms_days = *[15, 30, 45, 60].choose(rng).unwrap();
        customer.is_active = rng.gen_bool(0.85);
        let created = party.create_customer(&customer)?;
        ids.push(created.id.unwrap());
    }
    Ok(ids)
}

fn create_vendors(party: &PartyService, rng: &mut impl Rng, company_id: i64) -> AppResult<Vec<i64>> {
    let names = unique_business_names(rng, VENDOR_COUNT, PK_VENDOR_NOUNS);
    let mut ids = Vec::with_capacity(VENDOR_COUNT);
    for (i, base_name) in names.into_iter().enumerate() {
        let name = format!("DEMO- {base_name}");
        let mut vendor = Vendor::new(company_id, &name);
        vendor.email = Some(format!("ar+{}{}@example-demo.test", slugify_for_email(&name), i));
        vendor.phone = Some(pk_phone_number(rng));
        vendor.tax_id = Some(pk_ntn(1_000_000 + i as u32));
        vendor.payment_terms_days = *[15, 30, 45, 60].choose(rng).unwrap();
        vendor.is_active = rng.gen_bool(0.85);
        let created = party.create_vendor(&vendor)?;
        ids.push(created.id.unwrap());
    }
    Ok(ids)
}

fn create_products(inventory: &InventoryService, rng: &mut impl Rng, company_id: i64) -> AppResult<Vec<i64>> {
    let mut ids = Vec::with_capacity(PRODUCT_COUNT);
    for i in 0..PRODUCT_COUNT {
        let name = product_name(rng);
        let mut product = Product::new(company_id, format!("DEMO-SKU-{i:04}"), format!("DEMO- {name}"));
        product.cost_price = Decimal::new(rng.gen_range(500..15000), 2); // $5.00 - $150.00
        product.sale_price = product.cost_price * dec!(1.6); // simple markup, still realistic-looking
        let created = inventory.create_product(&product)?;
        ids.push(created.id.unwrap());
    }
    Ok(ids)
}

fn create_warehouses(inventory: &InventoryService, rng: &mut impl Rng, company_id: i64) -> AppResult<Vec<i64>> {
    // City x warehouse-type cartesian product, shuffled and truncated --
    // same "guaranteed-unique combos" approach as unique_business_names,
    // so 100 warehouses never collide on name.
    let mut combos: Vec<(&str, &str)> =
        PK_CITIES.iter().flat_map(|city| WAREHOUSE_TYPES.iter().map(move |kind| (*city, *kind))).collect();
    combos.shuffle(rng);
    combos.truncate(WAREHOUSE_COUNT);

    let mut ids = Vec::with_capacity(WAREHOUSE_COUNT);
    for (i, (city, kind)) in combos.into_iter().enumerate() {
        let name = format!("DEMO- {city} {kind}");
        let code = format!("DEMO-WH-{:04}", i + 1);
        let mut warehouse = Warehouse::new(company_id, &name, &code);
        let plot = rng.gen_range(1..999);
        let area = PK_INDUSTRIAL_AREAS.choose(rng).unwrap();
        warehouse.address = Some(format!("Plot {plot}, {area}, {city}, Pakistan"));
        warehouse.is_active = rng.gen_bool(0.9);
        let created = inventory.create_warehouse(&warehouse)?;
        ids.push(created.id.unwrap());
    }
    Ok(ids)
}

fn create_employees_with_salaries(organization: &OrganizationService, rng: &mut impl Rng, company_id: i64, hire_after: NaiveDate) -> AppResult<Vec<i64>> {
    let mut ids = Vec::with_capacity(EMPLOYEE_COUNT);
    for i in 0..EMPLOYEE_COUNT {
        let name = full_name(rng);
        let hire_date = random_date_between(rng, hire_after, hire_after + chrono::Days::new(180));
        let mut employee = Employee::new(company_id, format!("DEMO-EMP-{i:04}"), format!("DEMO- {name}"), hire_date);
        employee.email = Some(format!("hr+{}{}@example-demo.test", slugify_for_email(&name), i));
        let created = organization.create_employee(&employee)?;
        let employee_id = created.id.unwrap();

        let base = Decimal::new(rng.gen_range(300000..800000), 2); // $3,000 - $8,000/month base
        let mut structure = SalaryStructure::new(employee_id, hire_date);
        structure.components.push(SalaryComponent::addition("Base Salary", base));
        structure.components.push(SalaryComponent::addition("Housing Allowance", base * dec!(0.15)));
        structure.components.push(SalaryComponent::deduction("Income Tax Withholding", base * dec!(0.12)));
        organization.create_salary_structure(&structure)?;

        ids.push(employee_id);
    }
    Ok(ids)
}

fn create_bank_accounts(banking: &BankingService, rng: &mut impl Rng, company_id: i64, accounts: &HashMap<&str, i64>) -> AppResult<Vec<i64>> {
    let mut ids = Vec::with_capacity(BANK_ACCOUNT_COUNT);
    for i in 0..BANK_ACCOUNT_COUNT {
        let (display_name, gl_key, iban_bank_code, currency) = PK_BANKS[i % PK_BANKS.len()];
        let account_number = pk_iban(iban_bank_code, i);
        let mut bank_account = BankAccount::new(company_id, accounts[gl_key], format!("DEMO- {display_name}"), account_number, currency);
        bank_account.is_active = rng.gen_bool(0.92);
        let created = banking.create_bank_account(&bank_account)?;
        ids.push(created.id.unwrap());
    }
    Ok(ids)
}

fn create_terminals(sales: &SalesService, company_id: i64, warehouse_ids: &[i64]) -> AppResult<Vec<i64>> {
    let mut ids = Vec::with_capacity(TERMINAL_COUNT);
    for (i, warehouse_id) in warehouse_ids.iter().take(TERMINAL_COUNT).enumerate() {
        let created = sales.create_terminal(&Terminal::new(company_id, *warehouse_id, format!("DEMO- Register {}", i + 1)))?;
        ids.push(created.id.unwrap());
    }
    Ok(ids)
}

fn create_expense_categories(expense: &ExpenseService, company_id: i64, accounts: &HashMap<&str, i64>) -> AppResult<Vec<i64>> {
    let specs = [("DEMO- Travel", "travel_expense"), ("DEMO- Office Supplies", "office_supplies_expense"), ("DEMO- Client Entertainment", "misc_expense")];
    let mut ids = Vec::with_capacity(specs.len());
    for (name, gl_key) in specs {
        let created = expense.create_category(&ExpenseCategory::new(company_id, name, accounts[gl_key]))?;
        ids.push(created.id.unwrap());
    }
    Ok(ids)
}

// -- transactional records -----------------------------------------------

fn create_invoices(
    sales: &SalesService,
    rng: &mut impl Rng,
    company_id: i64,
    customer_ids: &[i64],
    accounts: &HashMap<&str, i64>,
    range_start: NaiveDate,
    range_end: NaiveDate,
) -> AppResult<usize> {
    let revenue_keys = [
        "sales_revenue", "service_revenue", "wholesale_revenue", "export_sales_revenue", "other_income",
        "interest_income", "rental_income", "commission_income",
    ];
    let mut posted = 0;
    for _ in 0..INVOICE_COUNT {
        let customer_id = *customer_ids.choose(rng).unwrap();
        let invoice_date = random_date_between(rng, range_start, range_end);
        let due_date = invoice_date + chrono::Days::new(30);

        let mut invoice = Invoice::new(company_id, customer_id, invoice_date, due_date);
        for _ in 0..rng.gen_range(1..=3) {
            let revenue_key = revenue_keys.choose(rng).unwrap();
            let quantity = Decimal::from(rng.gen_range(1..=10));
            let unit_price = Decimal::new(rng.gen_range(1000..50000), 2); // $10.00 - $500.00
            invoice.lines.push(InvoiceLine::new(accounts[*revenue_key], product_name(rng), quantity, unit_price));
        }

        let created = sales.create_invoice(&invoice)?;
        sales.post_invoice(created.id.unwrap(), accounts["ar"])?;
        posted += 1;
    }
    Ok(posted)
}

fn create_bills(
    purchase: &PurchaseService,
    rng: &mut impl Rng,
    company_id: i64,
    vendor_ids: &[i64],
    accounts: &HashMap<&str, i64>,
    range_start: NaiveDate,
    range_end: NaiveDate,
) -> AppResult<usize> {
    let expense_keys = [
        "cogs", "office_supplies_expense", "utilities_expense", "rent_expense", "marketing_expense",
        "repairs_maintenance_expense", "fuel_expense", "vehicle_maintenance_expense", "printing_stationery_expense",
        "it_software_expense", "legal_professional_expense", "warehouse_rent_expense", "freight_inward",
    ];
    let mut posted = 0;
    for _ in 0..BILL_COUNT {
        let vendor_id = *vendor_ids.choose(rng).unwrap();
        let bill_date = random_date_between(rng, range_start, range_end);
        let due_date = bill_date + chrono::Days::new(30);

        let mut bill = Bill::new(company_id, vendor_id, bill_date, due_date);
        for _ in 0..rng.gen_range(1..=3) {
            let expense_key = expense_keys.choose(rng).unwrap();
            let quantity = Decimal::from(rng.gen_range(1..=20));
            let unit_price = Decimal::new(rng.gen_range(500..30000), 2); // $5.00 - $300.00
            bill.lines.push(BillLine::new(accounts[*expense_key], product_name(rng), quantity, unit_price));
        }

        let created = purchase.create_bill(&bill)?;
        purchase.post_bill(created.id.unwrap(), accounts["ap"])?;
        posted += 1;
    }
    Ok(posted)
}

#[allow(clippy::too_many_arguments)]
fn create_pos_sales(
    sales: &SalesService,
    rng: &mut impl Rng,
    company_id: i64,
    cashier_user_id: i64,
    terminal_ids: &[i64],
    product_ids: &[i64],
    accounts: &HashMap<&str, i64>,
    range_start: NaiveDate,
    range_end: NaiveDate,
) -> AppResult<usize> {
    let mut posted = 0;
    for session_index in 0..POS_SESSION_COUNT {
        let terminal_id = terminal_ids[session_index % terminal_ids.len()];
        let opening_cash = dec!(200);
        let opened_at = random_date_between(rng, range_start, range_end).and_hms_opt(9, 0, 0).unwrap().and_utc();
        let session = memonbooks::backend::pos::sessions::PosSession {
            opened_at,
            ..PosSession::open(terminal_id, cashier_user_id, opening_cash)
        };
        let session = sales.open_session(&session)?;
        let sale_date = opened_at.date_naive();

        for _ in 0..POS_SALES_PER_SESSION {
            let product_id = *product_ids.choose(rng).unwrap();
            let quantity = Decimal::from(rng.gen_range(1..=5));
            let unit_price = Decimal::new(rng.gen_range(1000..30000), 2);
            let amount = quantity * unit_price;

            let mut sale = PosSale::new(company_id, terminal_id, session.id.unwrap(), sale_date);
            sale.lines.push(PosSaleLine::new(product_id, accounts["sales_revenue"], product_name(rng), quantity, unit_price));

            // Mostly single-tender, some split cash/card for realism.
            let payment_accounts: Vec<(PaymentMethod, i64)> = if rng.gen_bool(0.2) {
                let cash_part = (amount * dec!(0.5)).round_dp(2);
                let card_part = amount - cash_part;
                sale.payments.push(PosPayment::new(PaymentMethod::Cash, cash_part));
                sale.payments.push(PosPayment::new(PaymentMethod::Card, card_part));
                vec![(PaymentMethod::Cash, accounts["cash"]), (PaymentMethod::Card, accounts["bank_operating"])]
            } else if rng.gen_bool(0.5) {
                sale.payments.push(PosPayment::new(PaymentMethod::Cash, amount));
                vec![(PaymentMethod::Cash, accounts["cash"])]
            } else {
                sale.payments.push(PosPayment::new(PaymentMethod::Card, amount));
                vec![(PaymentMethod::Card, accounts["bank_operating"])]
            };

            sales.post_pos_sale(&sale, &payment_accounts)?;
            posted += 1;
        }
    }
    Ok(posted)
}

fn create_journal_entries(
    accounting: &AccountingService,
    rng: &mut impl Rng,
    company_id: i64,
    accounts: &HashMap<&str, i64>,
    range_start: NaiveDate,
    range_end: NaiveDate,
) -> AppResult<usize> {
    // (debit account, credit account, description, min cents, max cents)
    let templates: &[(&str, &str, &str, i64, i64)] = &[
        ("rent_expense", "cash", "DEMO- Monthly rent payment", 80000, 250000),
        ("utilities_expense", "cash", "DEMO- Utility bill payment", 5000, 40000),
        ("office_supplies_expense", "cash", "DEMO- Office supplies purchase", 2000, 30000),
        ("marketing_expense", "bank_operating", "DEMO- Marketing campaign spend", 10000, 500000),
        ("travel_expense", "bank_operating", "DEMO- Business travel", 5000, 200000),
        ("insurance_expense", "bank_operating", "DEMO- Insurance premium", 20000, 150000),
        ("bank_fees_expense", "bank_operating", "DEMO- Bank service charge", 500, 5000),
        ("telecom_expense", "cash", "DEMO- Phone and internet bill", 3000, 25000),
        ("misc_expense", "cash", "DEMO- Miscellaneous adjustment", 1000, 20000),
        ("cash", "sales_revenue", "DEMO- Cash sale adjustment", 5000, 100000),
        ("bank_operating", "owners_capital", "DEMO- Additional owner investment", 500000, 2000000),
        ("cogs", "inventory", "DEMO- Inventory usage adjustment", 5000, 80000),
        // -- Coverage for the rest of the 100-account chart: gives every
        // account a plausible, non-zero balance rather than leaving new
        // leaf accounts permanently at 0.00.
        ("petty_cash", "cash", "DEMO- Petty cash fund top-up", 5000, 20000),
        ("cash_in_transit", "cash", "DEMO- Cash deposit in transit to bank", 10000, 100000),
        ("bank_hbl", "owners_capital", "DEMO- Opening deposit - Habib Bank Limited", 200000, 1500000),
        ("bank_ubl", "owners_capital", "DEMO- Opening deposit - United Bank Limited", 200000, 1500000),
        ("bank_mcb", "owners_capital", "DEMO- Opening deposit - MCB Bank Limited", 200000, 1500000),
        ("bank_meezan", "owners_capital", "DEMO- Opening deposit - Meezan Bank Limited", 200000, 1500000),
        ("bank_alfalah", "owners_capital", "DEMO- Opening deposit - Bank Alfalah Limited", 200000, 1500000),
        ("bank_abl", "owners_capital", "DEMO- Opening deposit - Allied Bank Limited", 200000, 1500000),
        ("bank_nbp", "owners_capital", "DEMO- Opening deposit - National Bank of Pakistan", 200000, 1500000),
        ("bank_faysal", "owners_capital", "DEMO- Opening deposit - Faysal Bank Limited", 200000, 1500000),
        ("bank_askari", "owners_capital", "DEMO- Opening deposit - Askari Bank Limited", 200000, 1500000),
        ("bank_alhabib", "owners_capital", "DEMO- Opening deposit - Bank Al Habib Limited", 200000, 1500000),
        ("bank_scb", "owners_capital", "DEMO- Opening deposit - Standard Chartered Bank", 200000, 1500000),
        ("bank_js", "owners_capital", "DEMO- Opening deposit - JS Bank Limited", 200000, 1500000),
        ("bank_soneri", "owners_capital", "DEMO- Opening deposit - Soneri Bank Limited", 200000, 1500000),
        ("bank_fcy_usd", "owners_capital", "DEMO- Opening deposit - foreign currency account", 50000, 300000),
        ("prepaid_expenses", "bank_operating", "DEMO- Prepaid insurance premium", 20000, 150000),
        ("advances_to_suppliers", "bank_operating", "DEMO- Advance paid to supplier", 30000, 300000),
        ("advances_to_employees", "cash", "DEMO- Salary advance to employee", 10000, 50000),
        ("security_deposits", "bank_operating", "DEMO- Security deposit paid", 50000, 300000),
        ("sales_tax_recoverable", "ap", "DEMO- Input sales tax on purchases", 5000, 60000),
        ("wht_recoverable", "ar", "DEMO- Withholding tax deducted by customer", 2000, 30000),
        ("inventory_raw_material", "ap", "DEMO- Raw material purchase", 50000, 400000),
        ("inventory_finished_goods", "inventory_raw_material", "DEMO- Raw material converted to finished goods", 30000, 300000),
        ("inventory_wip", "ap", "DEMO- Work-in-progress inventory build-up", 20000, 150000),
        ("land", "bank_operating", "DEMO- Land purchase", 5000000, 20000000),
        ("building", "bank_operating", "DEMO- Building construction cost capitalized", 3000000, 15000000),
        ("plant_machinery", "bank_operating", "DEMO- Plant & machinery purchase", 1000000, 8000000),
        ("furniture_fixtures", "bank_operating", "DEMO- Furniture & fixtures purchase", 50000, 400000),
        ("vehicles", "bank_operating", "DEMO- Company vehicle purchase", 800000, 4000000),
        ("office_equipment", "bank_operating", "DEMO- Office equipment purchase", 30000, 300000),
        ("computers", "bank_operating", "DEMO- Computers & IT equipment purchase", 50000, 500000),
        ("depreciation_expense", "accum_dep_building", "DEMO- Depreciation - building", 10000, 80000),
        ("depreciation_expense", "accum_dep_vehicles", "DEMO- Depreciation - vehicles", 15000, 100000),
        ("depreciation_expense", "accum_dep_equipment", "DEMO- Depreciation - office equipment", 5000, 40000),
        ("cash", "sales_tax_payable", "DEMO- Sales tax collected on sale", 3000, 40000),
        ("misc_expense", "income_tax_payable", "DEMO- Income tax accrued", 10000, 80000),
        ("misc_expense", "wht_payable", "DEMO- Withholding tax accrued on payments", 3000, 30000),
        ("rent_expense", "accrued_expenses", "DEMO- Rent accrued, not yet paid", 20000, 100000),
        ("salaries_expense", "salaries_payable", "DEMO- Salaries accrued at period end", 100000, 400000),
        ("bank_operating", "unearned_revenue", "DEMO- Advance received from customer", 20000, 200000),
        ("bank_operating", "short_term_loan", "DEMO- Short-term bank loan proceeds", 500000, 3000000),
        ("bank_operating", "long_term_loan", "DEMO- Long-term loan proceeds", 1000000, 8000000),
        ("bank_operating", "lease_liability", "DEMO- Lease financing proceeds", 300000, 2000000),
        ("salaries_expense", "provision_gratuity", "DEMO- Provision for employee gratuity", 20000, 150000),
        ("owners_drawings", "cash", "DEMO- Owner's personal drawings", 10000, 100000),
        ("retained_earnings", "general_reserve", "DEMO- Transfer to general reserve", 50000, 500000),
        ("entertainment_expense", "cash", "DEMO- Client entertainment expense", 5000, 40000),
        ("donations_expense", "cash", "DEMO- Charitable donation", 5000, 50000),
        ("electricity_expense", "cash", "DEMO- Electricity bill payment", 8000, 60000),
        ("gas_expense", "cash", "DEMO- Gas bill payment", 3000, 30000),
        ("water_expense", "cash", "DEMO- Water bill payment", 1000, 15000),
        ("postage_courier_expense", "cash", "DEMO- Courier & postage charges", 1000, 15000),
        ("audit_fee_expense", "bank_operating", "DEMO- Annual audit fee", 30000, 150000),
        ("zakat_expense", "bank_operating", "DEMO- Zakat deduction", 10000, 100000),
        ("medical_expense", "cash", "DEMO- Employee medical reimbursement", 5000, 40000),
        ("bad_debts_expense", "ar", "DEMO- Provision for doubtful receivables", 10000, 100000),
        ("commission_expense", "bank_operating", "DEMO- Sales commission paid", 10000, 120000),
        ("purchase_discount", "ap", "DEMO- Purchase discount availed", 2000, 30000),
        ("discount_received", "ap", "DEMO- Early payment discount received", 2000, 25000),
    ];

    let mut posted = 0;
    for i in 0..JOURNAL_ENTRY_COUNT {
        let (debit_key, credit_key, description, min_cents, max_cents) = templates[i % templates.len()];
        let amount = Decimal::new(rng.gen_range(min_cents..max_cents), 2);
        let entry_date = random_date_between(rng, range_start, range_end);

        let entry = memonbooks::backend::accounting::journal_entries::JournalEntry::new(company_id, entry_date)
            .with_memo(description)
            .debit(accounts[debit_key], amount)
            .credit(accounts[credit_key], amount)
            .into_transaction();
        accounting.post_transaction(&entry)?;
        posted += 1;
    }
    Ok(posted)
}

fn create_expense_claims(
    expense: &ExpenseService,
    rng: &mut impl Rng,
    company_id: i64,
    accounts: &HashMap<&str, i64>,
    category_ids: &[i64],
    range_start: NaiveDate,
    range_end: NaiveDate,
) -> AppResult<usize> {
    let claimant_names: Vec<String> = (0..6).map(|_| format!("DEMO- {}", full_name(rng))).collect();

    let mut created_count = 0;
    for i in 0..EXPENSE_CLAIM_COUNT {
        let claimant = claimant_names.choose(rng).unwrap();
        let category_id = *category_ids.choose(rng).unwrap();
        let claim_date = random_date_between(rng, range_start, range_end);
        let amount = Decimal::new(rng.gen_range(2000..50000), 2); // $20 - $500

        let claim = ExpenseClaim::new(company_id, claimant, category_id, claim_date, amount, format!("DEMO- {}", ["Client dinner", "Taxi fare", "Hotel stay", "Conference ticket", "Office supplies", "Courier fee"].choose(rng).unwrap()));
        let created = expense.create_claim(&claim)?;
        let claim_id = created.id.unwrap();
        created_count += 1;

        // Roughly: 60% reimbursed, 20% approved-only, 10% rejected, 10% left submitted.
        match i % 10 {
            0..=5 => {
                expense.approve_claim(claim_id)?;
                let reimbursement_date = claim_date + chrono::Days::new(rng.gen_range(1..14));
                expense.reimburse_claim(claim_id, accounts["cash"], reimbursement_date)?;
            }
            6 | 7 => {
                expense.approve_claim(claim_id)?;
            }
            8 => {
                expense.reject_claim(claim_id)?;
            }
            _ => {} // left Submitted
        }
    }
    Ok(created_count)
}

fn create_payroll_runs(organization: &OrganizationService, company_id: i64, employee_ids: &[i64], accounts: &HashMap<&str, i64>, current_year_start: NaiveDate) -> AppResult<usize> {
    let mut posted = 0;
    for month_offset in 0..PAYROLL_RUN_COUNT {
        let period_start = current_year_start + chrono::Months::new(month_offset as u32);
        let period_end = period_start + chrono::Months::new(1) - chrono::Days::new(1);
        let payment_date = period_end + chrono::Days::new(3);

        let mut run = PayrollRun::new(company_id, period_start, period_end, payment_date);
        for employee_id in employee_ids {
            if let Some(structure) = organization.current_salary_structure(*employee_id, period_end)? {
                run.entries.push(PayrollEntry::from(&structure));
            }
        }
        if run.entries.is_empty() {
            continue;
        }

        organization.post_payroll_run(&run, accounts["salaries_expense"], accounts["deductions_payable"], accounts["net_pay_payable"])?;
        posted += 1;
    }
    Ok(posted)
}
