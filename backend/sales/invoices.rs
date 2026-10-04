//! A sales invoice: the document that actually creates AR and posts to
//! the general ledger (unlike a quotation). Posting an invoice debits
//! the AR control account for the total and credits each line's revenue
//! account, then records a `Receivable` for the same total -- see
//! `SalesRepository::post_invoice`.

use chrono::NaiveDate;
use rust_decimal::Decimal;

use crate::backend::accounting::accounts_receivable::receivables::Receivable;
use crate::backend::core::errors::{AppError, AppResult};
use crate::backend::core::validation::{require_non_empty, require_positive, Validate};
use crate::backend::transactions::transaction::Transaction;
use crate::backend::transactions::transaction_lines::TransactionLine;
use crate::backend::transactions::transaction_types::TransactionType;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InvoiceStatus {
    Draft,
    Posted,
    Voided,
}

impl InvoiceStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            InvoiceStatus::Draft => "draft",
            InvoiceStatus::Posted => "posted",
            InvoiceStatus::Voided => "voided",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "draft" => Some(InvoiceStatus::Draft),
            "posted" => Some(InvoiceStatus::Posted),
            "voided" => Some(InvoiceStatus::Voided),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct InvoiceLine {
    pub revenue_account_id: i64,
    pub description: String,
    pub quantity: Decimal,
    pub unit_price: Decimal,
}

impl InvoiceLine {
    pub fn new(
        revenue_account_id: i64,
        description: impl Into<String>,
        quantity: Decimal,
        unit_price: Decimal,
    ) -> Self {
        Self {
            revenue_account_id,
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
pub struct Invoice {
    pub id: Option<i64>,
    pub company_id: i64,
    pub customer_id: i64,
    pub invoice_date: NaiveDate,
    pub due_date: NaiveDate,
    pub status: InvoiceStatus,
    pub lines: Vec<InvoiceLine>,
    pub transaction_id: Option<i64>,
}

impl Invoice {
    pub fn new(
        company_id: i64,
        customer_id: i64,
        invoice_date: NaiveDate,
        due_date: NaiveDate,
    ) -> Self {
        Self {
            id: None,
            company_id,
            customer_id,
            invoice_date,
            due_date,
            status: InvoiceStatus::Draft,
            lines: Vec::new(),
            transaction_id: None,
        }
    }

    pub fn total(&self) -> Decimal {
        self.lines.iter().map(InvoiceLine::amount).sum()
    }

    /// Builds the transaction that posting this invoice would create:
    /// one debit to AR for the total, one credit per line to its revenue
    /// account. Doesn't post it -- that's `SalesRepository`'s job, since
    /// it also has to create the matching `Receivable` atomically.
    pub fn build_transaction(&self, ar_account_id: i64) -> Transaction {
        let mut transaction = Transaction::new(
            self.company_id,
            TransactionType::SalesInvoice,
            self.invoice_date,
        );
        transaction.description = Some(format!("Invoice for customer #{}", self.customer_id));
        transaction
            .lines
            .push(TransactionLine::debit(ar_account_id, self.total()));
        for line in &self.lines {
            transaction.lines.push(TransactionLine {
                description: Some(line.description.clone()),
                ..TransactionLine::credit(line.revenue_account_id, line.amount())
            });
        }
        transaction
    }

    pub fn build_receivable(&self) -> Receivable {
        let mut receivable = Receivable::new(
            self.company_id,
            self.customer_id,
            self.invoice_date,
            self.due_date,
            self.total(),
        );
        receivable.reference = self.id.map(|id| format!("INV-{id}"));
        receivable
    }
}

impl Validate for Invoice {
    fn validate(&self) -> AppResult<()> {
        if self.company_id <= 0 {
            return Err(AppError::Validation(
                "invoice must reference a valid company".into(),
            ));
        }
        if self.customer_id <= 0 {
            return Err(AppError::Validation(
                "invoice must reference a valid customer".into(),
            ));
        }
        if self.lines.is_empty() {
            return Err(AppError::Validation(
                "an invoice must have at least one line".into(),
            ));
        }
        if self.due_date < self.invoice_date {
            return Err(AppError::Validation(
                "invoice due_date must not be before invoice_date".into(),
            ));
        }
        for line in &self.lines {
            require_non_empty("invoice line description", &line.description)?;
            require_positive("invoice line quantity", line.quantity)?;
            require_positive("invoice line unit price", line.unit_price)?;
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

    fn invoice_with_two_lines() -> Invoice {
        let mut invoice = Invoice::new(1, 1, date(2026, 1, 1), date(2026, 1, 31));
        invoice
            .lines
            .push(InvoiceLine::new(10, "Consulting", dec!(5), dec!(100)));
        invoice
            .lines
            .push(InvoiceLine::new(11, "Widgets", dec!(2), dec!(25)));
        invoice
    }

    #[test]
    fn total_sums_line_amounts() {
        assert_eq!(invoice_with_two_lines().total(), dec!(550));
    }

    #[test]
    fn build_transaction_debits_ar_and_credits_each_revenue_line() {
        let invoice = invoice_with_two_lines();
        let transaction = invoice.build_transaction(99);

        assert!(transaction.is_balanced());
        assert_eq!(transaction.total_debits(), dec!(550));
        assert_eq!(transaction.lines[0].account_id, 99);
        assert_eq!(transaction.lines[0].debit, dec!(550));
        assert_eq!(transaction.transaction_type, TransactionType::SalesInvoice);
    }

    #[test]
    fn build_receivable_matches_the_invoice_total() {
        let invoice = invoice_with_two_lines();
        let receivable = invoice.build_receivable();
        assert_eq!(receivable.amount, dec!(550));
        assert_eq!(receivable.customer_id, invoice.customer_id);
    }

    #[test]
    fn validate_rejects_an_invoice_with_no_lines() {
        let invoice = Invoice::new(1, 1, date(2026, 1, 1), date(2026, 1, 31));
        assert!(invoice.validate().is_err());
    }
}
