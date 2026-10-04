//! Fiscal periods gate when postings are allowed. Per Architecture Rules
//! (Section 3): "Fiscal periods support open and closed/locked states
//! with permission checks." The state machine itself lives here; *who* is
//! allowed to trigger a transition is a service-layer concern (checked
//! against `core::permissions`) once the service/view layer calls into it.

use chrono::NaiveDate;

use crate::backend::core::errors::{AppError, AppResult};
use crate::backend::core::validation::{require_non_empty, Validate};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FiscalPeriodStatus {
    /// Normal postings are allowed.
    Open,
    /// No new postings; can still be reopened by someone with permission.
    Closed,
    /// Permanently closed (e.g. after an audit) -- cannot be reopened.
    Locked,
}

impl FiscalPeriodStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            FiscalPeriodStatus::Open => "open",
            FiscalPeriodStatus::Closed => "closed",
            FiscalPeriodStatus::Locked => "locked",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "open" => Some(FiscalPeriodStatus::Open),
            "closed" => Some(FiscalPeriodStatus::Closed),
            "locked" => Some(FiscalPeriodStatus::Locked),
            _ => None,
        }
    }

    /// Whether moving from `self` to `to` is a legal transition:
    /// Open <-> Closed, and Closed -> Locked. Locked is terminal, and
    /// no-op "transitions" to the same state aren't transitions at all.
    pub fn can_transition_to(self, to: FiscalPeriodStatus) -> bool {
        use FiscalPeriodStatus::*;
        matches!(
            (self, to),
            (Open, Closed) | (Closed, Open) | (Closed, Locked)
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FiscalPeriod {
    pub id: Option<i64>,
    pub company_id: i64,
    pub name: String,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub status: FiscalPeriodStatus,
}

impl FiscalPeriod {
    pub fn new(
        company_id: i64,
        name: impl Into<String>,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Self {
        Self {
            id: None,
            company_id,
            name: name.into(),
            start_date,
            end_date,
            status: FiscalPeriodStatus::Open,
        }
    }

    pub fn contains(&self, date: NaiveDate) -> bool {
        date >= self.start_date && date <= self.end_date
    }

    pub fn accepts_postings(&self) -> bool {
        matches!(self.status, FiscalPeriodStatus::Open)
    }
}

impl Validate for FiscalPeriod {
    fn validate(&self) -> AppResult<()> {
        if self.company_id <= 0 {
            return Err(AppError::Validation(
                "fiscal period must reference a valid company".into(),
            ));
        }
        require_non_empty("fiscal period name", &self.name)?;
        if self.start_date > self.end_date {
            return Err(AppError::Validation(
                "fiscal period start_date must not be after end_date".into(),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    fn january_2026(company_id: i64) -> FiscalPeriod {
        FiscalPeriod::new(
            company_id,
            "January 2026",
            date(2026, 1, 1),
            date(2026, 1, 31),
        )
    }

    #[test]
    fn validate_rejects_start_after_end() {
        let period = FiscalPeriod::new(1, "Backwards", date(2026, 2, 1), date(2026, 1, 1));
        assert!(period.validate().is_err());
    }

    #[test]
    fn contains_checks_the_inclusive_date_range() {
        let period = january_2026(1);
        assert!(period.contains(date(2026, 1, 1)));
        assert!(period.contains(date(2026, 1, 31)));
        assert!(!period.contains(date(2026, 2, 1)));
    }

    #[test]
    fn only_open_periods_accept_postings() {
        let mut period = january_2026(1);
        assert!(period.accepts_postings());
        period.status = FiscalPeriodStatus::Closed;
        assert!(!period.accepts_postings());
        period.status = FiscalPeriodStatus::Locked;
        assert!(!period.accepts_postings());
    }

    #[test]
    fn open_and_closed_transition_into_each_other() {
        assert!(FiscalPeriodStatus::Open.can_transition_to(FiscalPeriodStatus::Closed));
        assert!(FiscalPeriodStatus::Closed.can_transition_to(FiscalPeriodStatus::Open));
    }

    #[test]
    fn closed_can_be_locked_but_locked_is_terminal() {
        assert!(FiscalPeriodStatus::Closed.can_transition_to(FiscalPeriodStatus::Locked));
        assert!(!FiscalPeriodStatus::Locked.can_transition_to(FiscalPeriodStatus::Open));
        assert!(!FiscalPeriodStatus::Locked.can_transition_to(FiscalPeriodStatus::Closed));
    }

    #[test]
    fn open_cannot_jump_directly_to_locked() {
        assert!(!FiscalPeriodStatus::Open.can_transition_to(FiscalPeriodStatus::Locked));
    }

    #[test]
    fn as_str_and_parse_round_trip_for_every_variant() {
        for status in [
            FiscalPeriodStatus::Open,
            FiscalPeriodStatus::Closed,
            FiscalPeriodStatus::Locked,
        ] {
            assert_eq!(FiscalPeriodStatus::parse(status.as_str()), Some(status));
        }
    }
}
