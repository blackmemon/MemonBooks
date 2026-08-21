//! A quotation: a non-binding pre-sale offer to a customer. No GL or AR
//! impact -- that only happens if/when it's converted into an invoice.

use chrono::NaiveDate;
use rust_decimal::Decimal;

use crate::backend::core::errors::{AppError, AppResult};
use crate::backend::core::validation::{require_non_empty, require_positive, Validate};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QuotationStatus {
    Draft,
    Sent,
    Accepted,
    Rejected,
    Expired,
}

impl QuotationStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            QuotationStatus::Draft => "draft",
            QuotationStatus::Sent => "sent",
            QuotationStatus::Accepted => "accepted",
            QuotationStatus::Rejected => "rejected",
            QuotationStatus::Expired => "expired",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "draft" => Some(QuotationStatus::Draft),
            "sent" => Some(QuotationStatus::Sent),
            "accepted" => Some(QuotationStatus::Accepted),
            "rejected" => Some(QuotationStatus::Rejected),
            "expired" => Some(QuotationStatus::Expired),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct QuotationLine {
    pub description: String,
    pub quantity: Decimal,
    pub unit_price: Decimal,
}

impl QuotationLine {
    pub fn new(description: impl Into<String>, quantity: Decimal, unit_price: Decimal) -> Self {
        Self {
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
pub struct Quotation {
    pub id: Option<i64>,
    pub company_id: i64,
    pub customer_id: i64,
    pub quotation_date: NaiveDate,
    pub valid_until: NaiveDate,
    pub status: QuotationStatus,
    pub lines: Vec<QuotationLine>,
}

impl Quotation {
    pub fn new(company_id: i64, customer_id: i64, quotation_date: NaiveDate, valid_until: NaiveDate) -> Self {
        Self {
            id: None,
            company_id,
            customer_id,
            quotation_date,
            valid_until,
            status: QuotationStatus::Draft,
            lines: Vec::new(),
        }
    }

    pub fn total(&self) -> Decimal {
        self.lines.iter().map(QuotationLine::amount).sum()
    }
}

impl Validate for Quotation {
    fn validate(&self) -> AppResult<()> {
        if self.company_id <= 0 {
            return Err(AppError::Validation("quotation must reference a valid company".into()));
        }
        if self.customer_id <= 0 {
            return Err(AppError::Validation("quotation must reference a valid customer".into()));
        }
        if self.lines.is_empty() {
            return Err(AppError::Validation("a quotation must have at least one line".into()));
        }
        if self.valid_until < self.quotation_date {
            return Err(AppError::Validation("quotation valid_until must not be before quotation_date".into()));
        }
        for line in &self.lines {
            require_non_empty("quotation line description", &line.description)?;
            require_positive("quotation line quantity", line.quantity)?;
            require_positive("quotation line unit price", line.unit_price)?;
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

    #[test]
    fn total_sums_line_amounts() {
        let mut quotation = Quotation::new(1, 1, date(2026, 1, 1), date(2026, 1, 31));
        quotation.lines.push(QuotationLine::new("Widget", dec!(3), dec!(50)));
        quotation.lines.push(QuotationLine::new("Gadget", dec!(1), dec!(200)));
        assert_eq!(quotation.total(), dec!(350));
    }

    #[test]
    fn validate_rejects_a_quotation_with_no_lines() {
        let quotation = Quotation::new(1, 1, date(2026, 1, 1), date(2026, 1, 31));
        assert!(quotation.validate().is_err());
    }

    #[test]
    fn validate_rejects_valid_until_before_quotation_date() {
        let mut quotation = Quotation::new(1, 1, date(2026, 1, 31), date(2026, 1, 1));
        quotation.lines.push(QuotationLine::new("Widget", dec!(1), dec!(50)));
        assert!(quotation.validate().is_err());
    }

    #[test]
    fn as_str_and_parse_round_trip_for_every_status() {
        for status in [
            QuotationStatus::Draft,
            QuotationStatus::Sent,
            QuotationStatus::Accepted,
            QuotationStatus::Rejected,
            QuotationStatus::Expired,
        ] {
            assert_eq!(QuotationStatus::parse(status.as_str()), Some(status));
        }
    }
}
