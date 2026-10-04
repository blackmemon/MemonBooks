//! Native workflow definitions shared by validation, persistence and Iced forms.
use crate::backend::core::errors::{AppError, AppResult};
use chrono::{Datelike, NaiveDate, Utc};
use rust_decimal::Decimal;
use std::collections::BTreeMap;

pub type Fields = BTreeMap<String, String>;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    PurchaseOrder,
    PurchaseInvoice,
    PurchaseReturn,
    GoodsReceipt,
    CashReceipt,
    BankReceipt,
    CashPayment,
    BankPayment,
    JournalVoucher,
    Quotation,
    SaleOrder,
    DeliveryChallan,
    SaleInvoice,
    SaleReturn,
    Trip,
    Expense,
    Employee,
    Payroll,
    Vendor,
    Customer,
    Item,
    Location,
    Account,
}
pub const MASTERS: &[Kind] = &[
    Kind::Account,
    Kind::Vendor,
    Kind::Item,
    Kind::Customer,
    Kind::Location,
];
pub const PURCHASE: &[Kind] = &[
    Kind::PurchaseOrder,
    Kind::GoodsReceipt,
    Kind::PurchaseInvoice,
    Kind::PurchaseReturn,
];
pub const SALE: &[Kind] = &[
    Kind::Quotation,
    Kind::SaleOrder,
    Kind::DeliveryChallan,
    Kind::SaleInvoice,
    Kind::SaleReturn,
];
pub const MONEY: &[Kind] = &[
    Kind::CashReceipt,
    Kind::BankReceipt,
    Kind::CashPayment,
    Kind::BankPayment,
    Kind::JournalVoucher,
];
pub const ALL: &[Kind] = &[
    Kind::PurchaseOrder,
    Kind::PurchaseInvoice,
    Kind::PurchaseReturn,
    Kind::GoodsReceipt,
    Kind::CashReceipt,
    Kind::BankReceipt,
    Kind::CashPayment,
    Kind::BankPayment,
    Kind::JournalVoucher,
    Kind::Quotation,
    Kind::SaleOrder,
    Kind::DeliveryChallan,
    Kind::SaleInvoice,
    Kind::SaleReturn,
    Kind::Trip,
    Kind::Expense,
    Kind::Employee,
    Kind::Payroll,
    Kind::Vendor,
    Kind::Customer,
    Kind::Item,
    Kind::Location,
    Kind::Account,
];
impl std::fmt::Display for Kind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}
impl Kind {
    pub fn label(self) -> &'static str {
        match self {
            Self::PurchaseOrder => "Purchase Order",
            Self::PurchaseInvoice => "Purchase Invoice",
            Self::PurchaseReturn => "Purchase Return",
            Self::GoodsReceipt => "Goods Receipt",
            Self::CashReceipt => "Cash Receipt",
            Self::BankReceipt => "Bank Receipt",
            Self::CashPayment => "Cash Payment",
            Self::BankPayment => "Bank Payment",
            Self::JournalVoucher => "Journal Voucher",
            Self::Quotation => "Quotation",
            Self::SaleOrder => "Sale Order",
            Self::DeliveryChallan => "Delivery Challan",
            Self::SaleInvoice => "Sale Invoice",
            Self::SaleReturn => "Sale Return",
            Self::Trip => "Trip",
            Self::Expense => "Expense",
            Self::Employee => "Employee",
            Self::Payroll => "Payroll",
            Self::Vendor => "Vendors",
            Self::Customer => "Customers",
            Self::Item => "Items",
            Self::Location => "Inventory Locations",
            Self::Account => "Chart of Accounts",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        ALL.iter().copied().find(|k| k.label() == s)
    }
    pub fn prefix(self) -> &'static str {
        match self {
            Self::PurchaseOrder => "PO",
            Self::PurchaseInvoice => "PI",
            Self::PurchaseReturn => "PR",
            Self::GoodsReceipt => "GRN",
            Self::CashReceipt => "CR",
            Self::BankReceipt => "BR",
            Self::CashPayment => "CP",
            Self::BankPayment => "BP",
            Self::JournalVoucher => "JV",
            Self::Quotation => "QT",
            Self::SaleOrder => "SO",
            Self::DeliveryChallan => "DC",
            Self::SaleInvoice => "SI",
            Self::SaleReturn => "SR",
            Self::Trip => "TRIP",
            Self::Expense => "EXP",
            Self::Employee => "EMP",
            Self::Payroll => "PAY",
            Self::Vendor => "VEN",
            Self::Customer => "CUS",
            Self::Item => "ITM",
            Self::Location => "LOC",
            Self::Account => "COA",
        }
    }
    pub fn next(self) -> Option<Self> {
        match self {
            Self::PurchaseOrder => Some(Self::GoodsReceipt),
            Self::GoodsReceipt => Some(Self::PurchaseInvoice),
            Self::PurchaseInvoice => Some(Self::PurchaseReturn),
            Self::Quotation => Some(Self::SaleOrder),
            Self::SaleOrder => Some(Self::DeliveryChallan),
            Self::DeliveryChallan => Some(Self::SaleInvoice),
            Self::SaleInvoice => Some(Self::SaleReturn),
            _ => None,
        }
    }
    pub fn posts(self) -> bool {
        matches!(
            self,
            Self::PurchaseInvoice
                | Self::PurchaseReturn
                | Self::SaleInvoice
                | Self::SaleReturn
                | Self::CashReceipt
                | Self::BankReceipt
                | Self::CashPayment
                | Self::BankPayment
                | Self::JournalVoucher
                | Self::Payroll
        )
    }
    pub fn headers(self) -> Vec<&'static str> {
        use Kind::*;
        match self {
            Vendor => vec![
                "Vendor Name",
                "Company Name",
                "Phone",
                "Email",
                "Website",
                "Mailing Address",
                "Shipping Address",
                "Payment Terms",
                "Payment Method",
                "Notes/Memo",
            ],
            Customer => vec![
                "Customer Name",
                "Company Name",
                "Phone",
                "Email",
                "NTN",
                "STRN",
                "Mailing Address",
                "Shipping Address",
                "Description",
            ],
            Item => vec![
                "Item Name",
                "Item Type",
                "Category",
                "Sub-Category",
                "Unit of Measure",
                "Purchase Price",
                "Sale Price",
                "Description",
            ],
            Location => vec![
                "Location Name",
                "Location Type",
                "Contact Person",
                "Phone",
                "Email",
                "Capacity",
                "Mailing Address",
                "Shipping Address",
                "Description",
            ],
            Account => vec![
                "Account Name",
                "Account Type",
                "Parent Account",
                "Description",
            ],
            PurchaseOrder => vec![
                "Date",
                "Vendor",
                "Reference No",
                "Mailing Address",
                "Shipping Address",
                "Shipping Date",
                "Shipping Via",
            ],
            PurchaseInvoice => vec![
                "Date",
                "Link to GRN",
                "Vendor",
                "Vendor Invoice No",
                "Vendor Invoice Date",
                "Due Date",
                "PO Reference",
                "Mailing Address",
                "Shipping Address",
                "Location",
            ],
            PurchaseReturn => vec![
                "Date",
                "Vendor",
                "Purchase Invoice",
                "Mailing Address",
                "Shipping Address",
                "Reason for Return",
                "Location",
            ],
            GoodsReceipt => vec![
                "Date",
                "Link to PO",
                "Vendor",
                "Vendor Invoice No",
                "Location",
                "Mailing Address",
                "Shipping Address",
            ],
            Quotation => vec![
                "Date",
                "Valid Until",
                "Customer",
                "Customer Ref",
                "Mailing Address",
                "Shipping Address",
            ],
            SaleOrder => vec![
                "Date",
                "Link to Quotation",
                "Customer",
                "Customer PO",
                "Delivery Date",
                "Delivery Address",
            ],
            DeliveryChallan => vec![
                "Date",
                "Link to Sale Order",
                "Customer",
                "Vehicle No",
                "Driver Name",
                "Driver Phone",
                "Delivery Address",
                "Location",
            ],
            SaleInvoice => vec![
                "Date",
                "Link to Delivery Challan",
                "Link to Sale Order",
                "Customer",
                "Due Date",
                "Location",
                "Delivery Address",
            ],
            SaleReturn => vec![
                "Date",
                "Customer",
                "Sale Invoice",
                "Delivery Address",
                "Reason for Return",
                "Location",
            ],
            JournalVoucher => vec!["Date", "Voucher Type", "Reference", "Narration"],
            CashReceipt | BankReceipt | CashPayment | BankPayment => {
                let mut f = vec![
                    "Receipt Type",
                    "Date",
                    if matches!(self, CashReceipt | BankReceipt) {
                        "Customer"
                    } else {
                        "Vendor"
                    },
                    "Mailing Address",
                    "Shipping Address",
                    "Contra Account",
                    if matches!(self, BankReceipt | BankPayment) {
                        "Bank Account"
                    } else {
                        "Cash Account"
                    },
                    "Reference",
                    "Narration",
                ];
                if matches!(self, BankReceipt | BankPayment) {
                    f.extend(["Payment Mode", "Cheque No", "Cheque Date", "Drawee Bank"]);
                }
                f
            }
            Trip => vec![
                "Name",
                "Type",
                "Purpose",
                "Flight",
                "Depart from",
                "Departure Date",
                "Destination",
                "Return Date",
                "Budget limit",
                "Check-in",
                "Check-out",
                "Hotel",
            ],
            Expense => vec![
                "Subject",
                "Merchant",
                "Date",
                "Total",
                "Currency",
                "Reimbursable",
                "Category",
                "Description",
                "Employee Name",
                "Add to report",
                "Report Name",
            ],
            Employee => vec![
                "First Name",
                "Last Name",
                "National ID",
                "Date of Birth",
                "Gender",
                "Marital Status",
                "Mobile No",
                "Email Address",
                "Residential Address",
                "Emergency Contact Name",
                "Emergency Contact Phone",
                "Department",
                "Designation",
                "Employee Type",
                "Hire Date",
                "Leave Date",
                "Pay Schedule",
                "Work Location",
                "Transport Mode",
                "Experience Type",
                "Previous Company",
                "Previous Designation",
                "Previous Salary",
                "Previous Leave Date",
                "Qualification Name",
                "Qualification Degree",
                "Qualification Status",
                "Qualification Complete",
                "Pay Rate Type",
                "Basic Salary / Hourly Rate",
                "Overtime Eligibility",
                "KPI Allowance",
                "House Allowance",
                "Mobile Allowance",
                "Medical Allowance",
                "Transport Allowance",
                "Fuel Allowance",
                "Bonus Eligibility",
                "Commission / Incentive",
                "Employee PF",
                "Employer PF",
                "Increment Amount",
                "Gratuity",
                "Gratuity Amount",
                "Payment Method",
                "Bank Name",
                "Account IBAN",
            ],
            Payroll => vec![
                "Employee",
                "Employee Name",
                "Department",
                "Designation",
                "Pay Schedule",
                "Period",
                "Days in Month",
                "Days Worked",
                "Leave Days",
                "Absence Days",
                "Overtime Hours",
                "Basic Salary",
                "Overtime Eligibility",
                "Overtime Pay",
                "House Allowance",
                "Transport Allowance",
                "Mobile Allowance",
                "Medical Allowance",
                "Fuel Allowance",
                "KPI Allowance",
                "Bonus",
                "Commission",
                "Ad-Hoc Allowance",
                "Arrears",
                "Gross Salary",
                "Employee PF",
                "Employer PF",
                "Tax Deduction",
                "Loan Deduction",
                "Other Deductions",
                "Total Deductions",
                "Net Pay",
                "Payment Method",
                "Bank Name",
                "Account IBAN",
                "Payment Date",
                "Approved By",
                "Salary Expense Account",
                "Deductions Account",
                "Payment Account",
            ],
        }
    }
    pub fn lines(self) -> &'static [&'static str] {
        use Kind::*;
        match self {
            PurchaseOrder => &["Item", "Qty", "Rate", "Tax %", "Total"],
            PurchaseInvoice | PurchaseReturn | SaleReturn => &[
                "Item",
                "Description",
                "Qty",
                "Unit Price",
                "Discount",
                "Tax %",
                "Total",
            ],
            Quotation | SaleOrder | SaleInvoice => &[
                "Item",
                "Description",
                "Qty",
                "Price Type",
                "Unit Price",
                "Discount",
                "Tax %",
                "Total",
            ],
            GoodsReceipt => &[
                "Item",
                "Description",
                "Ordered",
                "Received",
                "Accepted",
                "Rejected",
                "Unit Cost",
                "Total",
            ],
            DeliveryChallan => &[
                "Item",
                "Description",
                "Ordered",
                "Delivered",
                "Unit Price",
                "Total",
            ],
            CashReceipt | BankReceipt => &["Customer", "Amount", "Invoices"],
            CashPayment | BankPayment => &[
                "Vendor",
                "Amount",
                "WHT %",
                "WHT Amount",
                "Net Amount",
                "Invoices",
            ],
            JournalVoucher => &["Account", "Narration", "Debit", "Credit"],
            _ => &[],
        }
    }
    pub fn footers(self) -> Vec<&'static str> {
        use Kind::*;
        match self {
            PurchaseInvoice => vec![
                "Amount Paid",
                "Payment Account",
                "Notes/Memo",
                "Total Amount",
                "Balance Due",
            ],
            SaleInvoice => vec![
                "Amount Received",
                "Payment Account",
                "Notes/Memo",
                "Total Amount",
                "Balance Due",
            ],
            PurchaseReturn | SaleReturn => vec![
                "Refund Amount",
                "Refund Account",
                "Notes/Memo",
                "Total Amount",
            ],
            PurchaseOrder => vec![
                "Total Qty",
                "Total Discount",
                "Total Amount",
                "Terms & Conditions",
                "Notes/Memo",
            ],
            Quotation | SaleOrder => vec!["Terms & Conditions", "Notes/Memo", "Total Amount"],
            GoodsReceipt => vec![
                "Total Received",
                "Total Accepted",
                "Total Rejected",
                "Total Amount",
                "Notes/Memo",
            ],
            CashPayment | BankPayment => vec![
                "Total Amount",
                "WHT Amount",
                "Net Amount to Pay",
                "Notes/Memo",
            ],
            CashReceipt | BankReceipt | DeliveryChallan => vec!["Total Amount", "Notes/Memo"],
            JournalVoucher => vec!["Total Debit", "Total Credit", "Notes/Memo"],
            Employee => vec!["Gross Salary", "Net Gross Salary", "Total Payable"],
            _ => vec![],
        }
    }
    pub fn statuses(self) -> &'static [&'static str] {
        match self {
            Self::Employee
            | Self::Vendor
            | Self::Customer
            | Self::Item
            | Self::Location
            | Self::Account => &["Active", "Inactive"],
            Self::Trip => &["Draft", "Planned", "Completed", "Cancelled"],
            Self::Expense => &["Draft", "Not Submitted", "Submitted"],
            Self::Payroll => &["Draft", "Approved", "Paid"],
            _ => &[
                "Draft",
                "Pending Approval",
                "Approved",
                "Posted",
                "Cancelled",
            ],
        }
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct Document {
    pub id: Option<i64>,
    pub company_id: i64,
    pub kind: Kind,
    pub number: String,
    pub status: String,
    pub fields: Fields,
    pub lines: Vec<Fields>,
    pub attachments: Vec<String>,
    pub transaction_id: Option<i64>,
    pub revision: i64,
}
impl Document {
    pub fn new(company_id: i64, kind: Kind) -> Self {
        let mut d = Self {
            id: None,
            company_id,
            kind,
            number: String::new(),
            status: kind.statuses()[0].into(),
            fields: Fields::new(),
            lines: vec![],
            attachments: vec![],
            transaction_id: None,
            revision: 0,
        };
        for f in kind.headers().into_iter().chain(kind.footers()) {
            let value = if matches!(
                f,
                "Leave Date" | "Previous Leave Date" | "Qualification Complete" | "Date of Birth"
            ) {
                String::new()
            } else if date_field(f) {
                Utc::now().date_naive().to_string()
            } else if f == "Period" {
                Utc::now().format("%Y-%m").to_string()
            } else if kind == Kind::Item && f == "Category" {
                "Product".to_string()
            } else if kind == Kind::Item && f == "Sub-Category" {
                "Raw Material".to_string()
            } else {
                options(f).first().unwrap_or(&"").to_string()
            };
            d.fields.insert(f.into(), value);
        }
        if !kind.lines().is_empty() {
            d.lines.push(Fields::new());
        }
        if kind == Kind::Trip {
            let future = (Utc::now().date_naive() + chrono::Duration::days(7)).to_string();
            d.set("Return Date", &future);
            d.set("Check-out", &future);
        }
        d
    }
    pub fn get(&self, k: &str) -> &str {
        get(&self.fields, k)
    }
    pub fn set(&mut self, k: &str, v: impl Into<String>) {
        self.fields.insert(k.into(), v.into());
    }
    pub fn locked(&self) -> bool {
        self.transaction_id.is_some()
            || matches!(self.status.as_str(), "Posted" | "Cancelled" | "Paid")
    }
    pub fn recalculate(&mut self) -> AppResult<()> {
        let mut total = Decimal::ZERO;
        let mut qty = Decimal::ZERO;
        let mut wht = Decimal::ZERO;
        let mut dr = Decimal::ZERO;
        let mut cr = Decimal::ZERO;
        for l in &mut self.lines {
            let quantity = match self.kind {
                Kind::GoodsReceipt => num(l, "Accepted")?,
                Kind::DeliveryChallan => num(l, "Delivered")?,
                _ => num(l, "Qty")?,
            };
            qty += quantity;
            let amount = if self.kind == Kind::JournalVoucher {
                dr += num(l, "Debit")?;
                cr += num(l, "Credit")?;
                Decimal::ZERO
            } else if MONEY.contains(&self.kind) {
                let a = num(l, "Amount")?;
                let w = (a * num(l, "WHT %")? / Decimal::from(100)).round_dp(2);
                wht += w;
                l.insert("WHT Amount".into(), w.to_string());
                l.insert("Net Amount".into(), (a - w).to_string());
                a
            } else {
                let rate = if self.kind == Kind::PurchaseOrder {
                    "Rate"
                } else if self.kind == Kind::GoodsReceipt {
                    "Unit Cost"
                } else {
                    "Unit Price"
                };
                let base = quantity * num(l, rate)? - num(l, "Discount")?;
                if base < Decimal::ZERO {
                    return invalid("Discount exceeds line amount");
                }
                (base * (Decimal::ONE + num(l, "Tax %")? / Decimal::from(100))).round_dp(2)
            };
            l.insert("Total".into(), amount.to_string());
            total += amount;
        }
        if !self.lines.is_empty() {
            self.set("Total Amount", total.to_string());
            self.set("Total Qty", qty.to_string());
            self.set(
                "Total Discount",
                self.lines
                    .iter()
                    .map(|l| num(l, "Discount"))
                    .collect::<AppResult<Vec<_>>>()?
                    .iter()
                    .sum::<Decimal>()
                    .to_string(),
            );
            self.set("WHT Amount", wht.to_string());
            self.set("Net Amount to Pay", (total - wht).to_string());
            self.set("Total Debit", dr.to_string());
            self.set("Total Credit", cr.to_string());
            let paid = num(
                &self.fields,
                if self.kind == Kind::PurchaseInvoice {
                    "Amount Paid"
                } else {
                    "Amount Received"
                },
            )?;
            if paid > total {
                return invalid("Payment exceeds total amount");
            }
            self.set("Balance Due", (total - paid).to_string());
        }
        if self.kind == Kind::GoodsReceipt {
            for (from, to) in [
                ("Received", "Total Received"),
                ("Accepted", "Total Accepted"),
                ("Rejected", "Total Rejected"),
            ] {
                let value = self
                    .lines
                    .iter()
                    .map(|l| num(l, from))
                    .collect::<AppResult<Vec<_>>>()?
                    .iter()
                    .sum::<Decimal>();
                self.set(to, value.to_string());
            }
        }
        if self.kind == Kind::Employee {
            let gross =
                num(&self.fields, "Basic Salary / Hourly Rate")? + sum(&self.fields, &ALLOWANCES)?;
            let net = gross - num(&self.fields, "Employee PF")?;
            self.set("Gross Salary", gross.to_string());
            self.set("Net Gross Salary", net.to_string());
            self.set("Total Payable", net.to_string());
        }
        if self.kind == Kind::Payroll {
            let start =
                NaiveDate::parse_from_str(&format!("{}-01", self.get("Period")), "%Y-%m-%d")
                    .map_err(|_| AppError::Validation("Period must be YYYY-MM".into()))?;
            let next = if start.month() == 12 {
                NaiveDate::from_ymd_opt(start.year() + 1, 1, 1)
            } else {
                NaiveDate::from_ymd_opt(start.year(), start.month() + 1, 1)
            }
            .ok_or_else(|| AppError::Validation("Invalid period".into()))?;
            let days = Decimal::from((next - start).num_days());
            self.set("Days in Month", days.to_string());
            let worked = num(&self.fields, "Days Worked")?;
            if worked > days {
                return invalid("Days worked exceeds days in month");
            }
            let base = num(&self.fields, "Basic Salary")?;
            let overtime = if self.get("Overtime Eligibility") == "Yes" {
                (base / days / Decimal::from(8) * num(&self.fields, "Overtime Hours")?).round_dp(2)
            } else {
                Decimal::ZERO
            };
            let gross = (base * worked / days
                + overtime
                + sum(&self.fields, &ALLOWANCES)?
                + sum(
                    &self.fields,
                    &["Bonus", "Commission", "Ad-Hoc Allowance", "Arrears"],
                )?)
            .round_dp(2);
            let deductions = sum(
                &self.fields,
                &[
                    "Employee PF",
                    "Tax Deduction",
                    "Loan Deduction",
                    "Other Deductions",
                ],
            )?;
            if deductions > gross {
                return invalid("Deductions exceed gross salary");
            }
            self.set("Overtime Pay", overtime.to_string());
            self.set("Gross Salary", gross.to_string());
            self.set("Total Deductions", deductions.to_string());
            self.set("Net Pay", (gross - deductions).to_string());
        }
        Ok(())
    }
    pub fn validate(&mut self) -> AppResult<()> {
        if !self.kind.statuses().contains(&self.status.as_str()) {
            return invalid("Invalid status");
        }
        if self.attachments.len() > 4 {
            return invalid("Maximum four attachments");
        }
        self.recalculate()?;
        for (key, value) in &self.fields {
            if date_field(key) && !value.is_empty() {
                parse_date(value)?;
            }
            if numeric_field(key) && !calculated(key) {
                num(&self.fields, key)?;
            }
        }
        let required: Vec<&str> = match self.kind {
            Kind::Trip if self.status != "Draft" => vec!["Name", "Depart from", "Destination"],
            Kind::Expense => vec!["Subject", "Merchant", "Total", "Employee Name"],
            Kind::Employee => vec!["First Name", "Last Name"],
            Kind::Payroll => vec!["Employee", "Period"],
            Kind::Vendor => vec!["Vendor Name", "Company Name", "Phone", "Mailing Address"],
            Kind::Customer => vec!["Customer Name"],
            Kind::Item => vec!["Item Name"],
            Kind::Location => vec!["Location Name"],
            Kind::Account => vec!["Account Name"],
            _ => vec![],
        };
        for key in required {
            if self.get(key).trim().is_empty() {
                return invalid(&format!("{key} is required"));
            }
        }
        if self.kind == Kind::Trip {
            for (a, b) in [("Departure Date", "Return Date"), ("Check-in", "Check-out")] {
                if parse_date(self.get(b))? < parse_date(self.get(a))? {
                    return invalid(&format!("{b} must not precede {a}"));
                }
            }
        }
        for key in ["Email", "Email Address"] {
            let email = self.get(key);
            if !email.is_empty()
                && (!email.contains('@') || !email.rsplit('@').next().unwrap_or("").contains('.'))
            {
                return invalid("Enter a valid email address");
            }
        }
        if self.kind == Kind::Account && self.get("Account Name").trim().len() < 3 {
            return invalid("Account name must contain at least three characters");
        }
        Ok(())
    }
    pub fn convert(&self) -> AppResult<Self> {
        if self.id.is_none() {
            return invalid("Save the document before converting");
        }
        let kind = self
            .kind
            .next()
            .ok_or_else(|| AppError::Validation("No conversion for this document".into()))?;
        let mut d = Self::new(self.company_id, kind);
        for (k, v) in &self.fields {
            if d.fields.contains_key(k) {
                d.set(k, v.clone());
            }
        }
        d.lines = self.lines.clone();
        let link = match self.kind {
            Kind::PurchaseOrder => "Link to PO",
            Kind::GoodsReceipt => "Link to GRN",
            Kind::PurchaseInvoice => "Purchase Invoice",
            Kind::Quotation => "Link to Quotation",
            Kind::SaleOrder => "Link to Sale Order",
            Kind::DeliveryChallan => "Link to Delivery Challan",
            _ => "Sale Invoice",
        };
        d.set(link, &self.number);
        for line in &mut d.lines {
            let q = get(line, "Qty").to_string();
            if kind == Kind::GoodsReceipt {
                line.insert("Ordered".into(), q.clone());
                line.insert("Received".into(), q.clone());
                line.insert("Accepted".into(), q);
                line.insert("Unit Cost".into(), get(line, "Rate").to_string());
            } else if kind == Kind::DeliveryChallan {
                line.insert("Ordered".into(), q.clone());
                line.insert("Delivered".into(), q);
            } else if self.kind == Kind::GoodsReceipt {
                line.insert("Qty".into(), get(line, "Accepted").to_string());
                line.insert("Unit Price".into(), get(line, "Unit Cost").to_string());
            } else if self.kind == Kind::DeliveryChallan {
                line.insert("Qty".into(), get(line, "Delivered").to_string());
            }
        }
        d.recalculate()?;
        Ok(d)
    }
}
pub const ALLOWANCES: [&str; 6] = [
    "House Allowance",
    "Transport Allowance",
    "Mobile Allowance",
    "Medical Allowance",
    "Fuel Allowance",
    "KPI Allowance",
];
pub fn get<'a>(f: &'a Fields, k: &str) -> &'a str {
    f.get(k).map(String::as_str).unwrap_or("")
}
pub fn invalid<T>(s: &str) -> AppResult<T> {
    Err(AppError::Validation(s.into()))
}
pub fn num(f: &Fields, k: &str) -> AppResult<Decimal> {
    let s = get(f, k).trim();
    if s.is_empty() {
        return Ok(Decimal::ZERO);
    }
    let n = s
        .parse::<Decimal>()
        .map_err(|_| AppError::Validation(format!("{k} must be a decimal number")))?;
    if n < Decimal::ZERO {
        return invalid(&format!("{k} cannot be negative"));
    }
    Ok(n)
}
fn sum(f: &Fields, keys: &[&str]) -> AppResult<Decimal> {
    keys.iter().map(|k| num(f, k)).sum()
}
pub fn parse_date(s: &str) -> AppResult<NaiveDate> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .or_else(|_| NaiveDate::parse_from_str(s, "%d/%m/%Y"))
        .map_err(|_| AppError::Validation("Enter a valid date (YYYY-MM-DD or DD/MM/YYYY)".into()))
}
pub fn date_field(s: &str) -> bool {
    s == "Date"
        || s == "Date of Birth"
        || s.ends_with("Date")
        || matches!(
            s,
            "Check-in" | "Check-out" | "Valid Until" | "Qualification Complete"
        )
}
pub fn calculated(s: &str) -> bool {
    matches!(
        s,
        "Total Amount"
            | "Total Qty"
            | "Total Discount"
            | "Balance Due"
            | "Total"
            | "Total Debit"
            | "Total Credit"
            | "WHT Amount"
            | "Net Amount"
            | "Net Amount to Pay"
            | "Total Received"
            | "Total Accepted"
            | "Total Rejected"
            | "Days in Month"
            | "Overtime Pay"
            | "Gross Salary"
            | "Total Deductions"
            | "Net Pay"
            | "Net Gross Salary"
            | "Total Payable"
    )
}
pub fn numeric_field(s: &str) -> bool {
    if s.ends_with("Account") {
        return false;
    }
    s.contains("Salary")
        || s.contains("Allowance")
        || s.ends_with("Amount")
        || s.ends_with("Price")
        || s.ends_with("Deduction")
        || s.ends_with("PF")
        || matches!(
            s,
            "Budget limit"
                | "Previous Salary"
                | "Qty"
                | "Rate"
                | "Discount"
                | "Tax %"
                | "Amount"
                | "WHT %"
                | "Debit"
                | "Credit"
                | "Accepted"
                | "Received"
                | "Rejected"
                | "Ordered"
                | "Delivered"
                | "Unit Cost"
                | "Days Worked"
                | "Leave Days"
                | "Absence Days"
                | "Overtime Hours"
                | "Bonus"
                | "Commission"
                | "Arrears"
                | "Other Deductions"
                | "Amount Paid"
                | "Amount Received"
                | "Total"
        )
}
pub fn options(s: &str) -> &'static [&'static str] {
    match s {
        "Account Type" => &["Asset", "Liability", "Equity", "Revenue", "Expense"],
        "Item Type" => &["Raw Material", "Finished Goods", "Service", "Other"],
        "Sub-Category" => &[
            "Metal",
            "Plastic",
            "Textile",
            "Chemical",
            "Retail",
            "Wholesale",
            "Export",
            "Labor",
            "Consulting",
            "Maintenance",
            "Other",
        ],
        "Unit of Measure" => &[
            "Pcs", "Kg", "Liter", "Box", "Pack", "Meter", "Dozen", "Unit", "Other",
        ],
        "Location Type" => &["Warehouse", "Store", "Office", "Other"],
        "Payment Terms" => &["Cash", "Cheque", "Bank transfer", "Digital Payment"],
        "Type" => &["International", "Domestic"],
        "Flight" => &["Roundtrip", "One-way"],
        "Currency" => &["EUR", "USD", "GBP", "PKR"],
        "Category" => &[
            "Travel",
            "Food",
            "Office Supplies",
            "Accommodation",
            "Entertainment",
            "Other",
        ],
        "Reimbursable"
        | "Overtime Eligibility"
        | "Bonus Eligibility"
        | "Commission / Incentive" => &["No", "Yes"],
        "Add to report" => &["Yes", "No"],
        "Price Type" => &["Retail", "Wholesale", "Custom"],
        "Shipping Via" => &["Road", "Air", "Sea", "Courier"],
        "Receipt Type" => &["Party", "Petty", "Multiparty"],
        "Payment Mode" => &["Cash", "Cheque", "NEFT", "RTGS", "UPI"],
        "Voucher Type" => &["Adjustment", "Transfer", "Accrual", "Opening", "Closing"],
        "Gender" => &["Male", "Female"],
        "Marital Status" => &["Single", "Married"],
        "Employee Type" => &["Full-time", "Part-time", "Contract"],
        "Pay Schedule" => &["Monthly", "Weekly"],
        "Transport Mode" => &["Public", "Own", "Company"],
        "Experience Type" => &["Fresh", "Previous"],
        "Qualification Status" => &["In Process", "Completed"],
        "Pay Rate Type" => &["Salary", "Hourly"],
        "Gratuity" => &["No", "Monthly", "Yes"],
        "Payment Method" => &["Bank", "Cash"],
        _ => &[],
    }
}
