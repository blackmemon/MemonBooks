// Application entry point -- Iced application foundation
// (Development Order step 1), now wired to real views module-by-module
// (Development Order step 19).
//
// Views route all data access through backend/services/*.rs, per
// Architecture Rules (Section 3: "Iced View -> Service -> Repository ->
// Database"). This file itself is the composition root -- it still
// talks to repositories directly for one-time bootstrap (creating the
// default company/admin before any session exists to route a service
// call through), which is the same exception every service's own
// constructor makes.

use std::path::PathBuf;

use chrono::{Datelike, Utc};

use iced::widget::{button, column, container, pick_list, row, text};
use iced::{Element, Length, Task};

use memonbooks::backend::accounting::fiscal_periods::FiscalPeriod;
use memonbooks::backend::core::auth::passwords::hash_password;
use memonbooks::backend::core::logging;
use memonbooks::backend::core::roles::Role;
use memonbooks::backend::core::users::User;
use memonbooks::backend::organization::companies::Company;
use memonbooks::backend::repositories::accounting_repository::AccountingRepository;
use memonbooks::backend::repositories::organization_repository::OrganizationRepository;
use memonbooks::backend::repositories::user_repository::UserRepository;
use memonbooks::configs::app::AppConfig;
use memonbooks::configs::theme_preference;
use memonbooks::database::database::Database;
use memonbooks::frontend::components::common::sidebar::{self, NavEntry};
use memonbooks::frontend::styles::button as button_style;
use memonbooks::frontend::styles::colors::Theme;
use memonbooks::frontend::styles::spacing;
use memonbooks::frontend::styles::typography;
use memonbooks::frontend::styles::dropdown;
use memonbooks::frontend::views::accounting::{chart_of_accounts_view, fixed_assets_view, journal_entries_view, journal_entry_view, tax_rates_view, trial_balance_view};
use memonbooks::frontend::views::auth::{login_view, password_change_view, switch_user_view};
use memonbooks::frontend::views::dashboard::dashboard_view;
use memonbooks::frontend::views::hr::{employees_view, expenses_view};
use memonbooks::frontend::views::inventory::{bills_view, products_view, vendors_view, warehouses_view};
use memonbooks::frontend::views::pos::pos_view;
use memonbooks::frontend::views::reports::{bank_accounts_view, reports_view};
use memonbooks::frontend::views::sales::{customers_view, invoices_view};
use memonbooks::frontend::views::settings::{companies_view, settings_view, users_view};
use memonbooks::frontend::views::Ctx;

/// First run: no companies exist yet. Creates one and an open fiscal
/// period covering the current year, so the rest of the app has
/// somewhere to post to immediately. Not a real onboarding wizard --
/// that's future work -- but enough for the app to be usable today.
fn ensure_default_company(db: &Database) -> i64 {
    let org_repo = OrganizationRepository::new(db.clone());
    if let Ok(companies) = org_repo.list_companies() {
        if let Some(company) = companies.into_iter().next() {
            return company.id.expect("persisted company must have an id");
        }
    }

    let company = org_repo
        .create_company(&Company::new("My Company", "USD"))
        .expect("failed to create default company");
    let company_id = company.id.expect("persisted company must have an id");

    let today = Utc::now().date_naive();
    let year_start = chrono::NaiveDate::from_ymd_opt(today.year(), 1, 1).unwrap_or(today);
    let year_end = chrono::NaiveDate::from_ymd_opt(today.year(), 12, 31).unwrap_or(today);
    let accounting_repo = AccountingRepository::new(db.clone());
    accounting_repo
        .create_fiscal_period(&FiscalPeriod::new(company_id, format!("FY{}", today.year()), year_start, year_end))
        .expect("failed to create default fiscal period");

    company_id
}

/// First run: no users exist yet, so nobody could log in at all. Creates
/// an Administrator role with every known permission and a single admin
/// account with a freshly-generated password, logged once (not printed
/// to a UI) so it isn't a silently-shipped hardcoded credential -- the
/// operator is expected to read it from the log and change it.
fn ensure_default_admin(db: &Database) {
    use rand::Rng;

    let user_repo = UserRepository::new(db.clone());
    if matches!(user_repo.list_roles(), Ok(roles) if !roles.is_empty()) {
        return;
    }

    let mut admin_role = Role::new("Administrator");
    admin_role.permissions = memonbooks::backend::core::permissions::KNOWN_PERMISSIONS
        .iter()
        .map(|p| p.to_string())
        .collect();
    let role = user_repo.create_role(&admin_role).expect("failed to create default administrator role");

    let password: String = rand::thread_rng()
        .sample_iter(rand::distributions::Alphanumeric)
        .take(16)
        .map(char::from)
        .collect();
    let password_hash = hash_password(&password).expect("failed to hash default admin password");
    let mut user = User::new("admin", "Administrator", password_hash, role.id.expect("persisted role must have an id"));
    user.must_change_password = true;
    user_repo.create_user(&user).expect("failed to create default admin user");

    log::warn!("No users existed yet -- created a default administrator account.");
    log::warn!("Username: admin");
    log::warn!("Password: {password}");
    log::warn!("Log in and change this password as soon as possible.");
}

/// OS-level light/dark detection, consulted only at launch and only
/// when no manual preference has been saved yet (see `theme_preference`).
///
/// Live OS-level switching (the app updating the moment Windows/macOS/
/// Linux display settings change, with no restart) was attempted and
/// deliberately not shipped: `dark-light`, the one maintained
/// cross-platform crate for this, only offers a one-shot `detect()` --
/// there's no bundled change-subscription API. Getting live updates
/// would mean either polling `detect()` on a timer (wasteful, and
/// laggy for something that should feel instant) or hand-rolling three
/// separate platform-specific event subscriptions (a Windows registry
/// key-change notification, a macOS `NSApplication.effectiveAppearance`
/// KVO observer, and a Linux `org.freedesktop.appearance` D-Bus signal)
/// with no shared abstraction and no way to test two of the three on a
/// Windows dev machine. Detect-at-launch plus the sidebar's manual
/// sun/moon toggle (persisted via `theme_preference`) is the supported
/// path instead.
fn detect_os_theme() -> Theme {
    match dark_light::detect() {
        dark_light::Mode::Dark => Theme::Dark,
        dark_light::Mode::Light | dark_light::Mode::Default => Theme::Light,
    }
}

fn main() -> iced::Result {
    let config = AppConfig::load().expect("failed to resolve application configuration");
    logging::init(&config).expect("failed to initialize logging");

    log::info!("MemonBooks starting in {} mode", config.environment);
    log::info!("database path: {}", config.database_path.display());

    let database = Database::open(&config.database_path).expect("failed to open database");
    database.run_migrations(&memonbooks::all_migrations()).expect("failed to run database migrations");

    let company_id = ensure_default_company(&database);
    ensure_default_admin(&database);

    let ctx = Ctx {
        db: database,
        company_id,
        current_user_id: None,
        export_dir: config.data_dir.join("exports"),
    };

    let theme_path = config.data_dir.join("theme_preference.txt");
    let theme = theme_preference::load(&theme_path).unwrap_or_else(detect_os_theme);

    iced::application("MemonBooks", MemonBooks::update, MemonBooks::view)
        .window(iced::window::Settings {
            size: iced::Size::new(1280.0, 800.0),
            min_size: Some(iced::Size::new(960.0, 600.0)),
            position: iced::window::Position::Centered,
            resizable: true,
            ..iced::window::Settings::default()
        })
        .run_with(move || (MemonBooks::new(ctx, theme, theme_path), Task::none()))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Route {
    Dashboard,
    Maintain,
    JournalEntry,
    JournalEntries,
    TrialBalance,
    FixedAssets,
    TaxRates,
    Transactions,
    Products,
    Employees,
    Expenses,
    Pos,
    Reports,
    Settings,
    Companies,
    SwitchUser,
    Users,
}

const NAV_ITEMS: &[(Route, &str)] = &[
    (Route::Dashboard, "Dashboard"),
    (Route::Maintain, "Maintain"),
    (Route::JournalEntry, "New Journal Entry"),
    (Route::JournalEntries, "Journal Entries"),
    (Route::TrialBalance, "Trial Balance"),
    (Route::FixedAssets, "Fixed Assets"),
    (Route::TaxRates, "Tax Rates"),
    (Route::Transactions, "Transactions"),
    (Route::Products, "Products"),
    (Route::Employees, "Employees"),
    (Route::Expenses, "Expenses"),
    (Route::Pos, "Point of Sale"),
    (Route::Reports, "Reports"),
    (Route::Settings, "Settings"),
    (Route::Companies, "Companies"),
    (Route::SwitchUser, "Switch User"),
    (Route::Users, "Users"),
];

/// The five record types grouped behind the sidebar's "Maintain" entry.
/// Each one keeps its own already-built `PageHeader` + `Toolbar` +
/// `DataTable` view untouched -- this only controls which one is shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MaintainSection {
    ChartOfAccounts,
    Vendors,
    Warehouses,
    BankAccounts,
    Customers,
}

const MAINTAIN_SECTIONS: [MaintainSection; 5] =
    [MaintainSection::ChartOfAccounts, MaintainSection::Vendors, MaintainSection::Warehouses, MaintainSection::BankAccounts, MaintainSection::Customers];

impl std::fmt::Display for MaintainSection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            MaintainSection::ChartOfAccounts => "Chart of Account",
            MaintainSection::Vendors => "Vendor",
            MaintainSection::Warehouses => "Warehouse",
            MaintainSection::BankAccounts => "Bank Accounts",
            MaintainSection::Customers => "Customers",
        })
    }
}

/// Which side of the ledger the sidebar's "Transactions" screen is
/// currently showing. Each side's document-type dropdown only lists
/// document types that actually have a working view today: Purchase
/// Orders and Quotations have backend models (`backend::purchases::
/// purchase_orders`, `backend::sales::quotations`) but no frontend view
/// yet, so `documents()` deliberately omits them rather than pointing
/// at nothing -- that's a gap for a future batch, not a bug here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TransactionSide {
    Purchase,
    Sale,
}

impl TransactionSide {
    fn documents(self) -> &'static [TransactionDocument] {
        match self {
            TransactionSide::Purchase => &[TransactionDocument::Bills],
            TransactionSide::Sale => &[TransactionDocument::Invoices],
        }
    }

    fn default_document(self) -> TransactionDocument {
        self.documents()[0]
    }
}

impl std::fmt::Display for TransactionSide {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            TransactionSide::Purchase => "Purchase",
            TransactionSide::Sale => "Sale",
        })
    }
}

/// One document-type view rendered inside "Transactions". Bills is
/// purchase-side, Invoices is sale-side -- see `TransactionSide::
/// documents`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TransactionDocument {
    Bills,
    Invoices,
}

impl std::fmt::Display for TransactionDocument {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            TransactionDocument::Bills => "Bills",
            TransactionDocument::Invoices => "Invoices",
        })
    }
}

/// Routes rendered in the sidebar's secondary section (below the divider)
/// instead of the primary nav list.
fn is_secondary_route(route: Route) -> bool {
    matches!(route, Route::Settings | Route::Companies | Route::SwitchUser | Route::Users)
}

struct MemonBooks {
    ctx: Ctx,
    session: Option<User>,
    /// Cached at login/switch-user time (not recomputed in `view()`, to
    /// keep it a pure render function) -- whether the signed-in user's
    /// role currently grants `users.manage`. Filters the "Users" nav
    /// entry out of the sidebar when false; the real security boundary
    /// is still `AuthService::require_permission` inside every mutating
    /// call `users_view` makes, not this cosmetic gate.
    can_manage_users: bool,
    theme: Theme,
    /// Where the manual theme toggle's choice is persisted -- a plain
    /// text file under the app's data directory, not the database (this
    /// is a device-level UI preference, not company data).
    theme_path: PathBuf,
    route: Route,
    /// Which of the five `MaintainSection`s the "Maintain" area currently
    /// shows -- independent of `route`, so it survives navigating away
    /// and back.
    maintain_selected: MaintainSection,
    /// Which side/document the "Transactions" area currently shows --
    /// independent of `route`, same reasoning as `maintain_selected`.
    transaction_side: TransactionSide,
    transaction_document: TransactionDocument,
    sidebar_collapsed: bool,
    login: login_view::State,
    password_change: password_change_view::State,
    dashboard: dashboard_view::State,
    chart_of_accounts: chart_of_accounts_view::State,
    journal_entry: journal_entry_view::State,
    journal_entries: journal_entries_view::State,
    trial_balance: trial_balance_view::State,
    fixed_assets: fixed_assets_view::State,
    tax_rates: tax_rates_view::State,
    customers: customers_view::State,
    invoices: invoices_view::State,
    products: products_view::State,
    warehouses: warehouses_view::State,
    vendors: vendors_view::State,
    bills: bills_view::State,
    bank_accounts: bank_accounts_view::State,
    employees: employees_view::State,
    expenses: expenses_view::State,
    pos: pos_view::State,
    reports: reports_view::State,
    settings: settings_view::State,
    companies: companies_view::State,
    switch_user: switch_user_view::State,
    users: users_view::State,
}

#[derive(Debug, Clone)]
enum Message {
    Navigate(Route),
    MaintainSelect(MaintainSection),
    TransactionSideSelect(TransactionSide),
    TransactionDocumentSelect(TransactionDocument),
    ToggleSidebar,
    ToggleTheme,
    Logout,
    Login(login_view::Message),
    PasswordChange(password_change_view::Message),
    Dashboard(dashboard_view::Message),
    ChartOfAccounts(chart_of_accounts_view::Message),
    JournalEntry(journal_entry_view::Message),
    JournalEntries(journal_entries_view::Message),
    TrialBalance(trial_balance_view::Message),
    FixedAssets(fixed_assets_view::Message),
    TaxRates(tax_rates_view::Message),
    Customers(customers_view::Message),
    Invoices(invoices_view::Message),
    Products(products_view::Message),
    Warehouses(warehouses_view::Message),
    Vendors(vendors_view::Message),
    Bills(bills_view::Message),
    BankAccounts(bank_accounts_view::Message),
    Employees(employees_view::Message),
    Expenses(expenses_view::Message),
    Pos(pos_view::Message),
    Reports(reports_view::Message),
    Settings(settings_view::Message),
    Companies(companies_view::Message),
    SwitchUser(switch_user_view::Message),
    Users(users_view::Message),
}

impl MemonBooks {
    fn new(ctx: Ctx, theme: Theme, theme_path: PathBuf) -> Self {
        Self {
            ctx,
            session: None,
            can_manage_users: false,
            theme,
            theme_path,
            route: Route::Dashboard,
            maintain_selected: MaintainSection::ChartOfAccounts,
            transaction_side: TransactionSide::Purchase,
            transaction_document: TransactionDocument::Bills,
            sidebar_collapsed: false,
            login: login_view::State::default(),
            password_change: password_change_view::State::default(),
            dashboard: dashboard_view::State::default(),
            chart_of_accounts: chart_of_accounts_view::State::default(),
            journal_entry: journal_entry_view::State::default(),
            journal_entries: journal_entries_view::State::default(),
            trial_balance: trial_balance_view::State::default(),
            fixed_assets: fixed_assets_view::State::default(),
            tax_rates: tax_rates_view::State::default(),
            customers: customers_view::State::default(),
            invoices: invoices_view::State::default(),
            products: products_view::State::default(),
            warehouses: warehouses_view::State::default(),
            vendors: vendors_view::State::default(),
            bills: bills_view::State::default(),
            bank_accounts: bank_accounts_view::State::default(),
            employees: employees_view::State::default(),
            expenses: expenses_view::State::default(),
            pos: pos_view::State::default(),
            reports: reports_view::State::default(),
            settings: settings_view::State::default(),
            companies: companies_view::State::default(),
            switch_user: switch_user_view::State::default(),
            users: users_view::State::default(),
        }
    }

    /// Loads whichever screen's data the route needs, freshly, each time
    /// navigation lands on it.
    fn load_route(&mut self, route: Route) {
        match route {
            Route::Dashboard => self.dashboard = dashboard_view::load(&self.ctx),
            Route::Maintain => self.load_maintain_section(self.maintain_selected),
            Route::JournalEntry => self.journal_entry = journal_entry_view::load(&self.ctx),
            Route::JournalEntries => self.journal_entries = journal_entries_view::load(&self.ctx),
            Route::TrialBalance => self.trial_balance = trial_balance_view::load(&self.ctx),
            Route::FixedAssets => self.fixed_assets = fixed_assets_view::load(&self.ctx),
            Route::TaxRates => self.tax_rates = tax_rates_view::load(&self.ctx),
            Route::Transactions => self.load_transaction_document(self.transaction_document),
            Route::Products => self.products = products_view::load(&self.ctx),
            Route::Employees => self.employees = employees_view::load(&self.ctx),
            Route::Expenses => self.expenses = expenses_view::load(&self.ctx),
            Route::Pos => self.pos = pos_view::load(&self.ctx),
            Route::Reports => self.reports = reports_view::load(&self.ctx),
            Route::Settings => self.settings = settings_view::load(&self.ctx),
            Route::Companies => self.companies = companies_view::load(&self.ctx),
            Route::SwitchUser => self.switch_user = switch_user_view::load(&self.ctx),
            Route::Users => self.users = users_view::load(&self.ctx),
        }
    }

    /// Loads fresh data for one `MaintainSection` -- shared by
    /// `load_route` (landing on "Maintain") and `Message::MaintainSelect`
    /// (switching sections while already there).
    fn load_maintain_section(&mut self, section: MaintainSection) {
        match section {
            MaintainSection::ChartOfAccounts => self.chart_of_accounts = chart_of_accounts_view::load(&self.ctx),
            MaintainSection::Vendors => self.vendors = vendors_view::load(&self.ctx),
            MaintainSection::Warehouses => self.warehouses = warehouses_view::load(&self.ctx),
            MaintainSection::BankAccounts => self.bank_accounts = bank_accounts_view::load(&self.ctx),
            MaintainSection::Customers => self.customers = customers_view::load(&self.ctx),
        }
    }

    /// Loads fresh data for one `TransactionDocument` -- shared by
    /// `load_route` (landing on "Transactions") and both
    /// `Message::TransactionSideSelect`/`TransactionDocumentSelect`
    /// (switching while already there).
    fn load_transaction_document(&mut self, document: TransactionDocument) {
        match document {
            TransactionDocument::Bills => self.bills = bills_view::load(&self.ctx),
            TransactionDocument::Invoices => self.invoices = invoices_view::load(&self.ctx),
        }
    }

    fn update(&mut self, message: Message) {
        match message {
            Message::Navigate(route) => {
                self.route = route;
                self.load_route(route);
            }
            Message::MaintainSelect(section) => {
                self.maintain_selected = section;
                self.load_maintain_section(section);
            }
            Message::TransactionSideSelect(side) => {
                self.transaction_side = side;
                self.transaction_document = side.default_document();
                self.load_transaction_document(self.transaction_document);
            }
            Message::TransactionDocumentSelect(document) => {
                self.transaction_document = document;
                self.load_transaction_document(document);
            }
            Message::ToggleSidebar => self.sidebar_collapsed = !self.sidebar_collapsed,
            Message::ToggleTheme => {
                self.theme = match self.theme {
                    Theme::Light => Theme::Dark,
                    Theme::Dark => Theme::Light,
                };
                if let Err(err) = theme_preference::save(&self.theme_path, self.theme) {
                    log::warn!("failed to persist theme preference: {err}");
                }
            }
            Message::Logout => {
                self.session = None;
                self.ctx.current_user_id = None;
                self.login = login_view::State::default();
            }
            Message::Login(inner) => {
                if let login_view::Event::LoggedIn(user) = login_view::update(&mut self.login, inner, &self.ctx) {
                    self.ctx.current_user_id = user.id;
                    self.can_manage_users = memonbooks::backend::services::auth_service::AuthService::new(self.ctx.db.clone())
                        .has_permission(&user, "users.manage")
                        .unwrap_or(false);
                    self.session = Some(user);
                    self.login = login_view::State::default();
                    self.route = Route::Dashboard;
                    self.load_route(Route::Dashboard);
                }
            }
            Message::PasswordChange(inner) => {
                let Some(user) = self.session.clone() else { return };
                if let password_change_view::Event::Changed(updated_user) = password_change_view::update(&mut self.password_change, inner, &self.ctx, &user) {
                    self.session = Some(updated_user);
                    self.password_change = password_change_view::State::default();
                    self.route = Route::Dashboard;
                    self.load_route(Route::Dashboard);
                }
            }
            Message::Dashboard(inner) => dashboard_view::update(&mut self.dashboard, inner, &self.ctx),
            Message::ChartOfAccounts(inner) => chart_of_accounts_view::update(&mut self.chart_of_accounts, inner, &self.ctx),
            Message::JournalEntry(inner) => journal_entry_view::update(&mut self.journal_entry, inner, &self.ctx),
            Message::JournalEntries(inner) => journal_entries_view::update(&mut self.journal_entries, inner, &self.ctx),
            Message::TrialBalance(inner) => trial_balance_view::update(&mut self.trial_balance, inner, &self.ctx),
            Message::FixedAssets(inner) => fixed_assets_view::update(&mut self.fixed_assets, inner, &self.ctx),
            Message::TaxRates(inner) => tax_rates_view::update(&mut self.tax_rates, inner, &self.ctx),
            Message::Customers(inner) => customers_view::update(&mut self.customers, inner, &self.ctx),
            Message::Invoices(inner) => invoices_view::update(&mut self.invoices, inner, &self.ctx),
            Message::Products(inner) => products_view::update(&mut self.products, inner, &self.ctx),
            Message::Warehouses(inner) => warehouses_view::update(&mut self.warehouses, inner, &self.ctx),
            Message::Vendors(inner) => vendors_view::update(&mut self.vendors, inner, &self.ctx),
            Message::Bills(inner) => bills_view::update(&mut self.bills, inner, &self.ctx),
            Message::BankAccounts(inner) => bank_accounts_view::update(&mut self.bank_accounts, inner, &self.ctx),
            Message::Employees(inner) => employees_view::update(&mut self.employees, inner, &self.ctx),
            Message::Expenses(inner) => expenses_view::update(&mut self.expenses, inner, &self.ctx),
            Message::Pos(inner) => pos_view::update(&mut self.pos, inner, &self.ctx),
            Message::Reports(inner) => reports_view::update(&mut self.reports, inner, &self.ctx),
            Message::Settings(inner) => settings_view::update(&mut self.settings, inner, &self.ctx),
            Message::Companies(inner) => {
                if let companies_view::Event::Switched(company_id) = companies_view::update(&mut self.companies, inner, &self.ctx) {
                    self.ctx.company_id = company_id;
                    self.route = Route::Dashboard;
                    self.load_route(Route::Dashboard);
                }
            }
            Message::SwitchUser(inner) => {
                if let switch_user_view::Event::SwitchedTo(user) = switch_user_view::update(&mut self.switch_user, inner, &self.ctx) {
                    self.ctx.current_user_id = user.id;
                    self.can_manage_users = memonbooks::backend::services::auth_service::AuthService::new(self.ctx.db.clone())
                        .has_permission(&user, "users.manage")
                        .unwrap_or(false);
                    self.session = Some(user);
                    self.switch_user = switch_user_view::State::default();
                    self.route = Route::Dashboard;
                    self.load_route(Route::Dashboard);
                }
            }
            Message::Users(inner) => {
                let Some(user) = self.session.clone() else { return };
                users_view::update(&mut self.users, inner, &self.ctx, &user);
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let Some(user) = &self.session else {
            return login_view::view(&self.login, self.theme).map(Message::Login);
        };

        // No way around this screen while set -- not even by navigating
        // directly, since nothing below here renders until it's clear.
        if user.must_change_password {
            return password_change_view::view(&self.password_change, self.theme).map(Message::PasswordChange);
        }

        let primary_routes: Vec<(Route, &str)> = NAV_ITEMS.iter().copied().filter(|(route, _)| !is_secondary_route(*route)).collect();
        let secondary_routes: Vec<(Route, &str)> = NAV_ITEMS
            .iter()
            .copied()
            .filter(|(route, _)| is_secondary_route(*route) && (*route != Route::Users || self.can_manage_users))
            .collect();

        let primary_entries: Vec<NavEntry> = primary_routes.iter().map(|(route, label)| NavEntry::new(label, *route == self.route)).collect();
        let secondary_entries: Vec<NavEntry> = secondary_routes.iter().map(|(route, label)| NavEntry::new(label, *route == self.route)).collect();

        let primary_targets: Vec<Route> = primary_routes.iter().map(|(route, _)| *route).collect();
        let secondary_targets: Vec<Route> = secondary_routes.iter().map(|(route, _)| *route).collect();

        let sidebar = sidebar::view(
            self.theme,
            "MemonBooks",
            self.sidebar_collapsed,
            Message::ToggleSidebar,
            primary_entries,
            move |index| Message::Navigate(primary_targets[index]),
            secondary_entries,
            move |index| Message::Navigate(secondary_targets[index]),
            &user.display_name,
            "Signed in",
            Message::Logout,
            Message::ToggleTheme,
        );

        let screen: Element<'_, Message> = match self.route {
            Route::Dashboard => dashboard_view::view(&self.dashboard, self.theme).map(Message::Dashboard),
            Route::Maintain => self.maintain_view(),
            Route::JournalEntry => journal_entry_view::view(&self.journal_entry, self.theme).map(Message::JournalEntry),
            Route::JournalEntries => journal_entries_view::view(&self.journal_entries, self.theme).map(Message::JournalEntries),
            Route::TrialBalance => trial_balance_view::view(&self.trial_balance, self.theme).map(Message::TrialBalance),
            Route::FixedAssets => fixed_assets_view::view(&self.fixed_assets, self.theme).map(Message::FixedAssets),
            Route::TaxRates => tax_rates_view::view(&self.tax_rates, self.theme).map(Message::TaxRates),
            Route::Transactions => self.transactions_view(),
            Route::Products => products_view::view(&self.products, self.theme).map(Message::Products),
            Route::Employees => employees_view::view(&self.employees, self.theme).map(Message::Employees),
            Route::Expenses => expenses_view::view(&self.expenses, self.theme).map(Message::Expenses),
            Route::Pos => pos_view::view(&self.pos, self.theme).map(Message::Pos),
            Route::Reports => reports_view::view(&self.reports, self.theme).map(Message::Reports),
            Route::Settings => settings_view::view(&self.settings, self.theme).map(Message::Settings),
            Route::Companies => companies_view::view(&self.companies, self.theme).map(Message::Companies),
            Route::SwitchUser => switch_user_view::view(&self.switch_user, self.theme).map(Message::SwitchUser),
            Route::Users => users_view::view(&self.users, self.theme).map(Message::Users),
        };

        row![sidebar, container(screen).width(Length::Fill).height(Length::Fill)].into()
    }

    /// The "Maintain" screen: whichever of the five existing list views
    /// (each with its own already-built `PageHeader` + `Toolbar` +
    /// `DataTable`) is currently selected, with a section picker inline
    /// on that view's own title row via `view_with_switcher`. The other
    /// four aren't rendered at all.
    fn maintain_view(&self) -> Element<'_, Message> {
        let switcher: Element<'_, Message> =
            pick_list(MAINTAIN_SECTIONS, Some(self.maintain_selected), Message::MaintainSelect).style(dropdown::style(self.theme)).padding(spacing::SM).width(Length::Fixed(220.0)).into();

        match self.maintain_selected {
            MaintainSection::ChartOfAccounts => chart_of_accounts_view::view_with_switcher(&self.chart_of_accounts, self.theme, switcher, Message::ChartOfAccounts),
            MaintainSection::Vendors => vendors_view::view_with_switcher(&self.vendors, self.theme, switcher, Message::Vendors),
            MaintainSection::Warehouses => warehouses_view::view_with_switcher(&self.warehouses, self.theme, switcher, Message::Warehouses),
            MaintainSection::BankAccounts => bank_accounts_view::view_with_switcher(&self.bank_accounts, self.theme, switcher, Message::BankAccounts),
            MaintainSection::Customers => customers_view::view_with_switcher(&self.customers, self.theme, switcher, Message::Customers),
        }
    }

    /// The "Transactions" screen: a Purchase/Sale toggle plus a
    /// document-type picker scoped to whichever side is active, above
    /// whichever document view is currently selected. Mirrors
    /// `maintain_view`'s "one already-built view at a time" shape.
    fn transactions_view(&self) -> Element<'_, Message> {
        let toggle = row![
            transaction_side_button(self.theme, "Purchase", self.transaction_side == TransactionSide::Purchase, Message::TransactionSideSelect(TransactionSide::Purchase)),
            transaction_side_button(self.theme, "Sale", self.transaction_side == TransactionSide::Sale, Message::TransactionSideSelect(TransactionSide::Sale)),
        ]
        .spacing(spacing::SM);

        let documents = self.transaction_side.documents();
        let picker = pick_list(documents, Some(self.transaction_document), Message::TransactionDocumentSelect).style(dropdown::style(self.theme)).padding(spacing::SM).width(Length::Fixed(180.0));

        let controls = container(row![toggle, picker].spacing(spacing::SPACE_12).align_y(iced::Alignment::Center)).padding(spacing::HEADER_PADDING);

        let body: Element<'_, Message> = match self.transaction_document {
            TransactionDocument::Bills => bills_view::view(&self.bills, self.theme).map(Message::Bills),
            TransactionDocument::Invoices => invoices_view::view(&self.invoices, self.theme).map(Message::Invoices),
        };

        column![controls, body].into()
    }
}

/// One "Purchase" / "Sale" toggle button for `transactions_view` -- the
/// active side is `button::primary`, the inactive one is
/// `button::secondary`.
fn transaction_side_button<'a, Message: Clone + 'a>(theme: Theme, label: &str, active: bool, on_press: Message) -> Element<'a, Message> {
    let label = label.to_string();
    let btn = button(text(label).size(typography::LABEL)).padding([spacing::SM, spacing::MD]).on_press(on_press);
    if active { btn.style(button_style::primary(theme)) } else { btn.style(button_style::secondary(theme)) }.into()
}

