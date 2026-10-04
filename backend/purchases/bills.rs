//! A vendor bill: the document that actually creates AP and posts to the
//! general ledger. Mirrors `sales::invoices::Invoice` -- posting debits
//! each line's expense/asset account and credits the AP control account,
//! then records a matching `Payable`.

use chrono::NaiveDate;
use rust_decimal::Decimal;

use crate::backend::accounting::accounts_payable::payables::Payable;
use crate::backend::core::errors::{AppError, AppResult};
use crate::backend::core::validation::{require_non_empty, require_positive, Validate};
use crate::backend::transactions::transaction::Transaction;
use crate::backend::transactions::transaction_lines::TransactionLine;
use crate::backend::transactions::transaction_types::TransactionType;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BillStatus {
    Draft,
    Posted,
    Voided,
}

impl BillStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            BillStatus::Draft => "draft",
            BillStatus::Posted => "posted",
            BillStatus::Voided => "voided",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "draft" => Some(BillStatus::Draft),
            "posted" => Some(BillStatus::Posted),
            "voided" => Some(BillStatus::Voided),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BillLine {
    /// The expense (or asset) account this cost lands on.
    pub account_id: i64,
    pub description: String,
    pub quantity: Decimal,
    pub unit_price: Decimal,
}

impl BillLine {
    pub fn new(
        account_id: i64,
        description: impl Into<String>,
        quantity: Decimal,
        unit_price: Decimal,
    ) -> Self {
        Self {
            account_id,
            description: description.into(),
            quantity,
            unit_price,
        }
    }

    pub fn amount(&self) -> Decimal {
        self.quantity * self.unit_price
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Bill {
    pub id: Option<i64>,
    pub company_id: i64,
    pub vendor_id: i64,
    pub bill_date: NaiveDate,
    pub due_date: NaiveDate,
    pub status: BillStatus,
    pub lines: Vec<BillLine>,
    pub transaction_id: Option<i64>,
}

impl Bill {
    pub fn new(company_id: i64, vendor_id: i64, bill_date: NaiveDate, due_date: NaiveDate) -> Self {
        Self {
            id: None,
            company_id,
            vendor_id,
            bill_date,
            due_date,
            status: BillStatus::Draft,
            lines: Vec::new(),
            transaction_id: None,
        }
    }

    pub fn total(&self) -> Decimal {
        self.lines.iter().map(BillLine::amount).sum()
    }

    pub fn build_transaction(&self, ap_account_id: i64) -> Transaction {
        let mut transaction = Transaction::new(
            self.company_id,
            TransactionType::PurchaseBill,
            self.bill_date,
        );
        transaction.description = Some(format!("Bill from vendor #{}", self.vendor_id));
        for line in &self.lines {
            transaction.lines.push(TransactionLine {
                description: Some(line.description.clone()),
                ..TransactionLine::debit(line.account_id, line.amount())
            });
        }
        transaction
            .lines
            .push(TransactionLine::credit(ap_account_id, self.total()));
        transaction
    }

    pub fn build_payable(&self) -> Payable {
        let mut payable = Payable::new(
            self.company_id,
            self.vendor_id,
            self.bill_date,
            self.due_date,
            self.total(),
        );
        payable.reference = self.id.map(|id| format!("BILL-{id}"));
        payable
    }
}

impl Validate for Bill {
    fn validate(&self) -> AppResult<()> {
        if self.company_id <= 0 {
            return Err(AppError::Validation(
                "bill must reference a valid company".into(),
            ));
        }
        if self.vendor_id <= 0 {
            return Err(AppError::Validation(
                "bill must reference a valid vendor".into(),
            ));
        }
        if self.lines.is_empty() {
            return Err(AppError::Validation(
                "a bill must have at least one line".into(),
            ));
        }
        if self.due_date < self.bill_date {
            return Err(AppError::Validation(
                "bill due_date must not be before bill_date".into(),
            ));
        }
        for line in &self.lines {
            require_non_empty("bill line description", &line.description)?;
            require_positive("bill line quantity", line.quantity)?;
            require_positive("bill line unit price", line.unit_price)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    fn bill_with_two_lines() -> Bill {
        let mut bill = Bill::new(1, 1, date(2026, 1, 1), date(2026, 1, 31));
        bill.lines
            .push(BillLine::new(50, "Office rent", dec!(1), dec!(1200)));
        bill.lines
            .push(BillLine::new(51, "Utilities", dec!(1), dec!(300)));
        bill
    }

    #[test]
    fn total_sums_line_amounts() {
        assert_eq!(bill_with_two_lines().total(), dec!(1500));
    }

    #[test]
    fn build_transaction_debits_each_expense_line_and_credits_ap() {
        let bill = bill_with_two_lines();
        let transaction = bill.build_transaction(99);

        assert!(transaction.is_balanced());
        assert_eq!(transaction.total_credits(), dec!(1500));
        let ap_line = transaction.lines.last().unwrap();
        assert_eq!(ap_line.account_id, 99);
        assert_eq!(ap_line.credit, dec!(1500));
    }

    #[test]
    fn build_payable_matches_the_bill_total() {
        let bill = bill_with_two_lines();
        let payable = bill.build_payable();
        assert_eq!(payable.amount, dec!(1500));
        assert_eq!(payable.vendor_id, bill.vendor_id);
    }
}
