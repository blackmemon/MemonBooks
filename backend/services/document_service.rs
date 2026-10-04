//! Application workflows; Iced never accesses workflow tables directly.
use crate::backend::core::errors::{AppError, AppResult};
use crate::backend::repositories::{
    accounting_repository::AccountingRepository,
    document_repository::{linked_id, DocumentRepository},
};
use crate::backend::transactions::documents::*;
use crate::backend::transactions::{
    transaction::Transaction, transaction_lines::TransactionLine,
    transaction_types::TransactionType,
};
use crate::database::database::Database;
use rust_decimal::Decimal;
#[derive(Clone)]
pub struct DocumentService {
    repo: DocumentRepository,
    accounting: AccountingRepository,
}
impl DocumentService {
    pub fn import(
        &self,
        company: i64,
        kind: Kind,
        rows: &[Vec<String>],
    ) -> AppResult<(usize, usize)> {
        if !MASTERS.contains(&kind) {
            return invalid("Imports are available for Maintain records");
        }
        let Some(headers) = rows.first() else {
            return invalid("The import file is empty");
        };
        let allowed = kind.headers();
        let mut drafts = vec![];
        let mut skipped = 0;
        for row in &rows[1..] {
            let mut d = Document::new(company, kind);
            for (index, header) in headers.iter().enumerate() {
                if let Some(key) = allowed
                    .iter()
                    .find(|k| k.eq_ignore_ascii_case(header.trim()))
                {
                    d.set(key, row.get(index).cloned().unwrap_or_default());
                }
            }
            if d.get(allowed[0]).trim().is_empty() {
                skipped += 1;
                continue;
            }
            d.validate()?;
            drafts.push(d);
        }
        let count = drafts.len();
        for d in drafts {
            self.save(&d)?;
        }
        Ok((count, skipped))
    }
    pub fn stock_report(&self, company: i64, location: &str) -> AppResult<String> {
        self.repo.stock_report(company, linked_id(location)?)
    }
    pub fn account_tree(&self, company: i64) -> AppResult<String> {
        let accounts = self.accounting.list_accounts_for_company(company)?;
        let mut output = String::new();
        for a in &accounts {
            let mut depth = 0;
            let mut parent = a.parent_account;
            let mut seen = std::collections::BTreeSet::new();
            while let Some(id) = parent {
                if !seen.insert(id) {
                    break;
                }
                depth += 1;
                parent = accounts
                    .iter()
                    .find(|a| a.id == Some(id))
                    .and_then(|a| a.parent_account);
            }
            output.push_str(&format!(
                "{}{} — {} ({})\n",
                "    ".repeat(depth),
                a.account_code,
                a.account_name,
                a.status
            ));
        }
        Ok(output)
    }
    pub fn new(db: Database) -> Self {
        Self {
            repo: DocumentRepository::new(db.clone()),
            accounting: AccountingRepository::new(db),
        }
    }
    pub fn list(&self, company: i64, kind: Kind) -> AppResult<Vec<Document>> {
        self.repo.list(company, kind)
    }
    pub fn save(&self, d: &Document) -> AppResult<Document> {
        let mut d = d.clone();
        d.validate()?;
        let posting = if (d.status == "Posted" && d.kind.posts()) || d.status == "Paid" {
            Some(self.posting(&d)?)
        } else {
            None
        };
        self.repo.save(&d, posting.as_ref())
    }
    pub fn delete(&self, company: i64, ids: &[i64]) -> AppResult<usize> {
        self.repo.delete(company, ids)
    }
    pub fn save_template(&self, d: &Document, name: &str) -> AppResult<()> {
        self.repo.save_template(d, name)
    }
    pub fn templates(&self, company: i64, kind: Kind) -> AppResult<Vec<String>> {
        self.repo.templates(company, kind)
    }
    pub fn template(&self, company: i64, kind: Kind, name: &str) -> AppResult<Document> {
        self.repo.template(company, kind, name)
    }
    pub fn payroll_for(&self, employee: &Document, period: &str) -> AppResult<Document> {
        let mut d = Document::new(employee.company_id, Kind::Payroll);
        for key in d.kind.headers() {
            if employee.fields.contains_key(key) {
                d.set(key, employee.get(key));
            }
        }
        d.set(
            "Employee",
            format!(
                "{} · {} {}",
                employee
                    .id
                    .ok_or_else(|| AppError::Validation("Save employee first".into()))?,
                employee.get("First Name"),
                employee.get("Last Name")
            ),
        );
        d.set(
            "Employee Name",
            format!(
                "{} {}",
                employee.get("First Name"),
                employee.get("Last Name")
            ),
        );
        d.set("Period", period);
        d.set("Basic Salary", employee.get("Basic Salary / Hourly Rate"));
        d.recalculate()?;
        d.set("Days Worked", d.get("Days in Month").to_string());
        d.recalculate()?;
        Ok(d)
    }
    pub fn generate_payroll(
        &self,
        company: i64,
        period: &str,
        employee_ids: &[i64],
    ) -> AppResult<(usize, usize)> {
        let employees = self.list(company, Kind::Employee)?;
        let existing = self.list(company, Kind::Payroll)?;
        let mut created = 0;
        let mut skipped = 0;
        for e in employees
            .iter()
            .filter(|e| e.status == "Active" && e.id.is_some_and(|id| employee_ids.contains(&id)))
        {
            let d = self.payroll_for(e, period)?;
            if existing
                .iter()
                .any(|p| p.get("Employee") == d.get("Employee") && p.get("Period") == period)
            {
                skipped += 1;
            } else {
                self.save(&d)?;
                created += 1;
            }
        }
        Ok((created, skipped))
    }
    fn posting(&self, d: &Document) -> AppResult<Transaction> {
        use Kind::*;
        let accounts = self.accounting.list_accounts_for_company(d.company_id)?;
        let account = |names: &[&str]| -> AppResult<i64> {
            accounts
                .iter()
                .find(|a| a.status == "Active" && names.contains(&a.account_name.as_str()))
                .and_then(|a| a.id)
                .ok_or_else(|| {
                    AppError::Validation(format!("Add an active account named {}", names[0]))
                })
        };
        let date = if d.kind == Payroll {
            parse_date(d.get("Payment Date"))?
        } else {
            parse_date(d.get("Date"))?
        };
        let mut t = Transaction::new(
            d.company_id,
            match d.kind {
                PurchaseInvoice | PurchaseReturn => TransactionType::PurchaseBill,
                SaleInvoice | SaleReturn => TransactionType::SalesInvoice,
                CashReceipt | BankReceipt => TransactionType::CustomerPayment,
                CashPayment | BankPayment => TransactionType::VendorPayment,
                Payroll => TransactionType::Payroll,
                _ => TransactionType::ManualJournal,
            },
            date,
        );
        let total = num(&d.fields, "Total Amount")?;
        match d.kind {
            JournalVoucher => {
                for l in &d.lines {
                    let mut line =
                        TransactionLine::debit(linked_id(get(l, "Account"))?, num(l, "Debit")?);
                    line.credit = num(l, "Credit")?;
                    line.description = Some(get(l, "Narration").into());
                    t.lines.push(line);
                }
            }
            PurchaseInvoice | SaleInvoice => {
                let sale = d.kind == SaleInvoice;
                let control = if sale {
                    account(&["Accounts Receivable (Debtors)", "Accounts Receivable"])?
                } else {
                    account(&["Accounts Payable (Creditors)", "Accounts Payable"])?
                };
                let other = if sale {
                    account(&["Sales", "Sales Revenue"])?
                } else {
                    account(&["Inventory"])?
                };
                t.lines.push(TransactionLine::debit(
                    if sale { control } else { other },
                    total,
                ));
                t.lines.push(TransactionLine::credit(
                    if sale { other } else { control },
                    total,
                ));
                let paid = num(
                    &d.fields,
                    if sale {
                        "Amount Received"
                    } else {
                        "Amount Paid"
                    },
                )?;
                if paid > Decimal::ZERO {
                    let bank = linked_id(d.get("Payment Account"))?;
                    t.lines.push(TransactionLine::debit(
                        if sale { bank } else { control },
                        paid,
                    ));
                    t.lines.push(TransactionLine::credit(
                        if sale { control } else { bank },
                        paid,
                    ));
                }
            }
            PurchaseReturn | SaleReturn => {
                let sale = d.kind == SaleReturn;
                let control = if sale {
                    account(&["Accounts Receivable (Debtors)", "Accounts Receivable"])?
                } else {
                    account(&["Accounts Payable (Creditors)", "Accounts Payable"])?
                };
                let other = if sale {
                    account(&["Sales", "Sales Revenue"])?
                } else {
                    account(&["Inventory"])?
                };
                t.lines.push(TransactionLine::debit(
                    if sale { other } else { control },
                    total,
                ));
                t.lines.push(TransactionLine::credit(
                    if sale { control } else { other },
                    total,
                ));
                let refund = num(&d.fields, "Refund Amount")?;
                if refund > total {
                    return invalid("Refund exceeds return amount");
                }
                if refund > Decimal::ZERO {
                    let bank = linked_id(d.get("Refund Account"))?;
                    t.lines.push(TransactionLine::debit(
                        if sale { control } else { bank },
                        refund,
                    ));
                    t.lines.push(TransactionLine::credit(
                        if sale { bank } else { control },
                        refund,
                    ));
                }
            }
            CashReceipt | BankReceipt | CashPayment | BankPayment => {
                let receipt = matches!(d.kind, CashReceipt | BankReceipt);
                let bank = linked_id(d.get(if matches!(d.kind, BankReceipt | BankPayment) {
                    "Bank Account"
                } else {
                    "Cash Account"
                }))?;
                let contra = if d.get("Receipt Type") == "Petty" {
                    linked_id(d.get("Contra Account"))?
                } else if receipt {
                    account(&["Accounts Receivable (Debtors)", "Accounts Receivable"])?
                } else {
                    account(&["Accounts Payable (Creditors)", "Accounts Payable"])?
                };
                if receipt {
                    t.lines.push(TransactionLine::debit(bank, total));
                    t.lines.push(TransactionLine::credit(contra, total));
                } else {
                    let withholding = num(&d.fields, "WHT Amount")?;
                    if withholding > total {
                        return invalid("Withholding exceeds payment");
                    }
                    t.lines.push(TransactionLine::debit(contra, total));
                    if total > withholding {
                        t.lines
                            .push(TransactionLine::credit(bank, total - withholding));
                    }
                    if withholding > Decimal::ZERO {
                        t.lines.push(TransactionLine::credit(
                            account(&["Taxes Payable"])?,
                            withholding,
                        ));
                    }
                }
            }
            Payroll => {
                let gross = num(&d.fields, "Gross Salary")?;
                let deductions = num(&d.fields, "Total Deductions")?;
                t.lines.push(TransactionLine::debit(
                    linked_id(d.get("Salary Expense Account"))?,
                    gross,
                ));
                if deductions > Decimal::ZERO {
                    t.lines.push(TransactionLine::credit(
                        linked_id(d.get("Deductions Account"))?,
                        deductions,
                    ));
                }
                let net = num(&d.fields, "Net Pay")?;
                if net > Decimal::ZERO {
                    t.lines.push(TransactionLine::credit(
                        linked_id(d.get("Payment Account"))?,
                        net,
                    ));
                }
            }
            _ => return invalid("This document does not post to the ledger"),
        }
        Ok(t)
    }
}
