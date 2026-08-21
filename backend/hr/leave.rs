//! Leave requests.

use chrono::NaiveDate;

use crate::backend::core::errors::{AppError, AppResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LeaveType {
    Annual,
    Sick,
    Unpaid,
    Other,
}

impl LeaveType {
    pub fn as_str(self) -> &'static str {
        match self {
            LeaveType::Annual => "annual",
            LeaveType::Sick => "sick",
            LeaveType::Unpaid => "unpaid",
            LeaveType::Other => "other",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "annual" => Some(LeaveType::Annual),
            "sick" => Some(LeaveType::Sick),
            "unpaid" => Some(LeaveType::Unpaid),
            "other" => Some(LeaveType::Other),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LeaveStatus {
    Requested,
    Approved,
    Rejected,
}

impl LeaveStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            LeaveStatus::Requested => "requested",
            LeaveStatus::Approved => "approved",
            LeaveStatus::Rejected => "rejected",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "requested" => Some(LeaveStatus::Requested),
            "approved" => Some(LeaveStatus::Approved),
            "rejected" => Some(LeaveStatus::Rejected),
            _ => None,
        }
    }

    pub fn can_transition_to(self, to: LeaveStatus) -> bool {
        matches!((self, to), (LeaveStatus::Requested, LeaveStatus::Approved) | (LeaveStatus::Requested, LeaveStatus::Rejected))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LeaveRequest {
    pub id: Option<i64>,
    pub employee_id: i64,
    pub leave_type: LeaveType,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub status: LeaveStatus,
}

impl LeaveRequest {
    pub fn new(employee_id: i64, leave_type: LeaveType, start_date: NaiveDate, end_date: NaiveDate) -> Self {
        Self {
            id: None,
            employee_id,
            leave_type,
            start_date,
            end_date,
            status: LeaveStatus::Requested,
        }
    }

    /// Inclusive day count.
    pub fn days(&self) -> i64 {
        (self.end_date - self.start_date).num_days() + 1
    }

    pub fn validate(&self) -> AppResult<()> {
        if self.employee_id <= 0 {
            return Err(AppError::Validation("leave request must reference a valid employee".into()));
        }
        if self.end_date < self.start_date {
            return Err(AppError::Validation("leave request end_date must not be before start_date".into()));
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

    #[test]
    fn days_counts_inclusively() {
        let request = LeaveRequest::new(1, LeaveType::Annual, date(2026, 1, 5), date(2026, 1, 9));
        assert_eq!(request.days(), 5);
    }

    #[test]
    fn validate_rejects_end_before_start() {
        let request = LeaveRequest::new(1, LeaveType::Annual, date(2026, 1, 9), date(2026, 1, 5));
        assert!(request.validate().is_err());
    }

    #[test]
    fn requested_can_move_to_approved_or_rejected_only() {
        assert!(LeaveStatus::Requested.can_transition_to(LeaveStatus::Approved));
        assert!(LeaveStatus::Requested.can_transition_to(LeaveStatus::Rejected));
        assert!(!LeaveStatus::Approved.can_transition_to(LeaveStatus::Rejected));
    }
}
