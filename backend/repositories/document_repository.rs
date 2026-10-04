//! Company-scoped workflow storage. Document, ledger, subledger and stock changes commit together.
use crate::backend::accounting::fiscal_periods::{FiscalPeriod, FiscalPeriodStatus};
use crate::backend::core::errors::{AppError, AppResult};
use crate::backend::repositories::accounting_repository::AccountingRepository;
use crate::backend::transactions::documents::*;
use crate::backend::transactions::{transaction::Transaction, transaction_engine};
use crate::database::database::{Database, Migration};
use rusqlite::{params, Connection, OptionalExtension};
use rust_decimal::Decimal;
pub const MIGRATIONS: &[Migration] = &[Migration {
    version: 2000,
    name: "native_document_workflows",
    sql: include_str!("../../database/migrations/2000_native_workflows.sql"),
}];
#[derive(Clone)]
pub struct DocumentRepository {
    db: Database,
}
impl DocumentRepository {
    pub fn stock_report(&self, company: i64, location: i64) -> AppResult<String> {
        let inventory = super::inventory_repository::InventoryRepository::new(self.db.clone());
        let products = inventory.list_products_for_company(company)?;
        let mut output = "Item Code — Item Name — Quantity\n".to_string();
        let mut total = Decimal::ZERO;
        for p in products {
            let quantity =
                inventory.on_hand_quantity(company, p.id.unwrap_or_default(), location)?;
            total += quantity;
            output.push_str(&format!("{} — {} — {}\n", p.sku, p.name, quantity));
        }
        output.push_str(&format!("Total units: {total}"));
        Ok(output)
    }
    pub fn new(db: Database) -> Self {
        Self { db }
    }
    pub fn list(&self, company: i64, kind: Kind) -> AppResult<Vec<Document>> {
        self.db.with_connection(|c| {
            let mut q = c.prepare(
                "SELECT id FROM workflow_documents WHERE company_id=? AND kind=? ORDER BY id DESC",
            )?;
            let ids = q
                .query_map(params![company, kind.label()], |r| r.get::<_, i64>(0))?
                .collect::<Result<Vec<_>, _>>()?;
            let records = ids
                .into_iter()
                .map(|id| read(c, company, id))
                .collect::<AppResult<Vec<_>>>()?;
            super::master_profiles::merge(c, company, kind, records)
        })
    }
    pub fn save(&self, d: &Document, posting: Option<&Transaction>) -> AppResult<Document> {
        let mut d = d.clone();
        d.validate()?;
        if d.id.is_some_and(|id| id < 0) && MASTERS.contains(&d.kind) {
            d.id = None;
        }
        self.db.with_transaction(|tx|{
 if let Some(id)=d.id{let previous=read(tx,d.company_id,id)?;if previous.locked(){return invalid("Posted, paid and cancelled records are locked");}if previous.revision!=d.revision{return invalid("Record changed since opening; reload before saving");}if previous.kind!=d.kind{return invalid("Cannot change document kind");}d.number=previous.number;}
 if d.kind==Kind::Payroll {let duplicate:i64=tx.query_row("SELECT count(*) FROM workflow_documents d JOIN workflow_fields e ON e.document_id=d.id AND e.row_no=-1 AND e.name='Employee' JOIN workflow_fields p ON p.document_id=d.id AND p.row_no=-1 AND p.name='Period' WHERE d.company_id=?1 AND d.kind='Payroll' AND e.value=?2 AND p.value=?3 AND d.id!=?4",params![d.company_id,d.get("Employee"),d.get("Period"),d.id.unwrap_or(0)],|r|r.get(0))?;if duplicate>0{return invalid("Payroll already exists for this employee and period");}}
 let id=if let Some(id)=d.id{id}else{tx.execute("INSERT INTO workflow_documents(company_id,kind,number,status) VALUES (?,?,NULL,?)",params![d.company_id,d.kind.label(),d.status])?;let id=tx.last_insert_rowid();d.number=format!("{}-{}-{id:06}",d.kind.prefix(),chrono::Utc::now().format("%Y"));id};
 if let Some(t)=posting{if t.company_id!=d.company_id{return invalid("Wrong posting company");}let period=tx.query_row("SELECT id,name,start_date,end_date,status FROM fiscal_periods WHERE company_id=?1 AND start_date<=?2 AND end_date>=?2 ORDER BY id LIMIT 1",params![d.company_id,t.transaction_date],|r|Ok((r.get::<_,i64>(0)?,r.get::<_,String>(1)?,r.get::<_,chrono::NaiveDate>(2)?,r.get::<_,chrono::NaiveDate>(3)?,r.get::<_,String>(4)?))).optional()?.ok_or_else(||AppError::Validation("No fiscal period covers the posting date".into()))?;let mut p=FiscalPeriod::new(d.company_id,period.1,period.2,period.3);p.id=Some(period.0);p.status=FiscalPeriodStatus::parse(&period.4).ok_or_else(||AppError::Validation("Invalid period status".into()))?;transaction_engine::validate_for_posting(t,&p)?;
 for line in &t.lines{let valid:i64=tx.query_row("SELECT count(*) FROM accounts WHERE id=? AND company_id=? AND status='Active'",params![line.account_id,d.company_id],|r|r.get(0))?;if valid!=1{return invalid("Posting account must be active and belong to this company");}}
 let mut t=t.clone();t.description=Some(format!("{} — {}",d.number,d.get("Notes/Memo")));let transaction_id=AccountingRepository::insert_transaction_and_lines(tx,&t)?;d.transaction_id=Some(transaction_id);post_subledger(tx,&d,id,transaction_id)?;
 }else if (d.status=="Posted"&&d.kind.posts())||d.status=="Paid"{return invalid("This status requires a balanced accounting posting");}
 tx.execute("UPDATE workflow_documents SET number=?,status=?,transaction_id=?,revision=revision+1,modified_at=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id=?",params![d.number,d.status,d.transaction_id,id])?;
 super::master_profiles::sync(tx,&mut d)?;
 tx.execute("DELETE FROM workflow_fields WHERE document_id=?",[id])?;for (row,f) in std::iter::once((-1,&d.fields)).chain(d.lines.iter().enumerate().map(|(i,f)|(i as i64,f))){for(k,v)in f{tx.execute("INSERT INTO workflow_fields(document_id,row_no,name,value) VALUES (?,?,?,?)",params![id,row,k,v])?;}}
 tx.execute("DELETE FROM workflow_attachments WHERE document_id=?",[id])?;for path in &d.attachments{tx.execute("INSERT INTO workflow_attachments(document_id,path) VALUES (?,?)",params![id,path])?;}
 apply_stock(tx,&d,id)?;
 d.id=Some(id);d.revision+=1;Ok(d)
 })
    }
    pub fn delete(&self, company: i64, ids: &[i64]) -> AppResult<usize> {
        self.db.with_transaction(|tx| {
            for id in ids {
                let record=if *id<0 {let mut d=Document::new(company,Kind::Account);d.id=Some(*id);d.set("Master ID",(-id).to_string());d.status="Inactive".into();
                    // The visible master selector owns the entity type. Its profile records retain the
                    // canonical type and current inactive status for a company scoped delete.
                    let profile=tx.query_row("SELECT d.kind,d.status FROM workflow_documents d JOIN workflow_fields f ON f.document_id=d.id AND f.name='Master ID' AND f.value=? WHERE d.company_id=?",params![(-id).to_string(),company],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?))).optional()?;
                    if let Some((kind,status))=profile {d.kind=Kind::parse(&kind).ok_or_else(||AppError::Internal("Invalid master kind".into()))?;d.status=status;}else {continue;} d
                } else {read(tx,company,*id)?};
                if record.locked() {
                    return invalid("Cannot delete posted, paid or cancelled records");
                }
            }
            for id in ids {
                if *id<0 {let master=tx.query_row("SELECT d.id,d.kind FROM workflow_documents d JOIN workflow_fields f ON f.document_id=d.id AND f.name='Master ID' AND f.value=? WHERE d.company_id=?",params![(-id).to_string(),company],|r|Ok((r.get::<_,i64>(0)?,r.get::<_,String>(1)?))).optional()?;if let Some((profile_id,kind))=master{let mut d=read(tx,company,profile_id)?;d.kind=Kind::parse(&kind).ok_or_else(||AppError::Internal("Invalid master kind".into()))?;super::master_profiles::remove(tx,&d)?;tx.execute("DELETE FROM workflow_documents WHERE id=?",[profile_id])?;}continue;}
                super::master_profiles::remove(tx,&read(tx,company,*id)?)?;
                tx.execute(
                    "DELETE FROM stock_movements WHERE reference=?",
                    [format!("workflow:{id}")],
                )?;
                tx.execute(
                    "DELETE FROM workflow_documents WHERE company_id=? AND id=?",
                    params![company, id],
                )?;
            }
            Ok(ids.len())
        })
    }
    pub fn save_template(&self, d: &Document, name: &str) -> AppResult<()> {
        if name.trim().is_empty() {
            return invalid("Template name is required");
        }
        self.db.with_transaction(|tx|{tx.execute("INSERT INTO workflow_templates(company_id,kind,name) VALUES (?,?,?) ON CONFLICT(company_id,kind,name) DO NOTHING",params![d.company_id,d.kind.label(),name])?;let id:i64=tx.query_row("SELECT id FROM workflow_templates WHERE company_id=? AND kind=? AND name=?",params![d.company_id,d.kind.label(),name],|r|r.get(0))?;tx.execute("DELETE FROM workflow_template_fields WHERE template_id=?",[id])?;for(row,f)in std::iter::once((-1,&d.fields)).chain(d.lines.iter().enumerate().map(|(i,f)|(i as i64,f))){for(k,v)in f {tx.execute("INSERT INTO workflow_template_fields VALUES (?,?,?,?)",params![id,row,k,v])?;}}Ok(())})
    }
    pub fn templates(&self, company: i64, kind: Kind) -> AppResult<Vec<String>> {
        self.db.with_connection(|c| {
            let mut q = c.prepare(
                "SELECT name FROM workflow_templates WHERE company_id=? AND kind=? ORDER BY name",
            )?;
            let v = q
                .query_map(params![company, kind.label()], |r| r.get(0))?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(v)
        })
    }
    pub fn template(&self, company: i64, kind: Kind, name: &str) -> AppResult<Document> {
        self.db.with_connection(|c|{let id:i64=c.query_row("SELECT id FROM workflow_templates WHERE company_id=? AND kind=? AND name=?",params![company,kind.label(),name],|r|r.get(0))?;let mut d=Document::new(company,kind);d.lines.clear();let mut q=c.prepare("SELECT row_no,name,value FROM workflow_template_fields WHERE template_id=? ORDER BY row_no")?;let rows=q.query_map([id],|r|Ok((r.get::<_,i64>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?)))?;for row in rows{let(n,k,v)=row?;set_field(&mut d,n,k,v);}Ok(d)})
    }
}
fn set_field(d: &mut Document, n: i64, k: String, v: String) {
    if n < 0 {
        d.fields.insert(k, v);
    } else {
        while d.lines.len() <= n as usize {
            d.lines.push(Fields::new());
        }
        d.lines[n as usize].insert(k, v);
    }
}
fn read(c: &Connection, company: i64, id: i64) -> AppResult<Document> {
    let (kind,number,status,transaction_id,revision)=c.query_row("SELECT kind,number,status,transaction_id,revision FROM workflow_documents WHERE company_id=? AND id=?",params![company,id],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,Option<i64>>(3)?,r.get::<_,i64>(4)?)))?;
    let mut d = Document::new(
        company,
        Kind::parse(&kind).ok_or_else(|| AppError::Validation("Unknown document kind".into()))?,
    );
    d.id = Some(id);
    d.number = number;
    d.status = status;
    d.transaction_id = transaction_id;
    d.revision = revision;
    d.lines.clear();
    let mut q = c.prepare(
        "SELECT row_no,name,value FROM workflow_fields WHERE document_id=? ORDER BY row_no",
    )?;
    for row in q.query_map([id], |r| {
        Ok((
            r.get::<_, i64>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
        ))
    })? {
        let (n, k, v) = row?;
        set_field(&mut d, n, k, v);
    }
    let mut a =
        c.prepare("SELECT path FROM workflow_attachments WHERE document_id=? ORDER BY id")?;
    d.attachments = a
        .query_map([id], |r| r.get(0))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(d)
}
pub fn linked_id(s: &str) -> AppResult<i64> {
    s.split(" · ")
        .next()
        .unwrap_or("")
        .parse()
        .map_err(|_| AppError::Validation("Choose a linked master record".into()))
}
fn apply_stock(tx: &rusqlite::Transaction, d: &Document, id: i64) -> AppResult<()> {
    use Kind::*;
    let direction = match d.kind {
        GoodsReceipt => Decimal::ONE,
        DeliveryChallan => -Decimal::ONE,
        SaleInvoice if d.get("Link to Delivery Challan").is_empty() => -Decimal::ONE,
        SaleReturn if d.status == "Posted" => Decimal::ONE,
        PurchaseReturn if d.status == "Posted" => -Decimal::ONE,
        _ => Decimal::ZERO,
    };
    let reference = format!("workflow:{id}");
    tx.execute(
        "DELETE FROM stock_movements WHERE reference=?",
        [&reference],
    )?;
    if direction == Decimal::ZERO || d.status == "Cancelled" {
        return Ok(());
    }
    let warehouse = linked_id(d.get("Location"))?;
    let valid: i64 = tx.query_row(
        "SELECT count(*) FROM warehouses WHERE id=? AND company_id=? AND is_active=1",
        params![warehouse, d.company_id],
        |r| r.get(0),
    )?;
    if valid != 1 {
        return invalid("Choose an active location in this company");
    }
    for l in &d.lines {
        let quantity = num(
            l,
            if d.kind == GoodsReceipt {
                "Accepted"
            } else if d.kind == DeliveryChallan {
                "Delivered"
            } else {
                "Qty"
            },
        )?;
        if quantity == Decimal::ZERO {
            continue;
        }
        let product = linked_id(get(l, "Item"))?;
        let tracked: Option<bool> = tx
            .query_row(
                "SELECT track_inventory FROM products WHERE id=? AND company_id=? AND is_active=1",
                params![product, d.company_id],
                |r| r.get(0),
            )
            .optional()?;
        match tracked {
            Some(false) => continue,
            Some(true) => {}
            None => return invalid("Choose an active item in this company"),
        };
        tx.execute("INSERT INTO stock_movements(company_id,product_id,warehouse_id,movement_date,movement_type,quantity_delta,reference,transaction_id) VALUES (?,?,?,?,?,?,?,?)",params![d.company_id,product,warehouse,parse_date(d.get("Date"))?,if direction>Decimal::ZERO{"purchase_receipt"}else{"sale_issue"},(quantity*direction).to_string(),reference,d.transaction_id])?;
    }
    Ok(())
}
fn post_subledger(
    tx: &rusqlite::Transaction,
    d: &Document,
    document_id: i64,
    transaction_id: i64,
) -> AppResult<()> {
    use Kind::*;
    let sale = matches!(d.kind, SaleInvoice | SaleReturn | CashReceipt | BankReceipt);
    let purchase = matches!(
        d.kind,
        PurchaseInvoice | PurchaseReturn | CashPayment | BankPayment
    );
    if !sale && !purchase {
        return Ok(());
    }
    if matches!(
        d.kind,
        CashReceipt | BankReceipt | CashPayment | BankPayment
    ) && d.get("Receipt Type") == "Petty"
    {
        return Ok(());
    }
    let (table, party_column, party_table, party_field, date_column) = if sale {
        (
            "receivables",
            "customer_id",
            "customers",
            "Customer",
            "invoice_date",
        )
    } else {
        ("payables", "vendor_id", "vendors", "Vendor", "bill_date")
    };
    let is_invoice = matches!(d.kind, SaleInvoice | PurchaseInvoice);
    let parties: Vec<(i64, Decimal)> =
        if is_invoice || matches!(d.kind, SaleReturn | PurchaseReturn) {
            vec![(
                linked_id(d.get(party_field))?,
                num(&d.fields, "Total Amount")?,
            )]
        } else {
            d.lines
                .iter()
                .map(|l| {
                    Ok((
                        linked_id(if get(l, party_field).is_empty() {
                            d.get(party_field)
                        } else {
                            get(l, party_field)
                        })?,
                        num(l, "Amount")?,
                    ))
                })
                .collect::<AppResult<_>>()?
        };
    for (party, amount) in parties {
        let valid: i64 = tx.query_row(
            &format!("SELECT count(*) FROM {party_table} WHERE id=? AND company_id=?"),
            params![party, d.company_id],
            |r| r.get(0),
        )?;
        if valid != 1 {
            return invalid("Party must belong to this company");
        }
        if is_invoice {
            let paid = num(
                &d.fields,
                if sale {
                    "Amount Received"
                } else {
                    "Amount Paid"
                },
            )?;
            let date = parse_date(d.get("Date"))?;
            let due = if d.get("Due Date").is_empty() {
                date
            } else {
                parse_date(d.get("Due Date"))?
            };
            tx.execute(&format!("INSERT INTO {table}(company_id,{party_column},reference,{date_column},due_date,amount,amount_paid,transaction_id) VALUES (?,?,?,?,?,?,?,?)"),params![d.company_id,party,d.number,date,due,amount.to_string(),paid.to_string(),transaction_id])?;
        } else {
            let mut remaining = amount;
            if matches!(d.kind, SaleReturn | PurchaseReturn) {
                remaining -= num(&d.fields, "Refund Amount")?;
            }
            let mut q=tx.prepare(&format!("SELECT id,amount,amount_paid FROM {table} WHERE company_id=? AND {party_column}=? ORDER BY {date_column},id"))?;
            let rows = q
                .query_map(params![d.company_id, party], |r| {
                    Ok((
                        r.get::<_, i64>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, String>(2)?,
                    ))
                })?
                .collect::<Result<Vec<_>, _>>()?;
            for (id, amount, paid) in rows {
                let original = amount
                    .parse::<Decimal>()
                    .map_err(|_| AppError::Internal("Invalid subledger amount".into()))?;
                let paid = paid
                    .parse::<Decimal>()
                    .map_err(|_| AppError::Internal("Invalid payment amount".into()))?;
                let allocation = remaining.min(original - paid).max(Decimal::ZERO);
                if allocation > Decimal::ZERO {
                    tx.execute(
                        &format!("UPDATE {table} SET amount_paid=? WHERE id=?"),
                        params![(paid + allocation).to_string(), id],
                    )?;
                    tx.execute("INSERT INTO workflow_allocations(document_id,subledger,entry_id,amount) VALUES (?,?,?,?)",params![document_id,table,id,allocation.to_string()])?;
                    remaining -= allocation;
                }
            }
            if remaining > Decimal::ZERO {
                tx.execute("INSERT INTO workflow_credits(company_id,party_type,party_id,transaction_id,amount) VALUES (?,?,?,?,?)",params![d.company_id,if sale{"Customer"}else{"Vendor"},party,transaction_id,remaining.to_string()])?;
            }
        }
    }
    Ok(())
}
