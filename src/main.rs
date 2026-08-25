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

use chrono::{Datelike, Utc};

use iced::widget::{container, row};
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
use memonbooks::database::database::Database;
use memonbooks::frontend::components::common::sidebar::{self, NavEntry};
use memonbooks::frontend::views::accounting::{chart_of_accounts_view, fixed_assets_view, journal_entry_view, tax_rates_view, trial_balance_view};
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

    iced::application("MemonBooks", MemonBooks::update, MemonBooks::view)
        .window(iced::window::Settings {
            size: iced::Size::new(1280.0, 800.0),
            min_size: Some(iced::Size::new(960.0, 600.0)),
            position: iced::window::Position::Centered,
            resizable: true,
            ..iced::window::Settings::default()
        })
        .run_with(move || (MemonBooks::new(ctx), Task::none()))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Route {
    Dashboard,
    ChartOfAccounts,
    JournalEntry,
    TrialBalance,
    FixedAssets,
    TaxRates,
    Customers,
    Invoices,
    Products,
    Warehouses,
    Vendors,
    Bills,
    BankAccounts,
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
    (Route::ChartOfAccounts, "Chart of Accounts"),
    (Route::JournalEntry, "New Journal Entry"),
    (Route::TrialBalance, "Trial Balance"),
    (Route::FixedAssets, "Fixed Assets"),
    (Route::TaxRates, "Tax Rates"),
    (Route::Customers, "Customers"),
    (Route::Invoices, "Invoices"),
    (Route::Products, "Products"),
    (Route::Warehouses, "Warehouses"),
    (Route::Vendors, "Vendors"),
    (Route::Bills, "Bills"),
    (Route::BankAccounts, "Bank Accounts"),
    (Route::Employees, "Employees"),
    (Route::Expenses, "Expenses"),
    (Route::Pos, "Point of Sale"),
    (Route::Reports, "Reports"),
    (Route::Settings, "Settings"),
    (Route::Companies, "Companies"),
    (Route::SwitchUser, "Switch User"),
    (Route::Users, "Users"),
];

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
    route: Route,
    sidebar_collapsed: bool,
    login: login_view::State,
    password_change: password_change_view::State,
    dashboard: dashboard_view::State,
    chart_of_accounts: chart_of_accounts_view::State,
    journal_entry: journal_entry_view::State,
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
    ToggleSidebar,
    Logout,
    Login(login_view::Message),
    PasswordChange(password_change_view::Message),
    Dashboard(dashboard_view::Message),
    ChartOfAccounts(chart_of_accounts_view::Message),
    JournalEntry(journal_entry_view::Message),
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
    fn new(ctx: Ctx) -> Self {
        Self {
            ctx,
            session: None,
            can_manage_users: false,
            route: Route::Dashboard,
            sidebar_collapsed: false,
            login: login_view::State::default(),
            password_change: password_change_view::State::default(),
            dashboard: dashboard_view::State::default(),
            chart_of_accounts: chart_of_accounts_view::State::default(),
            journal_entry: journal_entry_view::State::default(),
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
            Route::ChartOfAccounts => self.chart_of_accounts = chart_of_accounts_view::load(&self.ctx),
            Route::JournalEntry => self.journal_entry = journal_entry_view::load(&self.ctx),
            Route::TrialBalance => self.trial_balance = trial_balance_view::load(&self.ctx),
            Route::FixedAssets => self.fixed_assets = fixed_assets_view::load(&self.ctx),
            Route::TaxRates => self.tax_rates = tax_rates_view::load(&self.ctx),
            Route::Customers => self.customers = customers_view::load(&self.ctx),
            Route::Invoices => self.invoices = invoices_view::load(&self.ctx),
            Route::Products => self.products = products_view::load(&self.ctx),
            Route::Warehouses => self.warehouses = warehouses_view::load(&self.ctx),
            Route::Vendors => self.vendors = vendors_view::load(&self.ctx),
            Route::Bills => self.bills = bills_view::load(&self.ctx),
            Route::BankAccounts => self.bank_accounts = bank_accounts_view::load(&self.ctx),
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

    fn update(&mut self, message: Message) {
        match message {
            Message::Navigate(route) => {
                self.route = route;
                self.load_route(route);
            }
            Message::ToggleSidebar => self.sidebar_collapsed = !self.sidebar_collapsed,
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
            return login_view::view(&self.login).map(Message::Login);
        };

        // No way around this screen while set -- not even by navigating
        // directly, since nothing below here renders until it's clear.
        if user.must_change_password {
            return password_change_view::view(&self.password_change).map(Message::PasswordChange);
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
        );

        let screen: Element<'_, Message> = match self.route {
            Route::Dashboard => dashboard_view::view(&self.dashboard).map(Message::Dashboard),
            Route::ChartOfAccounts => chart_of_accounts_view::view(&self.chart_of_accounts).map(Message::ChartOfAccounts),
            Route::JournalEntry => journal_entry_view::view(&self.journal_entry).map(Message::JournalEntry),
            Route::TrialBalance => trial_balance_view::view(&self.trial_balance).map(Message::TrialBalance),
            Route::FixedAssets => fixed_assets_view::view(&self.fixed_assets).map(Message::FixedAssets),
            Route::TaxRates => tax_rates_view::view(&self.tax_rates).map(Message::TaxRates),
            Route::Customers => customers_view::view(&self.customers).map(Message::Customers),
            Route::Invoices => invoices_view::view(&self.invoices).map(Message::Invoices),
            Route::Products => products_view::view(&self.products).map(Message::Products),
            Route::Warehouses => warehouses_view::view(&self.warehouses).map(Message::Warehouses),
            Route::Vendors => vendors_view::view(&self.vendors).map(Message::Vendors),
            Route::Bills => bills_view::view(&self.bills).map(Message::Bills),
            Route::BankAccounts => bank_accounts_view::view(&self.bank_accounts).map(Message::BankAccounts),
            Route::Employees => employees_view::view(&self.employees).map(Message::Employees),
            Route::Expenses => expenses_view::view(&self.expenses).map(Message::Expenses),
            Route::Pos => pos_view::view(&self.pos).map(Message::Pos),
            Route::Reports => reports_view::view(&self.reports).map(Message::Reports),
            Route::Settings => settings_view::view(&self.settings).map(Message::Settings),
            Route::Companies => companies_view::view(&self.companies).map(Message::Companies),
            Route::SwitchUser => switch_user_view::view(&self.switch_user).map(Message::SwitchUser),
            Route::Users => users_view::view(&self.users).map(Message::Users),
        };

        row![sidebar, container(screen).width(Length::Fill).height(Length::Fill)].into()
    }
}
