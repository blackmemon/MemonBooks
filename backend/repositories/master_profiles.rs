//! Extended master fields reuse the established accounting/party/inventory tables.
use super::document_repository::linked_id;
use crate::backend::{core::errors::AppResult, transactions::documents::*};
use rusqlite::{params, Connection, OptionalExtension};
pub fn table(kind: Kind) -> Option<&'static str> {
    match kind {
        Kind::Account => Some("accounts"),
        Kind::Vendor => Some("vendors"),
        Kind::Customer => Some("customers"),
        Kind::Item => Some("products"),
        Kind::Location => Some("warehouses"),
        _ => None,
    }
}
pub fn merge(
    c: &Connection,
    company: i64,
    kind: Kind,
    mut profiles: Vec<Document>,
) -> AppResult<Vec<Document>> {
    let Some(table) = table(kind) else {
        return Ok(profiles);
    };
    let mut q = c.prepare(&format!(
        "SELECT id FROM {table} WHERE company_id=? ORDER BY id"
    ))?;
    let ids = q
        .query_map([company], |r| r.get::<_, i64>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    for id in ids {
        let pos = profiles
            .iter()
            .position(|d| d.get("Master ID") == id.to_string());
        let mut d = pos.map(|i| profiles[i].clone()).unwrap_or_else(|| {
            let mut d = Document::new(company, kind);
            d.id = Some(-id);
            d.number = format!("{}-{id:06}", kind.prefix());
            d.set("Master ID", id.to_string());
            d
        });
        let columns: Vec<(&str, &str)> = match kind {
            Kind::Vendor => vec![
                ("name", "Vendor Name"),
                ("email", "Email"),
                ("phone", "Phone"),
            ],
            Kind::Customer => vec![
                ("name", "Customer Name"),
                ("email", "Email"),
                ("phone", "Phone"),
                ("tax_id", "NTN"),
            ],
            Kind::Item => vec![
                ("name", "Item Name"),
                ("unit_of_measure", "Unit of Measure"),
                ("cost_price", "Purchase Price"),
                ("sale_price", "Sale Price"),
            ],
            Kind::Location => vec![("name", "Location Name"), ("address", "Mailing Address")],
            Kind::Account => vec![
                ("account_name", "Account Name"),
                ("description", "Description"),
            ],
            _ => vec![],
        };
        for (col, key) in columns {
            let value: String = c.query_row(
                &format!("SELECT COALESCE({col},'') FROM {table} WHERE id=?"),
                [id],
                |r| r.get(0),
            )?;
            d.set(key, value);
        }
        if kind == Kind::Account {
            let (value, status, parent): (String, String, Option<i64>) = c.query_row(
                "SELECT account_type,status,parent_account FROM accounts WHERE id=?",
                [id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )?;
            let mut chars = value.chars();
            let title = chars
                .next()
                .map(|ch| ch.to_uppercase().to_string() + chars.as_str())
                .unwrap_or_default();
            d.set("Account Type", title);
            d.status = status;
            if let Some(parent) = parent {
                d.set("Parent Account", parent.to_string());
            }
        } else {
            let active: bool = c.query_row(
                &format!("SELECT is_active FROM {table} WHERE id=?"),
                [id],
                |r| r.get(0),
            )?;
            d.status = if active { "Active" } else { "Inactive" }.into();
        }
        if let Some(pos) = pos {
            profiles[pos] = d;
        } else {
            profiles.push(d);
        }
    }
    Ok(profiles)
}
pub fn sync(tx: &rusqlite::Transaction, d: &mut Document) -> AppResult<()> {
    let Some(table) = table(d.kind) else {
        return Ok(());
    };
    let current = d.get("Master ID").parse::<i64>().ok();
    if let Some(id) = current {
        let valid: i64 = tx.query_row(
            &format!("SELECT count(*) FROM {table} WHERE id=? AND company_id=?"),
            params![id, d.company_id],
            |r| r.get(0),
        )?;
        if valid != 1 {
            return invalid("Master record belongs to another company or was deleted");
        }
    }
    let active = d.status == "Active";
    let id = match d.kind {
        Kind::Vendor | Kind::Customer => {
            let name = d.get(if d.kind == Kind::Vendor {
                "Vendor Name"
            } else {
                "Customer Name"
            });
            if let Some(id) = current {
                tx.execute(
                    &format!(
                        "UPDATE {table} SET name=?,email=?,phone=?,tax_id=?,is_active=? WHERE id=?"
                    ),
                    params![
                        name,
                        d.get("Email"),
                        d.get("Phone"),
                        d.get("NTN"),
                        active,
                        id
                    ],
                )?;
                id
            } else {
                let (extra_col, extra_val) = if d.kind == Kind::Customer {
                    (",credit_limit", ",'0'")
                } else {
                    ("", "")
                };
                tx.execute(&format!("INSERT INTO {table}(company_id,name,email,phone,tax_id,payment_terms_days,is_active,reference_code{extra_col}) VALUES (?,?,?,?,?,30,?,?{extra_val})"),params![d.company_id,name,d.get("Email"),d.get("Phone"),d.get("NTN"),active,d.number])?;
                tx.last_insert_rowid()
            }
        }
        Kind::Item => {
            let cost = num(&d.fields, "Purchase Price")?.to_string();
            let price = num(&d.fields, "Sale Price")?.to_string();
            let tracked = d.get("Item Type") != "Service";
            if let Some(id) = current {
                tx.execute("UPDATE products SET name=?,unit_of_measure=?,cost_price=?,sale_price=?,track_inventory=?,is_active=? WHERE id=?",params![d.get("Item Name"),d.get("Unit of Measure"),cost,price,tracked,active,id])?;
                id
            } else {
                tx.execute("INSERT INTO products(company_id,sku,name,unit_of_measure,cost_price,sale_price,track_inventory,is_active) VALUES (?,?,?,?,?,?,?,?)",params![d.company_id,d.number,d.get("Item Name"),d.get("Unit of Measure"),cost,price,tracked,active])?;
                tx.last_insert_rowid()
            }
        }
        Kind::Location => {
            if let Some(id) = current {
                tx.execute(
                    "UPDATE warehouses SET name=?,address=?,is_active=? WHERE id=?",
                    params![d.get("Location Name"), d.get("Mailing Address"), active, id],
                )?;
                id
            } else {
                tx.execute("INSERT INTO warehouses(company_id,name,code,address,is_active) VALUES (?,?,?,?,?)",params![d.company_id,d.get("Location Name"),d.number,d.get("Mailing Address"),active])?;
                tx.last_insert_rowid()
            }
        }
        Kind::Account => {
            let parent = if d.get("Parent Account").is_empty() {
                None
            } else {
                Some(linked_id(d.get("Parent Account"))?)
            };
            let mut ancestor = parent;
            let mut seen = std::collections::BTreeSet::new();
            while let Some(id) = ancestor {
                if Some(id) == current || !seen.insert(id) {
                    return invalid("Account parent would create a hierarchy cycle");
                }
                ancestor = tx
                    .query_row(
                        "SELECT parent_account FROM accounts WHERE id=? AND company_id=?",
                        params![id, d.company_id],
                        |r| r.get::<_, Option<i64>>(0),
                    )
                    .optional()?
                    .ok_or_else(|| {
                        crate::backend::core::errors::AppError::Validation(
                            "Parent must belong to this company".into(),
                        )
                    })?;
            }
            let account_type = d.get("Account Type").to_lowercase();
            if !["asset", "liability", "equity", "revenue", "expense"]
                .contains(&account_type.as_str())
            {
                return invalid("Choose a valid account type");
            }
            let now = chrono::Utc::now().timestamp();
            if let Some(id) = current {
                tx.execute("UPDATE accounts SET account_name=?,account_type=?,parent_account=?,status=?,description=?,modified_timestamp=? WHERE id=?",params![d.get("Account Name"),account_type,parent,d.status,d.get("Description"),now,id])?;
                id
            } else {
                tx.execute("INSERT INTO accounts(company_id,account_code,account_name,account_type,parent_account,status,description,create_timestamp,modified_timestamp) VALUES (?,?,?,?,?,?,?,?,?)",params![d.company_id,d.number,d.get("Account Name"),account_type,parent,d.status,d.get("Description"),now,now])?;
                tx.last_insert_rowid()
            }
        }
        _ => unreachable!(),
    };
    d.set("Master ID", id.to_string());
    if d.kind == Kind::Item {
        for (key, value) in &d.fields {
            if let Some(location) = key.strip_prefix("Stock at ") {
                if value.trim().is_empty() {
                    continue;
                }
                let location = linked_id(location)?;
                let target = num(&d.fields, key)?;
                let valid: i64 = tx.query_row(
                    "SELECT count(*) FROM warehouses WHERE id=? AND company_id=? AND is_active=1",
                    params![location, d.company_id],
                    |r| r.get(0),
                )?;
                if valid != 1 {
                    return invalid("Stock location must be active and belong to this company");
                }
                let mut query=tx.prepare("SELECT quantity_delta FROM stock_movements WHERE company_id=? AND product_id=? AND warehouse_id=?")?;
                let amounts = query
                    .query_map(params![d.company_id, id, location], |r| {
                        r.get::<_, String>(0)
                    })?
                    .collect::<Result<Vec<_>, _>>()?;
                let mut current = rust_decimal::Decimal::ZERO;
                for amount in amounts {
                    current += amount.parse::<rust_decimal::Decimal>().map_err(|_| {
                        crate::backend::core::errors::AppError::Internal(
                            "Invalid stock quantity".into(),
                        )
                    })?;
                }
                let delta = target - current;
                if delta != rust_decimal::Decimal::ZERO {
                    tx.execute("INSERT INTO stock_movements(company_id,product_id,warehouse_id,movement_date,movement_type,quantity_delta,reference) VALUES (?,?,?,?,?,?,?)",params![d.company_id,id,location,chrono::Utc::now().date_naive(),"adjustment",delta.to_string(),format!("master:{}",d.number)])?;
                }
            }
        }
    }
    Ok(())
}
pub fn remove(tx: &rusqlite::Transaction, d: &Document) -> AppResult<()> {
    if let Some(table) = table(d.kind) {
        if d.status != "Inactive" {
            return invalid("Archive master records before permanent deletion");
        }
        let id = linked_id(d.get("Master ID"))?;
        if d.kind == Kind::Account {
            tx.execute("DELETE FROM account_attachments WHERE account_id=?", [id])?;
        }
        tx.execute(
            &format!("DELETE FROM {table} WHERE id=? AND company_id=?"),
            params![id, d.company_id],
        )?;
    }
    Ok(())
}
