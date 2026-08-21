//! A purchase order: a non-binding request to a vendor. No GL or AP
//! impact -- that only happens once a bill is received against it.

use chrono::NaiveDate;
use rust_decimal::Decimal;

use crate::backend::core::errors::{AppError, AppResult};
use crate::backend::core::validation::{require_non_empty, require_positive, Validate};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PurchaseOrderStatus {
    Draft,
    Sent,
    PartiallyReceived,
    Received,
    Cancelled,
}

impl PurchaseOrderStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            PurchaseOrderStatus::Draft => "draft",
            PurchaseOrderStatus::Sent => "sent",
            PurchaseOrderStatus::PartiallyReceived => "partially_received",
            PurchaseOrderStatus::Received => "received",
            PurchaseOrderStatus::Cancelled => "cancelled",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "draft" => Some(PurchaseOrderStatus::Draft),
            "sent" => Some(PurchaseOrderStatus::Sent),
            "partially_received" => Some(PurchaseOrderStatus::PartiallyReceived),
            "received" => Some(PurchaseOrderStatus::Received),
            "cancelled" => Some(PurchaseOrderStatus::Cancelled),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PurchaseOrderLine {
    pub description: String,
    pub quantity: Decimal,
    pub unit_price: Decimal,
}

impl PurchaseOrderLine {
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
pub struct PurchaseOrder {
    pub id: Option<i64>,
    pub company_id: i64,
    pub vendor_id: i64,
    pub order_date: NaiveDate,
    pub expected_date: NaiveDate,
    pub status: PurchaseOrderStatus,
    pub lines: Vec<PurchaseOrderLine>,
}

impl PurchaseOrder {
    pub fn new(company_id: i64, vendor_id: i64, order_date: NaiveDate, expected_date: NaiveDate) -> Self {
        Self {
            id: None,
            company_id,
            vendor_id,
            order_date,
            expected_date,
            status: PurchaseOrderStatus::Draft,
            lines: Vec::new(),
        }
    }

    pub fn total(&self) -> Decimal {
        self.lines.iter().map(PurchaseOrderLine::amount).sum()
    }
}

impl Validate for PurchaseOrder {
    fn validate(&self) -> AppResult<()> {
        if self.company_id <= 0 {
            return Err(AppError::Validation("purchase order must reference a valid company".into()));
        }
        if self.vendor_id <= 0 {
            return Err(AppError::Validation("purchase order must reference a valid vendor".into()));
        }
        if self.lines.is_empty() {
            return Err(AppError::Validation("a purchase order must have at least one line".into()));
        }
        if self.expected_date < self.order_date {
            return Err(AppError::Validation("purchase order expected_date must not be before order_date".into()));
        }
        for line in &self.lines {
            require_non_empty("purchase order line description", &line.description)?;
            require_positive("purchase order line quantity", line.quantity)?;
            require_positive("purchase order line unit price", line.unit_price)?;
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
        let mut po = PurchaseOrder::new(1, 1, date(2026, 1, 1), date(2026, 1, 15));
        po.lines.push(PurchaseOrderLine::new("Raw materials", dec!(10), dec!(20)));
        assert_eq!(po.total(), dec!(200));
    }

    #[test]
    fn validate_rejects_a_purchase_order_with_no_lines() {
        let po = PurchaseOrder::new(1, 1, date(2026, 1, 1), date(2026, 1, 15));
        assert!(po.validate().is_err());
    }
}
