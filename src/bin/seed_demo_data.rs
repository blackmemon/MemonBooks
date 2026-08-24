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
const WAREHOUSE_COUNT: usize = 3;
const EMPLOYEE_COUNT: usize = 15;
const BANK_ACCOUNT_COUNT: usize = 3;
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

const BIZ_ADJECTIVES: &[&str] = &[
    "Golden", "Silver", "Blue", "Northern", "Southern", "Royal", "Crown", "Summit", "Metro", "Pacific", "Atlantic",
    "Central", "Union", "Pioneer", "Heritage", "Falcon", "Cedar", "Maple", "River", "Valley", "Star", "Sunrise",
    "Horizon", "Coastal", "Prime", "Apex", "Harbor", "Grand", "Evergreen", "Meridian",
];
const BIZ_NOUNS: &[&str] = &[
    "Trading", "Supply", "Mercantile", "Distributors", "Wholesale", "Imports", "Exports", "Logistics", "Commerce",
    "Market", "Retail", "Goods", "Ventures", "Enterprises", "Industries", "Holdings",
];
const BIZ_SUFFIXES: &[&str] = &["Co", "Ltd", "Inc", "LLC", "Group", "Partners"];

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

fn business_name(rng: &mut impl Rng) -> String {
    format!(
        "{} {} {}",
        BIZ_ADJECTIVES.choose(rng).unwrap(),
        BIZ_NOUNS.choose(rng).unwrap(),
        BIZ_SUFFIXES.choose(rng).unwrap()
    )
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

    let warehouse_ids = create_warehouses(&services.inventory, company_id)?;
    println!("Warehouses: {}", warehouse_ids.len());

    let employee_ids = create_employees_with_salaries(&services.organization, &mut rng, company_id, data_range_start)?;
    println!("Employees: {} (each with a salary structure)", employee_ids.len());

    let bank_account_ids = create_bank_accounts(&services.banking, company_id, &accounts)?;
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

fn create_chart_of_accounts(accounting: &AccountingService, company_id: i64) -> AppResult<HashMap<&'static str, i64>> {
    let specs: &[(&str, &str, &str, AccountType)] = &[
        ("cash", "1000", "Cash", AccountType::Asset),
        ("bank_operating", "1010", "Operating Bank Account", AccountType::Asset),
        ("bank_payroll", "1020", "Payroll Bank Account", AccountType::Asset),
        ("bank_savings", "1030", "Savings Account", AccountType::Asset),
        ("ar", "1100", "Accounts Receivable", AccountType::Asset),
        ("inventory", "1200", "Inventory", AccountType::Asset),
        ("ap", "2000", "Accounts Payable", AccountType::Liability),
        ("tax_payable", "2100", "Tax Payable", AccountType::Liability),
        ("deductions_payable", "2200", "Payroll Deductions Payable", AccountType::Liability),
        ("net_pay_payable", "2300", "Net Pay Payable", AccountType::Liability),
        ("owners_capital", "3000", "Owner's Capital", AccountType::Equity),
        ("retained_earnings", "3900", "Retained Earnings", AccountType::Equity),
        ("sales_revenue", "4000", "Sales Revenue", AccountType::Revenue),
        ("service_revenue", "4010", "Service Revenue", AccountType::Revenue),
        ("wholesale_revenue", "4020", "Wholesale Revenue", AccountType::Revenue),
        ("cogs", "5000", "Cost of Goods Sold", AccountType::Expense),
        ("rent_expense", "6000", "Rent Expense", AccountType::Expense),
        ("utilities_expense", "6010", "Utilities Expense", AccountType::Expense),
        ("office_supplies_expense", "6020", "Office Supplies Expense", AccountType::Expense),
        ("salaries_expense", "6030", "Salaries Expense", AccountType::Expense),
        ("marketing_expense", "6040", "Marketing Expense", AccountType::Expense),
        ("travel_expense", "6050", "Travel Expense", AccountType::Expense),
        ("insurance_expense", "6060", "Insurance Expense", AccountType::Expense),
        ("bank_fees_expense", "6070", "Bank Fees Expense", AccountType::Expense),
        ("misc_expense", "6090", "Miscellaneous Expense", AccountType::Expense),
        ("telecom_expense", "6100", "Telephone & Internet Expense", AccountType::Expense),
    ];

    let mut accounts = HashMap::new();
    for (key, code, name, account_type) in specs {
        let created = accounting.create_account(&Account::new(company_id, *code, *name, *account_type))?;
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
    let mut ids = Vec::with_capacity(CUSTOMER_COUNT);
    for i in 0..CUSTOMER_COUNT {
        let name = format!("DEMO- {}", business_name(rng));
        let mut customer = Customer::new(company_id, &name);
        customer.email = Some(format!("ap+{}{}@example-demo.test", slugify_for_email(&name), i));
        customer.payment_terms_days = *[15, 30, 45, 60].choose(rng).unwrap();
        let created = party.create_customer(&customer)?;
        ids.push(created.id.unwrap());
    }
    Ok(ids)
}

fn create_vendors(party: &PartyService, rng: &mut impl Rng, company_id: i64) -> AppResult<Vec<i64>> {
    let mut ids = Vec::with_capacity(VENDOR_COUNT);
    for i in 0..VENDOR_COUNT {
        let name = format!("DEMO- {}", business_name(rng));
        let mut vendor = Vendor::new(company_id, &name);
        vendor.email = Some(format!("ar+{}{}@example-demo.test", slugify_for_email(&name), i));
        vendor.payment_terms_days = *[15, 30, 45, 60].choose(rng).unwrap();
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

fn create_warehouses(inventory: &InventoryService, company_id: i64) -> AppResult<Vec<i64>> {
    let specs = [("DEMO- Main Warehouse", "MAIN"), ("DEMO- North Branch", "NORTH"), ("DEMO- South Branch", "SOUTH")];
    let mut ids = Vec::with_capacity(WAREHOUSE_COUNT);
    for (name, code) in specs.iter().take(WAREHOUSE_COUNT) {
        let created = inventory.create_warehouse(&Warehouse::new(company_id, *name, *code))?;
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

fn create_bank_accounts(banking: &BankingService, company_id: i64, accounts: &HashMap<&str, i64>) -> AppResult<Vec<i64>> {
    let specs = [("DEMO- Operating Account", "1000200001", "bank_operating"), ("DEMO- Payroll Account", "1000200002", "bank_payroll"), ("DEMO- Savings Account", "1000200003", "bank_savings")];
    let mut ids = Vec::with_capacity(BANK_ACCOUNT_COUNT);
    for (name, number, gl_key) in specs.iter().take(BANK_ACCOUNT_COUNT) {
        let created = banking.create_bank_account(&BankAccount::new(company_id, accounts[gl_key], *name, *number, "USD"))?;
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
    let revenue_keys = ["sales_revenue", "service_revenue", "wholesale_revenue"];
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
    let expense_keys = ["cogs", "office_supplies_expense", "utilities_expense", "rent_expense", "marketing_expense"];
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
