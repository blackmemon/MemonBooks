use memonbooks::{
    backend::{
        accounting::{
            accounting_rules::AccountType, chart_of_accounts::Account, fiscal_periods::FiscalPeriod,
        },
        inventory::{products::Product, warehouses::Warehouse},
        organization::companies::Company,
        parties::{customers::Customer, vendors::Vendor},
        repositories::{
            accounting_repository::AccountingRepository, inventory_repository::InventoryRepository,
            organization_repository::OrganizationRepository, party_repository::PartyRepository,
        },
        services::document_service::DocumentService,
        transactions::documents::*,
    },
    database::database::Database,
};
use rust_decimal_macros::dec;
fn setup() -> (Database, DocumentService, i64) {
    let db = Database::open_in_memory().unwrap();
    db.run_migrations(&memonbooks::all_migrations()).unwrap();
    let company = OrganizationRepository::new(db.clone())
        .create_company(&Company::new("Test", "PKR"))
        .unwrap()
        .id
        .unwrap();
    let r = AccountingRepository::new(db.clone());
    r.create_fiscal_period(&FiscalPeriod::new(
        company,
        "2026",
        parse_date("2026-01-01").unwrap(),
        parse_date("2026-12-31").unwrap(),
    ))
    .unwrap();
    for (code, name, kind) in [
        ("100", "Cash", AccountType::Asset),
        ("110", "Accounts Receivable (Debtors)", AccountType::Asset),
        ("120", "Inventory", AccountType::Asset),
        (
            "200",
            "Accounts Payable (Creditors)",
            AccountType::Liability,
        ),
        ("210", "Taxes Payable", AccountType::Liability),
        ("400", "Sales", AccountType::Revenue),
        ("500", "Salaries", AccountType::Expense),
    ] {
        r.create_account(&Account::new(company, code, name, kind))
            .unwrap();
    }
    let p = PartyRepository::new(db.clone());
    p.create_customer(&Customer::new(company, "Customer"))
        .unwrap();
    p.create_vendor(&Vendor::new(company, "Vendor")).unwrap();
    let i = InventoryRepository::new(db.clone());
    i.create_product(&Product::new(company, "ITEM", "Widget"))
        .unwrap();
    i.create_warehouse(&Warehouse::new(company, "WH", "Warehouse"))
        .unwrap();
    (db.clone(), DocumentService::new(db), company)
}
fn line(kv: &[(&str, &str)]) -> Fields {
    kv.iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}
fn sale(company: i64) -> Document {
    let mut d = Document::new(company, Kind::SaleInvoice);
    d.set("Date", "2026-10-04");
    d.set("Due Date", "2026-10-31");
    d.set("Customer", "1 · Customer");
    d.set("Location", "1 · Warehouse");
    d.lines = vec![line(&[
        ("Item", "1 · Widget"),
        ("Qty", "2"),
        ("Unit Price", "50"),
        ("Tax %", "10"),
    ])];
    d
}
#[test]
fn all_workflow_definitions_have_unique_names_and_roundtrip() {
    let mut names = std::collections::BTreeSet::new();
    for kind in ALL {
        assert!(names.insert(kind.label()));
        assert_eq!(Kind::parse(kind.label()), Some(*kind));
        assert!(!kind.headers().is_empty());
    }
}

#[test]
fn item_defaults_match_the_item_category_options() {
    let item = Document::new(1, Kind::Item);
    assert_eq!(item.get("Category"), "Product");
    assert_eq!(item.get("Sub-Category"), "Raw Material");
}
#[test]
fn decimal_totals_discount_tax_and_payment_are_exact() {
    let mut d = Document::new(1, Kind::PurchaseInvoice);
    d.lines = vec![line(&[
        ("Qty", "3"),
        ("Unit Price", "0.10"),
        ("Discount", "0.01"),
        ("Tax %", "10"),
    ])];
    d.set("Amount Paid", "0.10");
    d.recalculate().unwrap();
    assert_eq!(num(&d.fields, "Total Amount").unwrap(), dec!(0.32));
    assert_eq!(num(&d.fields, "Balance Due").unwrap(), dec!(0.22));
    d.set("Amount Paid", "1");
    assert!(d.recalculate().is_err());
}
#[test]
fn trip_roundtrip_persists_flight_and_draft_status() {
    let (_, service, c) = setup();
    let mut d = Document::new(c, Kind::Trip);
    d.set("Flight", "One-way");
    d.set("Budget limit", "42.50");
    let saved = service.save(&d).unwrap();
    let actual = service.list(c, Kind::Trip).unwrap();
    assert_eq!(actual[0], saved);
    assert_eq!(actual[0].get("Flight"), "One-way");
    d.set("Budget limit", "bad");
    assert!(service.save(&d).is_err());
    d.set("Budget limit", "-1");
    assert!(service.save(&d).is_err());
}

#[test]
fn saved_documents_survive_database_close_and_reopen() {
    let unique = format!(
        "memonbooks-reopen-{}-{}",
        std::process::id(),
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
    );
    let path = std::env::temp_dir().join(unique).join("app.sqlite3");
    let company_id;
    let saved;
    {
        let db = Database::open(&path).unwrap();
        db.run_migrations(&memonbooks::all_migrations()).unwrap();
        company_id = OrganizationRepository::new(db.clone())
            .create_company(&Company::new("Reopen test", "PKR"))
            .unwrap()
            .id
            .unwrap();
        let service = DocumentService::new(db);
        let mut trip = Document::new(company_id, Kind::Trip);
        trip.set("Name", "Persisted trip");
        trip.set("Budget limit", "125.50");
        saved = service.save(&trip).unwrap();
    }
    {
        let db = Database::open(&path).unwrap();
        db.run_migrations(&memonbooks::all_migrations()).unwrap();
        let reopened = DocumentService::new(db)
            .list(company_id, Kind::Trip)
            .unwrap();
        assert_eq!(reopened, vec![saved]);
    }
    std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
}

#[test]
fn posted_invoice_and_stock_are_atomic_and_immutable() {
    let (db, s, c) = setup();
    let mut d = sale(c);
    d.status = "Posted".into();
    let saved = s.save(&d).unwrap();
    assert!(saved.transaction_id.is_some());
    assert_eq!(
        AccountingRepository::new(db.clone())
            .list_open_receivables_for_company(c)
            .unwrap()[0]
            .amount,
        dec!(110)
    );
    assert!(s.save(&saved).is_err());
    assert!(s.delete(c, &[saved.id.unwrap()]).is_err());
    let tb = AccountingRepository::new(db)
        .trial_balance_as_of(c, parse_date("2026-12-31").unwrap())
        .unwrap();
    assert_eq!(tb.total_debits(), tb.total_credits());
}
#[test]
fn failed_stock_validation_rolls_back_document_ledger_and_receivable() {
    let (db, s, c) = setup();
    let mut d = sale(c);
    d.status = "Posted".into();
    d.set("Location", "999 · Missing");
    assert!(s.save(&d).is_err());
    assert!(s.list(c, Kind::SaleInvoice).unwrap().is_empty());
    let a = AccountingRepository::new(db);
    assert!(a.list_transactions_for_company(c).unwrap().is_empty());
    assert!(a.list_open_receivables_for_company(c).unwrap().is_empty());
}
#[test]
fn editing_stock_document_replaces_effect_instead_of_doubling() {
    let (db, s, c) = setup();
    let mut d = sale(c);
    let saved = s.save(&d).unwrap();
    d = saved;
    d.lines[0].insert("Qty".into(), "3".into());
    let saved = s.save(&d).unwrap();
    let qty = db
        .with_connection(|conn| {
            Ok(conn.query_row(
                "SELECT quantity_delta FROM stock_movements WHERE reference=?",
                [format!("workflow:{}", saved.id.unwrap())],
                |r| r.get::<_, String>(0),
            )?)
        })
        .unwrap();
    assert_eq!(qty, "-3");
    assert!(s.save(&d).is_err());
    s.delete(c, &[saved.id.unwrap()]).unwrap();
    let count = db
        .with_connection(|conn| {
            Ok(
                conn.query_row("SELECT count(*) FROM stock_movements", [], |r| {
                    r.get::<_, i64>(0)
                })?,
            )
        })
        .unwrap();
    assert_eq!(count, 0);
}

#[test]
fn posted_purchase_and_sale_returns_reverse_stock_in_their_location() {
    let (db, service, company) = setup();
    for (kind, party_key, party, direction) in [
        (Kind::PurchaseReturn, "Vendor", "1 · Vendor", -1),
        (Kind::SaleReturn, "Customer", "1 · Customer", 1),
    ] {
        let mut returned = Document::new(company, kind);
        returned.set("Date", "2026-10-04");
        returned.set(party_key, party);
        returned.set("Location", "1 · Warehouse");
        returned.lines = vec![line(&[
            ("Item", "1 · Widget"),
            ("Qty", "1"),
            ("Unit Price", "50"),
        ])];
        returned.status = "Posted".into();
        let saved = service.save(&returned).unwrap();
        let stock_delta = db
            .with_connection(|connection| {
                Ok(connection.query_row(
                    "SELECT quantity_delta FROM stock_movements WHERE reference=?",
                    [format!("workflow:{}", saved.id.unwrap())],
                    |row| row.get::<_, String>(0),
                )?)
            })
            .unwrap();
        assert_eq!(stock_delta.parse::<i32>().unwrap(), direction);
    }
    assert_eq!(
        InventoryRepository::new(db)
            .on_hand_quantity(company, 1, 1)
            .unwrap(),
        dec!(0)
    );
}

#[test]
fn conversion_is_new_draft_and_carries_source_link() {
    let (_, s, c) = setup();
    let mut d = Document::new(c, Kind::PurchaseOrder);
    d.lines = vec![line(&[
        ("Qty", "2"),
        ("Rate", "12.50"),
        ("Item", "1 · Widget"),
    ])];
    assert!(d.convert().is_err());
    let original = s.save(&d).unwrap();
    let converted = original.convert().unwrap();
    assert_eq!(converted.kind, Kind::GoodsReceipt);
    assert_eq!(converted.id, None);
    assert_eq!(converted.get("Link to PO"), original.number);
    assert_eq!(get(&converted.lines[0], "Accepted"), "2");
    assert_eq!(get(&converted.lines[0], "Unit Cost"), "12.50");
}
#[test]
fn template_has_no_identity_status_or_attachments() {
    let (_, s, c) = setup();
    let mut d = Document::new(c, Kind::Quotation);
    d.set("Notes/Memo", "Reusable");
    s.save_template(&d, "Retail").unwrap();
    let new = s.template(c, Kind::Quotation, "Retail").unwrap();
    assert!(new.id.is_none());
    assert_eq!(new.status, "Draft");
    assert_eq!(new.get("Notes/Memo"), "Reusable");
    assert_eq!(s.templates(c, Kind::Quotation).unwrap(), vec!["Retail"]);
}
#[test]
fn employee_custom_fields_status_leave_date_and_payroll_duplicates() {
    let (_, s, c) = setup();
    let mut e = Document::new(c, Kind::Employee);
    e.set("First Name", "Ada");
    e.set("Last Name", "Example");
    e.set("Basic Salary / Hourly Rate", "3100");
    e.set("Leave Date", "2026-12-31");
    e.set("Custom 1 name", "Badge");
    e.set("Custom 1 value", "R-42");
    let e = s.save(&e).unwrap();
    assert_eq!(
        s.list(c, Kind::Employee).unwrap()[0].get("Leave Date"),
        "2026-12-31"
    );
    assert_eq!(
        s.list(c, Kind::Employee).unwrap()[0].get("Custom 1 value"),
        "R-42"
    );
    assert_eq!(
        s.generate_payroll(c, "2026-10", &[e.id.unwrap()]).unwrap(),
        (1, 0)
    );
    assert_eq!(
        s.generate_payroll(c, "2026-10", &[e.id.unwrap()]).unwrap(),
        (0, 1)
    );
    let p = &s.list(c, Kind::Payroll).unwrap()[0];
    assert_eq!(num(&p.fields, "Net Pay").unwrap(), dec!(3100));
    let mut duplicate = p.clone();
    duplicate.id = None;
    assert!(s.save(&duplicate).is_err());
}
#[test]
fn payroll_uses_calendar_days_and_excludes_employer_pf_from_deductions() {
    let mut d = Document::new(1, Kind::Payroll);
    d.set("Period", "2024-02");
    d.set("Days Worked", "29");
    d.set("Basic Salary", "2900");
    d.set("Employer PF", "300");
    d.set("Employee PF", "100");
    d.recalculate().unwrap();
    assert_eq!(d.get("Days in Month"), "29");
    assert_eq!(num(&d.fields, "Net Pay").unwrap(), dec!(2800));
}
#[test]
fn unbalanced_journal_is_rejected_without_persistence() {
    let (_, s, c) = setup();
    let mut d = Document::new(c, Kind::JournalVoucher);
    d.status = "Posted".into();
    d.set("Date", "2026-10-04");
    d.lines = vec![
        line(&[("Account", "1 · Cash"), ("Debit", "100")]),
        line(&[("Account", "6 · Sales"), ("Credit", "99")]),
    ];
    assert!(s.save(&d).is_err());
    assert!(s.list(c, Kind::JournalVoucher).unwrap().is_empty());
}
#[test]
fn receipt_allocates_oldest_invoice_and_reduces_outstanding() {
    let (db, s, c) = setup();
    let mut first = sale(c);
    first.status = "Posted".into();
    s.save(&first).unwrap();
    s.save(&first).unwrap();
    let mut receipt = Document::new(c, Kind::CashReceipt);
    receipt.status = "Posted".into();
    receipt.set("Date", "2026-10-04");
    receipt.set("Customer", "1 · Customer");
    receipt.set("Cash Account", "1 · Cash");
    receipt.lines = vec![line(&[("Amount", "150"), ("Customer", "1 · Customer")])];
    s.save(&receipt).unwrap();
    let open = AccountingRepository::new(db)
        .list_open_receivables_for_company(c)
        .unwrap();
    assert_eq!(open.len(), 1);
    assert_eq!(open[0].balance(), dec!(70));
}

#[test]
fn maintains_existing_masters_and_synchronizes_extended_fields() {
    let (db, s, c) = setup();
    let vendors = s.list(c, Kind::Vendor).unwrap();
    assert_eq!(vendors.len(), 1);
    assert!(vendors[0].id.unwrap() < 0);
    let mut v = vendors[0].clone();
    v.set("Vendor Name", "Updated Vendor");
    v.set("Company Name", "Company");
    v.set("Phone", "5550100");
    v.set("Mailing Address", "One Example Street");
    let v = s.save(&v).unwrap();
    assert_eq!(s.list(c, Kind::Vendor).unwrap().len(), 1);
    assert_eq!(
        PartyRepository::new(db)
            .list_vendors_for_company(c)
            .unwrap()[0]
            .name,
        "Updated Vendor"
    );
    assert!(s.delete(c, &[v.id.unwrap()]).is_err());
    let mut inactive = v;
    inactive.status = "Inactive".into();
    let inactive = s.save(&inactive).unwrap();
    s.delete(c, &[inactive.id.unwrap()]).unwrap();
    assert!(s.list(c, Kind::Vendor).unwrap().is_empty());
}
#[test]
fn account_parent_cycles_are_rejected_atomically() {
    let (_, s, c) = setup();
    let mut a = Document::new(c, Kind::Account);
    a.set("Account Name", "Parent Account");
    let a = s.save(&a).unwrap();
    let mut b = Document::new(c, Kind::Account);
    b.set("Account Name", "Child Account");
    b.set("Parent Account", a.get("Master ID"));
    let b = s.save(&b).unwrap();
    let mut cycle = a.clone();
    cycle.set("Parent Account", b.get("Master ID"));
    assert!(s.save(&cycle).is_err());
    let saved = s
        .list(c, Kind::Account)
        .unwrap()
        .into_iter()
        .find(|d| d.id == a.id)
        .unwrap();
    assert!(saved.get("Parent Account").is_empty());
}
#[test]
fn stock_master_can_set_quantity_to_zero() {
    let (db, s, c) = setup();
    let mut item = s.list(c, Kind::Item).unwrap()[0].clone();
    item.set("Stock at 1 · Warehouse", "4");
    let mut item = s.save(&item).unwrap();
    item.set("Stock at 1 · Warehouse", "0");
    s.save(&item).unwrap();
    assert_eq!(
        InventoryRepository::new(db)
            .on_hand_quantity(c, 1, 1)
            .unwrap(),
        dec!(0)
    );
}
#[test]
fn payroll_paid_posts_to_ledger_and_locks_record() {
    let (db, s, c) = setup();
    let mut employee = Document::new(c, Kind::Employee);
    employee.set("First Name", "Test");
    employee.set("Last Name", "Employee");
    employee.set("Basic Salary / Hourly Rate", "1000");
    let employee = s.save(&employee).unwrap();
    let mut p = s.payroll_for(&employee, "2026-10").unwrap();
    p.set("Salary Expense Account", "7 · Salaries");
    p.set("Payment Account", "1 · Cash");
    p.set("Payment Date", "2026-10-04");
    p.status = "Paid".into();
    let saved = s.save(&p).unwrap();
    assert!(saved.locked());
    assert_eq!(
        AccountingRepository::new(db)
            .list_transactions_for_company(c)
            .unwrap()[0]
            .total_debits(),
        dec!(1000)
    );
}
#[test]
fn imports_case_insensitive_reordered_headers_and_skips_blank_names() {
    let (_, s, c) = setup();
    let result = s
        .import(
            c,
            Kind::Customer,
            &[
                vec!["EMAIL".into(), "customer name".into()],
                vec!["a@example.test".into(), "Alice".into()],
                vec!["".into(), "".into()],
            ],
        )
        .unwrap();
    assert_eq!(result, (1, 1));
    assert!(s
        .list(c, Kind::Customer)
        .unwrap()
        .iter()
        .any(|d| d.get("Customer Name") == "Alice"));
}
