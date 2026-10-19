//! Persistence for customers, vendors, and their addresses/contacts.

use rusqlite::{params, OptionalExtension, Row};

use crate::backend::core::errors::{AppError, AppResult};
use crate::backend::core::validation::Validate;
use crate::backend::parties::addresses::{Address, AddressLabel};
use crate::backend::parties::contacts::Contact;
use crate::backend::parties::customers::Customer;
use crate::backend::parties::vendors::Vendor;
use crate::backend::parties::PartyType;
use crate::database::database::{Database, Migration};

pub const MIGRATIONS: &[Migration] = &[Migration {
    version: 900,
    name: "parties_schema",
    sql: "
        CREATE TABLE customers (
            id                  INTEGER PRIMARY KEY AUTOINCREMENT,
            company_id          INTEGER NOT NULL REFERENCES companies(id),
            name                TEXT NOT NULL,
            email               TEXT,
            phone               TEXT,
            tax_id              TEXT,
            credit_limit        TEXT NOT NULL,
            payment_terms_days  INTEGER NOT NULL,
            is_active           INTEGER NOT NULL DEFAULT 1
        );

        CREATE TABLE vendors (
            id                  INTEGER PRIMARY KEY AUTOINCREMENT,
            company_id          INTEGER NOT NULL REFERENCES companies(id),
            name                TEXT NOT NULL,
            email               TEXT,
            phone               TEXT,
            tax_id              TEXT,
            payment_terms_days  INTEGER NOT NULL,
            is_active           INTEGER NOT NULL DEFAULT 1
        );

        CREATE TABLE addresses (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            party_type  TEXT NOT NULL,
            party_id    INTEGER NOT NULL,
            label       TEXT NOT NULL,
            line1       TEXT NOT NULL,
            line2       TEXT,
            city        TEXT NOT NULL,
            state       TEXT,
            postal_code TEXT,
            country     TEXT NOT NULL
        );

        CREATE TABLE contacts (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            party_type  TEXT NOT NULL,
            party_id    INTEGER NOT NULL,
            name        TEXT NOT NULL,
            email       TEXT,
            phone       TEXT,
            is_primary  INTEGER NOT NULL DEFAULT 0
        );
    ",
}, Migration {
    // 902, not 901: 901 (accounting_repository's receivables/payables
    // schema) already claims the next slot after this file's own 900.
    version: 902,
    name: "customers_and_vendors_reference_code_columns",
    sql: "
        ALTER TABLE customers ADD COLUMN reference_code TEXT;
        ALTER TABLE vendors ADD COLUMN reference_code TEXT;
        -- Plain indexes, not UNIQUE column constraints on the ALTER
        -- TABLE itself -- SQLite's ADD COLUMN doesn't support UNIQUE
        -- constraints on new columns. NULLs (rows from before this
        -- column existed) are never considered equal to each other
        -- under a UNIQUE index, so pre-existing rows with no
        -- reference_code don't collide with one another.
        CREATE UNIQUE INDEX customers_reference_code_unique ON customers (reference_code) WHERE reference_code IS NOT NULL;
        CREATE UNIQUE INDEX vendors_reference_code_unique ON vendors (reference_code) WHERE reference_code IS NOT NULL;
    ",
}, Migration {
    version: 903,
    name: "vendor_type_column",
    sql: "ALTER TABLE vendors ADD COLUMN vendor_type TEXT NOT NULL DEFAULT 'Other';",
}, Migration {
    version: 904,
    name: "vendor_company_name_column",
    sql: "ALTER TABLE vendors ADD COLUMN company_name TEXT;",
}];

#[derive(Clone)]
pub struct PartyRepository {
    db: Database,
}

impl PartyRepository {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    // -- customers --------------------------------------------------------

    pub fn create_customer(&self, customer: &Customer) -> AppResult<Customer> {
        customer.validate()?;
        self.db.with_connection(|conn| {
            conn.execute(
                "INSERT INTO customers (company_id, name, email, phone, tax_id, credit_limit, payment_terms_days, is_active, reference_code)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    customer.company_id,
                    customer.name,
                    customer.email,
                    customer.phone,
                    customer.tax_id,
                    customer.credit_limit.to_string(),
                    customer.payment_terms_days,
                    customer.is_active,
                    customer.reference_code,
                ],
            )?;
            Ok(Customer {
                id: Some(conn.last_insert_rowid()),
                ..customer.clone()
            })
        })
    }

    pub fn get_customer(&self, id: i64) -> AppResult<Customer> {
        self.db.with_connection(|conn| {
            conn.query_row(
                "SELECT id, company_id, name, email, phone, tax_id, credit_limit, payment_terms_days, is_active, reference_code
                 FROM customers WHERE id = ?1",
                params![id],
                map_customer,
            )
            .optional()?
            .ok_or_else(|| AppError::NotFound(format!("customer {id} not found")))
        })
    }

    pub fn list_customers_for_company(&self, company_id: i64) -> AppResult<Vec<Customer>> {
        self.db.with_connection(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, company_id, name, email, phone, tax_id, credit_limit, payment_terms_days, is_active, reference_code
                 FROM customers WHERE company_id = ?1 ORDER BY name",
            )?;
            let rows = stmt.query_map(params![company_id], map_customer)?;
            rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
        })
    }

    pub fn update_customer(&self, customer: &Customer) -> AppResult<()> {
        customer.validate()?;
        let id = customer
            .id
            .ok_or_else(|| AppError::Validation("cannot update a customer without an id".into()))?;
        self.db.with_connection(|conn| {
            let affected = conn.execute(
                "UPDATE customers SET name = ?1, email = ?2, phone = ?3, tax_id = ?4, credit_limit = ?5,
                 payment_terms_days = ?6, is_active = ?7 WHERE id = ?8",
                params![
                    customer.name,
                    customer.email,
                    customer.phone,
                    customer.tax_id,
                    customer.credit_limit.to_string(),
                    customer.payment_terms_days,
                    customer.is_active,
                    id,
                ],
            )?;
            if affected == 0 {
                return Err(AppError::NotFound(format!("customer {id} not found")));
            }
            Ok(())
        })
    }

    /// Rejects with a clear message, rather than a raw SQLite one, when
    /// the customer is still referenced elsewhere -- invoices,
    /// quotations, credit notes, and receivables all reference a
    /// customer by id, and every one of those foreign keys is enforced.
    pub fn delete_customer(&self, id: i64) -> AppResult<()> {
        self.db
            .with_connection(|conn| {
                let affected = conn.execute("DELETE FROM customers WHERE id = ?1", params![id])?;
                if affected == 0 {
                    return Err(AppError::NotFound(format!("customer {id} not found")));
                }
                Ok(())
            })
            .map_err(|err| {
                if err.is_constraint_violation() {
                    AppError::Validation(format!("cannot delete customer {id}: it's still referenced by invoices or other records"))
                } else {
                    err
                }
            })
    }

    // -- vendors ------------------------------------------------------

    pub fn create_vendor(&self, vendor: &Vendor) -> AppResult<Vendor> {
        vendor.validate()?;
        self.db.with_connection(|conn| {
            conn.execute(
                "INSERT INTO vendors (company_id, name, company_name, email, phone, tax_id, payment_terms_days, is_active, reference_code, vendor_type)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    vendor.company_id,
                    vendor.name,
                    vendor.company_name,
                    vendor.email,
                    vendor.phone,
                    vendor.tax_id,
                    vendor.payment_terms_days,
                    vendor.is_active,
                    vendor.reference_code,
                    vendor.vendor_type,
                ],
            )?;
            Ok(Vendor {
                id: Some(conn.last_insert_rowid()),
                ..vendor.clone()
            })
        })
    }

    pub fn get_vendor(&self, id: i64) -> AppResult<Vendor> {
        self.db.with_connection(|conn| {
            conn.query_row(
                "SELECT id, company_id, name, email, phone, tax_id, payment_terms_days, is_active, reference_code, vendor_type, company_name
                 FROM vendors WHERE id = ?1",
                params![id],
                map_vendor,
            )
            .optional()?
            .ok_or_else(|| AppError::NotFound(format!("vendor {id} not found")))
        })
    }

    pub fn list_vendors_for_company(&self, company_id: i64) -> AppResult<Vec<Vendor>> {
        self.db.with_connection(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, company_id, name, email, phone, tax_id, payment_terms_days, is_active, reference_code, vendor_type, company_name
                 FROM vendors WHERE company_id = ?1 ORDER BY name",
            )?;
            let rows = stmt.query_map(params![company_id], map_vendor)?;
            rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
        })
    }

    pub fn update_vendor(&self, vendor: &Vendor) -> AppResult<()> {
        vendor.validate()?;
        let id = vendor
            .id
            .ok_or_else(|| AppError::Validation("cannot update a vendor without an id".into()))?;
        self.db.with_connection(|conn| {
            let affected = conn.execute(
                "UPDATE vendors SET name = ?1, company_name = ?2, email = ?3, phone = ?4, tax_id = ?5,
                 payment_terms_days = ?6, is_active = ?7, vendor_type = ?8 WHERE id = ?9",
                params![
                    vendor.name,
                    vendor.company_name,
                    vendor.email,
                    vendor.phone,
                    vendor.tax_id,
                    vendor.payment_terms_days,
                    vendor.is_active,
                    vendor.vendor_type,
                    id,
                ],
            )?;
            if affected == 0 {
                return Err(AppError::NotFound(format!("vendor {id} not found")));
            }
            Ok(())
        })
    }

    /// Rejects with a clear message, rather than a raw SQLite one, when
    /// the vendor is still referenced elsewhere -- bills, purchase
    /// orders, debit notes, and payables all reference a vendor by id,
    /// and every one of those foreign keys is enforced.
    pub fn delete_vendor(&self, id: i64) -> AppResult<()> {
        self.db
            .with_connection(|conn| {
                let affected = conn.execute("DELETE FROM vendors WHERE id = ?1", params![id])?;
                if affected == 0 {
                    return Err(AppError::NotFound(format!("vendor {id} not found")));
                }
                Ok(())
            })
            .map_err(|err| {
                if err.is_constraint_violation() {
                    AppError::Validation(format!("cannot delete vendor {id}: it's still referenced by bills or other records"))
                } else {
                    err
                }
            })
    }

    // -- addresses and contacts (shared by customers and vendors) ------

    pub fn create_address(&self, address: &Address) -> AppResult<Address> {
        address.validate()?;
        self.db.with_connection(|conn| {
            conn.execute(
                "INSERT INTO addresses (party_type, party_id, label, line1, line2, city, state, postal_code, country)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    address.party_type.as_str(),
                    address.party_id,
                    address.label.as_str(),
                    address.line1,
                    address.line2,
                    address.city,
                    address.state,
                    address.postal_code,
                    address.country,
                ],
            )?;
            Ok(Address {
                id: Some(conn.last_insert_rowid()),
                ..address.clone()
            })
        })
    }

    pub fn list_addresses_for_party(
        &self,
        party_type: PartyType,
        party_id: i64,
    ) -> AppResult<Vec<Address>> {
        self.db.with_connection(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, party_type, party_id, label, line1, line2, city, state, postal_code, country
                 FROM addresses WHERE party_type = ?1 AND party_id = ?2 ORDER BY id",
            )?;
            let rows = stmt.query_map(params![party_type.as_str(), party_id], map_address)?;
            rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
        })
    }

    pub fn create_contact(&self, contact: &Contact) -> AppResult<Contact> {
        contact.validate()?;
        self.db.with_transaction(|tx| {
            if contact.is_primary {
                tx.execute(
                    "UPDATE contacts SET is_primary = 0 WHERE party_type = ?1 AND party_id = ?2",
                    params![contact.party_type.as_str(), contact.party_id],
                )?;
            }
            tx.execute(
                "INSERT INTO contacts (party_type, party_id, name, email, phone, is_primary)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    contact.party_type.as_str(),
                    contact.party_id,
                    contact.name,
                    contact.email,
                    contact.phone,
                    contact.is_primary,
                ],
            )?;
            Ok(Contact {
                id: Some(tx.last_insert_rowid()),
                ..contact.clone()
            })
        })
    }

    pub fn list_contacts_for_party(
        &self,
        party_type: PartyType,
        party_id: i64,
    ) -> AppResult<Vec<Contact>> {
        self.db.with_connection(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, party_type, party_id, name, email, phone, is_primary
                 FROM contacts WHERE party_type = ?1 AND party_id = ?2 ORDER BY id",
            )?;
            let rows = stmt.query_map(params![party_type.as_str(), party_id], map_contact)?;
            rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
        })
    }
}

fn map_customer(row: &Row) -> rusqlite::Result<Customer> {
    let credit_limit_text: String = row.get(6)?;
    Ok(Customer {
        id: Some(row.get(0)?),
        company_id: row.get(1)?,
        name: row.get(2)?,
        email: row.get(3)?,
        phone: row.get(4)?,
        tax_id: row.get(5)?,
        credit_limit: credit_limit_text
            .parse()
            .unwrap_or(rust_decimal::Decimal::ZERO),
        payment_terms_days: row.get(7)?,
        is_active: row.get(8)?,
        reference_code: row.get(9)?,
    })
}

fn map_vendor(row: &Row) -> rusqlite::Result<Vendor> {
    Ok(Vendor {
        id: Some(row.get(0)?),
        company_id: row.get(1)?,
        name: row.get(2)?,
        email: row.get(3)?,
        phone: row.get(4)?,
        tax_id: row.get(5)?,
        payment_terms_days: row.get(6)?,
        is_active: row.get(7)?,
        reference_code: row.get(8)?,
        vendor_type: row.get(9)?,
        company_name: row.get(10)?,
    })
}

fn map_address(row: &Row) -> rusqlite::Result<Address> {
    let party_type_text: String = row.get(1)?;
    let party_type = PartyType::parse(&party_type_text).ok_or_else(|| {
        rusqlite::Error::FromSqlConversionFailure(
            1,
            rusqlite::types::Type::Text,
            "invalid party_type".into(),
        )
    })?;
    let label_text: String = row.get(3)?;
    let label = AddressLabel::parse(&label_text).ok_or_else(|| {
        rusqlite::Error::FromSqlConversionFailure(
            3,
            rusqlite::types::Type::Text,
            "invalid address label".into(),
        )
    })?;
    Ok(Address {
        id: Some(row.get(0)?),
        party_type,
        party_id: row.get(2)?,
        label,
        line1: row.get(4)?,
        line2: row.get(5)?,
        city: row.get(6)?,
        state: row.get(7)?,
        postal_code: row.get(8)?,
        country: row.get(9)?,
    })
}

fn map_contact(row: &Row) -> rusqlite::Result<Contact> {
    let party_type_text: String = row.get(1)?;
    let party_type = PartyType::parse(&party_type_text).ok_or_else(|| {
        rusqlite::Error::FromSqlConversionFailure(
            1,
            rusqlite::types::Type::Text,
            "invalid party_type".into(),
        )
    })?;
    Ok(Contact {
        id: Some(row.get(0)?),
        party_type,
        party_id: row.get(2)?,
        name: row.get(3)?,
        email: row.get(4)?,
        phone: row.get(5)?,
        is_primary: row.get(6)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::organization::companies::Company;
    use crate::backend::repositories::organization_repository::{self, OrganizationRepository};

    fn repo_with_company() -> (PartyRepository, i64) {
        let db = Database::open_in_memory().unwrap();
        db.run_migrations(organization_repository::MIGRATIONS)
            .unwrap();
        db.run_migrations(MIGRATIONS).unwrap();

        let org_repo = OrganizationRepository::new(db.clone());
        let company = org_repo
            .create_company(&Company::new("Acme Trading Co", "USD"))
            .unwrap();

        (PartyRepository::new(db), company.id.unwrap())
    }

    #[test]
    fn creates_and_fetches_a_customer() {
        let (repo, company_id) = repo_with_company();
        let created = repo
            .create_customer(&Customer::new(company_id, "Acme Retail"))
            .unwrap();
        let fetched = repo.get_customer(created.id.unwrap()).unwrap();
        assert_eq!(fetched.name, "Acme Retail");
        assert_eq!(fetched.payment_terms_days, 30);
    }

    #[test]
    fn delete_customer_removes_an_unreferenced_customer() {
        let (repo, company_id) = repo_with_company();
        let created = repo
            .create_customer(&Customer::new(company_id, "Acme Retail"))
            .unwrap();

        repo.delete_customer(created.id.unwrap()).unwrap();

        assert!(repo.get_customer(created.id.unwrap()).is_err());
    }

    #[test]
    fn delete_customer_is_rejected_with_a_clear_message_when_still_referenced_by_an_invoice() {
        use crate::backend::accounting::accounting_rules::AccountType;
        use crate::backend::accounting::chart_of_accounts::Account;
        use crate::backend::accounting::fiscal_periods::FiscalPeriod;
        use crate::backend::repositories::accounting_repository::{self, AccountingRepository};
        use crate::backend::repositories::sales_repository::{self, SalesRepository};
        use crate::backend::sales::invoices::{Invoice, InvoiceLine};

        let (repo, company_id) = repo_with_company();
        repo.db
            .run_migrations(accounting_repository::MIGRATIONS)
            .unwrap();
        repo.db
            .run_migrations(crate::backend::audit::audit_logs::MIGRATIONS)
            .unwrap();
        repo.db
            .run_migrations(sales_repository::MIGRATIONS)
            .unwrap();

        let customer = repo
            .create_customer(&Customer::new(company_id, "Acme Retail"))
            .unwrap();

        let accounting = AccountingRepository::new(repo.db.clone());
        accounting
            .create_fiscal_period(&FiscalPeriod::new(
                company_id,
                "Open Period",
                date(2026, 1, 1),
                date(2030, 12, 31),
            ))
            .unwrap();
        let ar = accounting
            .create_account(&Account::new(
                company_id,
                "1100",
                "Accounts Receivable",
                AccountType::Asset,
            ))
            .unwrap();
        let revenue = accounting
            .create_account(&Account::new(
                company_id,
                "4000",
                "Sales Revenue",
                AccountType::Revenue,
            ))
            .unwrap();

        let sales = SalesRepository::new(repo.db.clone());
        let mut invoice = Invoice::new(
            company_id,
            customer.id.unwrap(),
            date(2026, 1, 5),
            date(2026, 2, 4),
        );
        invoice.lines.push(InvoiceLine::new(
            revenue.id.unwrap(),
            "Consulting",
            rust_decimal_macros::dec!(1),
            rust_decimal_macros::dec!(500),
        ));
        let created_invoice = sales.create_invoice(&invoice).unwrap();
        sales
            .post_invoice(created_invoice.id.unwrap(), ar.id.unwrap())
            .unwrap();

        let result = repo.delete_customer(customer.id.unwrap());

        assert!(matches!(result, Err(AppError::Validation(_))));
        assert!(result.unwrap_err().to_string().contains("still referenced"));
        assert!(repo.get_customer(customer.id.unwrap()).is_ok());
    }

    fn date(y: i32, m: u32, d: u32) -> chrono::NaiveDate {
        chrono::NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    #[test]
    fn creates_and_fetches_a_vendor() {
        let (repo, company_id) = repo_with_company();
        let mut vendor = Vendor::new(company_id, "Acme Supplies Ltd");
        vendor.company_name = Some("Acme Group".into());
        vendor.vendor_type = "Contractor".into();
        let created = repo.create_vendor(&vendor).unwrap();
        let fetched = repo.get_vendor(created.id.unwrap()).unwrap();
        assert_eq!(fetched.name, "Acme Supplies Ltd");
        assert_eq!(fetched.company_name.as_deref(), Some("Acme Group"));
        assert_eq!(fetched.vendor_type, "Contractor");

        let mut updated = fetched;
        updated.company_name = Some("Acme Holdings".into());
        repo.update_vendor(&updated).unwrap();
        assert_eq!(
            repo.list_vendors_for_company(company_id).unwrap()[0]
                .company_name
                .as_deref(),
            Some("Acme Holdings")
        );
    }

    #[test]
    fn delete_vendor_removes_an_unreferenced_vendor() {
        let (repo, company_id) = repo_with_company();
        let created = repo
            .create_vendor(&Vendor::new(company_id, "Acme Supplies"))
            .unwrap();

        repo.delete_vendor(created.id.unwrap()).unwrap();

        assert!(repo.get_vendor(created.id.unwrap()).is_err());
    }

    #[test]
    fn delete_vendor_is_rejected_with_a_clear_message_when_still_referenced_by_a_bill() {
        use crate::backend::accounting::accounting_rules::AccountType;
        use crate::backend::accounting::chart_of_accounts::Account;
        use crate::backend::accounting::fiscal_periods::FiscalPeriod;
        use crate::backend::purchases::bills::{Bill, BillLine};
        use crate::backend::repositories::accounting_repository::{self, AccountingRepository};
        use crate::backend::repositories::purchase_repository::{self, PurchaseRepository};

        let (repo, company_id) = repo_with_company();
        repo.db
            .run_migrations(accounting_repository::MIGRATIONS)
            .unwrap();
        repo.db
            .run_migrations(crate::backend::audit::audit_logs::MIGRATIONS)
            .unwrap();
        repo.db
            .run_migrations(purchase_repository::MIGRATIONS)
            .unwrap();

        let vendor = repo
            .create_vendor(&Vendor::new(company_id, "Acme Supplies"))
            .unwrap();

        let accounting = AccountingRepository::new(repo.db.clone());
        accounting
            .create_fiscal_period(&FiscalPeriod::new(
                company_id,
                "Open Period",
                date(2026, 1, 1),
                date(2030, 12, 31),
            ))
            .unwrap();
        let ap = accounting
            .create_account(&Account::new(
                company_id,
                "2000",
                "Accounts Payable",
                AccountType::Liability,
            ))
            .unwrap();
        let expense = accounting
            .create_account(&Account::new(
                company_id,
                "6000",
                "Office Supplies",
                AccountType::Expense,
            ))
            .unwrap();

        let purchases = PurchaseRepository::new(repo.db.clone());
        let mut bill = Bill::new(
            company_id,
            vendor.id.unwrap(),
            date(2026, 1, 5),
            date(2026, 2, 4),
        );
        bill.lines.push(BillLine::new(
            expense.id.unwrap(),
            "Office supplies",
            rust_decimal_macros::dec!(1),
            rust_decimal_macros::dec!(300),
        ));
        let created_bill = purchases.create_bill(&bill).unwrap();
        purchases
            .post_bill(created_bill.id.unwrap(), ap.id.unwrap())
            .unwrap();

        let result = repo.delete_vendor(vendor.id.unwrap());

        assert!(matches!(result, Err(AppError::Validation(_))));
        assert!(result.unwrap_err().to_string().contains("still referenced"));
        assert!(repo.get_vendor(vendor.id.unwrap()).is_ok());
    }

    #[test]
    fn addresses_round_trip_for_a_customer() {
        let (repo, company_id) = repo_with_company();
        let customer = repo
            .create_customer(&Customer::new(company_id, "Acme Retail"))
            .unwrap();
        repo.create_address(&Address::new(
            PartyType::Customer,
            customer.id.unwrap(),
            AddressLabel::Billing,
            "123 Main St",
            "Karachi",
            "PK",
        ))
        .unwrap();

        let addresses = repo
            .list_addresses_for_party(PartyType::Customer, customer.id.unwrap())
            .unwrap();
        assert_eq!(addresses.len(), 1);
        assert_eq!(addresses[0].label, AddressLabel::Billing);
    }

    #[test]
    fn creating_a_new_primary_contact_demotes_the_previous_one() {
        let (repo, company_id) = repo_with_company();
        let vendor = repo
            .create_vendor(&Vendor::new(company_id, "Acme Supplies Ltd"))
            .unwrap();

        let mut first = Contact::new(PartyType::Vendor, vendor.id.unwrap(), "Jane Doe");
        first.is_primary = true;
        let first = repo.create_contact(&first).unwrap();

        let mut second = Contact::new(PartyType::Vendor, vendor.id.unwrap(), "John Smith");
        second.is_primary = true;
        repo.create_contact(&second).unwrap();

        let contacts = repo
            .list_contacts_for_party(PartyType::Vendor, vendor.id.unwrap())
            .unwrap();
        let refetched_first = contacts.iter().find(|c| c.id == first.id).unwrap();
        assert!(!refetched_first.is_primary);
        let refetched_second = contacts.iter().find(|c| c.name == "John Smith").unwrap();
        assert!(refetched_second.is_primary);
    }
}
