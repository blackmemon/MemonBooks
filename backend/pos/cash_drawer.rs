//! Cash drawer movements within a POS session: cash sales/refunds and
//! manual paid-in/paid-out events (e.g. a cash drop to the safe, or
//! petty cash taken from the drawer). Used to compute what the drawer
//! *should* hold at close, to compare against what's actually counted.

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;

use crate::backend::core::errors::{AppError, AppResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CashDrawerEventType {
    CashSale,
    CashRefund,
    PaidIn,
    PaidOut,
}

impl CashDrawerEventType {
    pub fn as_str(self) -> &'static str {
        match self {
            CashDrawerEventType::CashSale => "cash_sale",
            CashDrawerEventType::CashRefund => "cash_refund",
            CashDrawerEventType::PaidIn => "paid_in",
            CashDrawerEventType::PaidOut => "paid_out",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "cash_sale" => Some(CashDrawerEventType::CashSale),
            "cash_refund" => Some(CashDrawerEventType::CashRefund),
            "paid_in" => Some(CashDrawerEventType::PaidIn),
            "paid_out" => Some(CashDrawerEventType::PaidOut),
            _ => None,
        }
    }

    /// Whether this event type adds cash to the drawer (true) or removes
    /// it (false).
    fn increases_drawer(self) -> bool {
        matches!(
            self,
            CashDrawerEventType::CashSale | CashDrawerEventType::PaidIn
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CashDrawerEvent {
    pub id: Option<i64>,
    pub session_id: i64,
    pub event_type: CashDrawerEventType,
    pub amount: Decimal,
    pub reason: Option<String>,
    pub occurred_at: DateTime<Utc>,
}

impl CashDrawerEvent {
    pub fn new(session_id: i64, event_type: CashDrawerEventType, amount: Decimal) -> Self {
        Self {
            id: None,
            session_id,
            event_type,
            amount,
            reason: None,
            occurred_at: Utc::now(),
        }
    }

    pub fn validate(&self) -> AppResult<()> {
        if self.session_id <= 0 {
            return Err(AppError::Validation(
                "cash drawer event must reference a valid session".into(),
            ));
        }
        if self.amount <= Decimal::ZERO {
            return Err(AppError::Validation(
                "cash drawer event amount must be positive".into(),
            ));
        }
        Ok(())
    }

    fn signed_amount(&self) -> Decimal {
        if self.event_type.increases_drawer() {
            self.amount
        } else {
            -self.amount
        }
    }
}

/// What the drawer should hold at close: opening float plus every
/// event's signed effect.
pub fn expected_closing_cash(opening_cash: Decimal, events: &[CashDrawerEvent]) -> Decimal {
    opening_cash
        + events
            .iter()
            .map(CashDrawerEvent::signed_amount)
            .sum::<Decimal>()
}

/// Positive means the drawer has more than expected (over), negative
/// means less (short).
pub fn over_or_short(expected: Decimal, counted: Decimal) -> Decimal {
    counted - expected
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn event(event_type: CashDrawerEventType, amount: Decimal) -> CashDrawerEvent {
        CashDrawerEvent::new(1, event_type, amount)
    }

    #[test]
    fn expected_closing_cash_combines_opening_float_and_events() {
        let events = vec![
            event(CashDrawerEventType::CashSale, dec!(500)),
            event(CashDrawerEventType::CashRefund, dec!(50)),
            event(CashDrawerEventType::PaidOut, dec!(20)),
        ];
        // 200 float + 500 sales - 50 refund - 20 paid out = 630
        assert_eq!(expected_closing_cash(dec!(200), &events), dec!(630));
    }

    #[test]
    fn over_or_short_is_positive_when_drawer_has_more_than_expected() {
        assert_eq!(over_or_short(dec!(630), dec!(635)), dec!(5));
        assert_eq!(over_or_short(dec!(630), dec!(625)), dec!(-5));
        assert_eq!(over_or_short(dec!(630), dec!(630)), Decimal::ZERO);
    }

    #[test]
    fn validate_rejects_non_positive_amount() {
        let event = event(CashDrawerEventType::PaidIn, Decimal::ZERO);
        assert!(event.validate().is_err());
    }
}
