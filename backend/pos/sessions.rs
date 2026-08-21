//! A POS session (cashier shift): opening a terminal with a starting
//! cash float, and closing it with a counted amount that may or may not
//! match what the sales/cash-drawer activity implies it should be.
//! Distinct from `core::auth::sessions::Session` (a login session).

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;

use crate::backend::core::errors::{AppError, AppResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PosSessionStatus {
    Open,
    Closed,
}

impl PosSessionStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            PosSessionStatus::Open => "open",
            PosSessionStatus::Closed => "closed",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "open" => Some(PosSessionStatus::Open),
            "closed" => Some(PosSessionStatus::Closed),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PosSession {
    pub id: Option<i64>,
    pub terminal_id: i64,
    pub cashier_user_id: i64,
    pub opened_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
    pub opening_cash: Decimal,
    /// The amount actually counted at close -- compared against the
    /// expected float (`cash_drawer::expected_closing_cash`) to find any
    /// over/short.
    pub closing_cash: Option<Decimal>,
    pub status: PosSessionStatus,
}

impl PosSession {
    pub fn open(terminal_id: i64, cashier_user_id: i64, opening_cash: Decimal) -> Self {
        Self {
            id: None,
            terminal_id,
            cashier_user_id,
            opened_at: Utc::now(),
            closed_at: None,
            opening_cash,
            closing_cash: None,
            status: PosSessionStatus::Open,
        }
    }

    pub fn validate(&self) -> AppResult<()> {
        if self.terminal_id <= 0 {
            return Err(AppError::Validation("POS session must reference a valid terminal".into()));
        }
        if self.cashier_user_id <= 0 {
            return Err(AppError::Validation("POS session must reference a valid cashier".into()));
        }
        if self.opening_cash < Decimal::ZERO {
            return Err(AppError::Validation("opening cash must not be negative".into()));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn open_starts_in_the_open_status_with_no_close_time() {
        let session = PosSession::open(1, 1, dec!(200));
        assert_eq!(session.status, PosSessionStatus::Open);
        assert!(session.closed_at.is_none());
    }

    #[test]
    fn validate_rejects_negative_opening_cash() {
        let session = PosSession::open(1, 1, dec!(-50));
        assert!(session.validate().is_err());
    }
}
