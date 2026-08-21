//! Recording that a fixed asset moved to a different branch/department --
//! a custody/location change, not a financial event, so (unlike
//! `inventory::transfers`) this has no GL or stock-ledger effect.

use chrono::NaiveDate;

use crate::backend::core::errors::{AppError, AppResult};

#[derive(Debug, Clone, PartialEq)]
pub struct AssetTransfer {
    pub id: Option<i64>,
    pub asset_id: i64,
    pub transfer_date: NaiveDate,
    pub from_branch_id: Option<i64>,
    pub to_branch_id: Option<i64>,
    pub notes: Option<String>,
}

impl AssetTransfer {
    pub fn new(asset_id: i64, transfer_date: NaiveDate, to_branch_id: i64) -> Self {
        Self {
            id: None,
            asset_id,
            transfer_date,
            from_branch_id: None,
            to_branch_id: Some(to_branch_id),
            notes: None,
        }
    }

    pub fn validate(&self) -> AppResult<()> {
        if self.asset_id <= 0 {
            return Err(AppError::Validation("asset transfer must reference a valid asset".into()));
        }
        if self.from_branch_id.is_some() && self.from_branch_id == self.to_branch_id {
            return Err(AppError::Validation("asset transfer source and destination branches must differ".into()));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_rejects_transferring_to_the_same_branch() {
        let mut transfer = AssetTransfer::new(1, NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(), 5);
        transfer.from_branch_id = Some(5);
        assert!(transfer.validate().is_err());
    }

    #[test]
    fn validate_accepts_a_well_formed_transfer() {
        let mut transfer = AssetTransfer::new(1, NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(), 5);
        transfer.from_branch_id = Some(3);
        assert!(transfer.validate().is_ok());
    }
}
