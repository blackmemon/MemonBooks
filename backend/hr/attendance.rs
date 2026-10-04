//! Daily attendance records.

use chrono::NaiveDate;
use rust_decimal::Decimal;

use crate::backend::core::errors::{AppError, AppResult};
use crate::backend::core::validation::require_non_negative;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AttendanceStatus {
    Present,
    Absent,
    OnLeave,
    Holiday,
}

impl AttendanceStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            AttendanceStatus::Present => "present",
            AttendanceStatus::Absent => "absent",
            AttendanceStatus::OnLeave => "on_leave",
            AttendanceStatus::Holiday => "holiday",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "present" => Some(AttendanceStatus::Present),
            "absent" => Some(AttendanceStatus::Absent),
            "on_leave" => Some(AttendanceStatus::OnLeave),
            "holiday" => Some(AttendanceStatus::Holiday),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AttendanceRecord {
    pub id: Option<i64>,
    pub employee_id: i64,
    pub date: NaiveDate,
    pub status: AttendanceStatus,
    pub hours_worked: Option<Decimal>,
}

impl AttendanceRecord {
    pub fn new(employee_id: i64, date: NaiveDate, status: AttendanceStatus) -> Self {
        Self {
            id: None,
            employee_id,
            date,
            status,
            hours_worked: None,
        }
    }

    pub fn validate(&self) -> AppResult<()> {
        if self.employee_id <= 0 {
            return Err(AppError::Validation(
                "attendance record must reference a valid employee".into(),
            ));
        }
        if let Some(hours) = self.hours_worked {
            require_non_negative("hours worked", hours)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn as_str_and_parse_round_trip_for_every_status() {
        for status in [
            AttendanceStatus::Present,
            AttendanceStatus::Absent,
            AttendanceStatus::OnLeave,
            AttendanceStatus::Holiday,
        ] {
            assert_eq!(AttendanceStatus::parse(status.as_str()), Some(status));
        }
    }

    #[test]
    fn validate_rejects_negative_hours_worked() {
        let mut record = AttendanceRecord::new(
            1,
            NaiveDate::from_ymd_opt(2026, 1, 5).unwrap(),
            AttendanceStatus::Present,
        );
        record.hours_worked = Some(rust_decimal_macros::dec!(-1));
        assert!(record.validate().is_err());
    }
}
