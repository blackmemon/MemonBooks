//! Integration test for POS (Development Order step 16): open a
//! session, ring up a split-tender sale, record a cash drawer event,
//! and close the session, verified against the ledger and the expected
//! cash math together.

use chrono::{NaiveDate, Utc};
use rust_decimal_macros::dec;

use memonbooks::backend::accounting::accounting_rules::AccountType;
use memonbooks::backend::accounting::chart_of_accounts::Account;
use memonbooks::backend::accounting::fiscal_periods::FiscalPeriod;
use memonbooks::backend::core::roles::Role;
use memonbooks::backend::core::users::User;
use memonbooks::backend::inventory::products::Product;
use memonbooks::backend::inventory::warehouses::Warehouse;
use memonbooks::backend::organization::companies::Company;
use memonbooks::backend::pos::cash_drawer::{self, CashDrawerEvent, CashDrawerEventType};
use memonbooks::backend::pos::payments::{PaymentMethod, PosPayment};
use memonbooks::backend::pos::sales::{PosSale, PosSaleLine};
use memonbooks::backend::pos::sessions::{PosSession, PosSessionStatus};
use memonbooks::backend::pos::terminals::Terminal;
use memonbooks::backend::repositories::accounting_repository::{self, AccountingRepository};
use memonbooks::backend::repositories::inventory_repository::{self, InventoryRepository};
use memonbooks::backend::repositories::organization_repository::{self, OrganizationRepository};
use memonbooks::backend::repositories::sales_repository::{self, SalesRepository};
use memonbooks::backend::repositories::user_repository::{self, UserRepository};
use memonbooks::database::database::Database;

fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
}

#[test]
fn full_pos_session_rings_up_a_split_tender_sale_and_reconciles_cash() {
    let db = Database::open_in_memory().unwrap();
    db.run_migrations(organization_repository::MIGRATIONS)
        .unwrap();
    db.run_migrations(user_repository::MIGRATIONS).unwrap();
    db.run_migrations(accounting_repository::MIGRATIONS)
        .unwrap();
    db.run_migrations(inventory_repository::MIGRATIONS).unwrap();
    db.run_migrations(memonbooks::backend::audit::audit_logs::MIGRATIONS)
        .unwrap();
    db.run_migrations(sales_repository::MIGRATIONS).unwrap();

    let org_repo = OrganizationRepository::new(db.clone());
    let company_id = org_repo
        .create_company(&Company::new("Acme Trading Co", "USD"))
        .unwrap()
        .id
        .unwrap();

    let user_repo = UserRepository::new(db.clone());
    let role = user_repo.create_role(&Role::new("Cashier")).unwrap();
    let cashier = user_repo
        .create_user(&User::new(
            "cashier",
            "Cashier One",
            "hash",
            role.id.unwrap(),
        ))
        .unwrap();

    let inventory_repo = InventoryRepository::new(db.clone());
    let warehouse = inventory_repo
        .create_warehouse(&Warehouse::new(company_id, "Main Warehouse", "MAIN"))
        .unwrap();
    let product = inventory_repo
        .create_product(&Product::new(company_id, "WIDGET-1", "Widget"))
        .unwrap();

    let accounting_repo = AccountingRepository::new(db.clone());
    accounting_repo
        .create_fiscal_period(&FiscalPeriod::new(
            company_id,
            "January 2026",
            date(2026, 1, 1),
            date(2026, 1, 31),
        ))
        .unwrap();
    let revenue = accounting_repo
        .create_account(&Account::new(
            company_id,
            "4000",
            "Sales Revenue",
            AccountType::Revenue,
        ))
        .unwrap();
    let cash_account = accounting_repo
        .create_account(&Account::new(
            company_id,
            "1000",
            "Cash",
            AccountType::Asset,
        ))
        .unwrap();
    let card_clearing = accounting_repo
        .create_account(&Account::new(
            company_id,
            "1050",
            "Card Clearing",
            AccountType::Asset,
        ))
        .unwrap();

    let sales_repo = SalesRepository::new(db);
    let terminal = sales_repo
        .create_terminal(&Terminal::new(
            company_id,
            warehouse.id.unwrap(),
            "Front Counter",
        ))
        .unwrap();
    let session = sales_repo
        .open_session(&PosSession::open(
            terminal.id.unwrap(),
            cashier.id.unwrap(),
            dec!(200),
        ))
        .unwrap();

    let mut sale = PosSale::new(
        company_id,
        terminal.id.unwrap(),
        session.id.unwrap(),
        date(2026, 1, 15),
    );
    sale.lines.push(PosSaleLine::new(
        product.id.unwrap(),
        revenue.id.unwrap(),
        "Widget",
        dec!(3),
        dec!(50),
    ));
    sale.payments
        .push(PosPayment::new(PaymentMethod::Cash, dec!(90)));
    sale.payments
        .push(PosPayment::new(PaymentMethod::Card, dec!(60)));

    let (posted_sale, transaction) = sales_repo
        .post_pos_sale(
            &sale,
            &[
                (PaymentMethod::Cash, cash_account.id.unwrap()),
                (PaymentMethod::Card, card_clearing.id.unwrap()),
            ],
        )
        .unwrap();
    assert!(transaction.is_balanced());
    assert!(posted_sale.transaction_id.is_some());

    let cash_sale_event = sales_repo
        .record_cash_drawer_event(&CashDrawerEvent::new(
            session.id.unwrap(),
            CashDrawerEventType::CashSale,
            dec!(90),
        ))
        .unwrap();
    let events = sales_repo
        .list_cash_drawer_events(session.id.unwrap())
        .unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].id, cash_sale_event.id);

    let expected_closing = cash_drawer::expected_closing_cash(session.opening_cash, &events);
    assert_eq!(expected_closing, dec!(290)); // 200 float + 90 cash sale

    let closed = sales_repo
        .close_session(session.id.unwrap(), expected_closing, Utc::now())
        .unwrap();
    assert_eq!(closed.status, PosSessionStatus::Closed);
    assert_eq!(
        cash_drawer::over_or_short(expected_closing, closed.closing_cash.unwrap()),
        rust_decimal::Decimal::ZERO
    );

    let trial_balance = accounting_repo
        .trial_balance_as_of(company_id, date(2026, 1, 31))
        .unwrap();
    assert!(trial_balance.is_balanced());
}

/// The happy-path test above only ever closes a session with the
/// drawer counted exactly as expected. This covers the case that
/// actually matters operationally: the drawer is short (a till count
/// discrepancy) at close, and `close_session` still succeeds and
/// records the real counted amount rather than silently forcing it to
/// match the expectation.
#[test]
fn closing_a_session_with_a_cash_shortage_still_succeeds_and_records_the_real_count() {
    let db = Database::open_in_memory().unwrap();
    db.run_migrations(organization_repository::MIGRATIONS)
        .unwrap();
    db.run_migrations(user_repository::MIGRATIONS).unwrap();
    db.run_migrations(accounting_repository::MIGRATIONS)
        .unwrap();
    db.run_migrations(inventory_repository::MIGRATIONS).unwrap();
    db.run_migrations(memonbooks::backend::audit::audit_logs::MIGRATIONS)
        .unwrap();
    db.run_migrations(sales_repository::MIGRATIONS).unwrap();

    let org_repo = OrganizationRepository::new(db.clone());
    let company_id = org_repo
        .create_company(&Company::new("Acme Trading Co", "USD"))
        .unwrap()
        .id
        .unwrap();

    let user_repo = UserRepository::new(db.clone());
    let role = user_repo.create_role(&Role::new("Cashier")).unwrap();
    let cashier = user_repo
        .create_user(&User::new(
            "cashier",
            "Cashier One",
            "hash",
            role.id.unwrap(),
        ))
        .unwrap();

    let inventory_repo = InventoryRepository::new(db.clone());
    let warehouse = inventory_repo
        .create_warehouse(&Warehouse::new(company_id, "Main Warehouse", "MAIN"))
        .unwrap();

    let accounting_repo = AccountingRepository::new(db.clone());
    accounting_repo
        .create_fiscal_period(&FiscalPeriod::new(
            company_id,
            "January 2026",
            date(2026, 1, 1),
            date(2026, 1, 31),
        ))
        .unwrap();

    let sales_repo = SalesRepository::new(db);
    let terminal = sales_repo
        .create_terminal(&Terminal::new(
            company_id,
            warehouse.id.unwrap(),
            "Front Counter",
        ))
        .unwrap();
    let session = sales_repo
        .open_session(&PosSession::open(
            terminal.id.unwrap(),
            cashier.id.unwrap(),
            dec!(200),
        ))
        .unwrap();

    let sale_event = sales_repo
        .record_cash_drawer_event(&CashDrawerEvent::new(
            session.id.unwrap(),
            CashDrawerEventType::CashSale,
            dec!(150),
        ))
        .unwrap();
    let events = sales_repo
        .list_cash_drawer_events(session.id.unwrap())
        .unwrap();
    assert_eq!(events[0].id, sale_event.id);

    let expected_closing = cash_drawer::expected_closing_cash(session.opening_cash, &events);
    assert_eq!(expected_closing, dec!(350)); // 200 float + 150 cash sale

    // The actual count comes up short by 15 -- cash missing from the drawer.
    let counted = dec!(335);
    let closed = sales_repo
        .close_session(session.id.unwrap(), counted, Utc::now())
        .unwrap();

    assert_eq!(closed.status, PosSessionStatus::Closed);
    assert_eq!(closed.closing_cash, Some(counted));
    assert_eq!(
        cash_drawer::over_or_short(expected_closing, closed.closing_cash.unwrap()),
        dec!(-15)
    );
}
