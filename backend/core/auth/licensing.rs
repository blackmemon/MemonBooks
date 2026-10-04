//! What a license grants: who it's for, and whether (and when) it
//! expires. `activation.rs` turns a license key string into one of
//! these; this module just shapes the result and the trial fallback
//! when there's no key at all.

use chrono::NaiveDate;

#[derive(Debug, Clone, PartialEq)]
pub struct LicenseInfo {
    pub license_id: String,
    pub licensed_to: String,
    pub issued_at: NaiveDate,
    /// `None` means perpetual (no expiry).
    pub expires_at: Option<NaiveDate>,
}

impl LicenseInfo {
    pub fn is_expired(&self, as_of: NaiveDate) -> bool {
        self.expires_at.is_some_and(|expiry| as_of > expiry)
    }
}

pub const TRIAL_PERIOD_DAYS: i64 = 30;

#[derive(Debug, Clone, PartialEq)]
pub enum LicenseStatus {
    /// No license key entered yet; running on the trial clock.
    Trial {
        days_remaining: i64,
    },
    TrialExpired,
    Active(LicenseInfo),
    Expired(LicenseInfo),
}

impl LicenseStatus {
    pub fn allows_use(&self) -> bool {
        match self {
            LicenseStatus::Trial { days_remaining } => *days_remaining > 0,
            LicenseStatus::TrialExpired | LicenseStatus::Expired(_) => false,
            LicenseStatus::Active(_) => true,
        }
    }
}

/// The trial-clock status when no license key has been entered:
/// `first_launch` is when the app was first run on this machine.
pub fn trial_status(first_launch: NaiveDate, as_of: NaiveDate) -> LicenseStatus {
    let days_elapsed = (as_of - first_launch).num_days();
    let days_remaining = TRIAL_PERIOD_DAYS - days_elapsed;
    if days_remaining > 0 {
        LicenseStatus::Trial { days_remaining }
    } else {
        LicenseStatus::TrialExpired
    }
}

/// The status for a successfully verified license key.
pub fn licensed_status(license: LicenseInfo, as_of: NaiveDate) -> LicenseStatus {
    if license.is_expired(as_of) {
        LicenseStatus::Expired(license)
    } else {
        LicenseStatus::Active(license)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    #[test]
    fn trial_counts_down_from_first_launch() {
        let status = trial_status(date(2026, 1, 1), date(2026, 1, 10));
        assert_eq!(status, LicenseStatus::Trial { days_remaining: 21 });
        assert!(status.allows_use());
    }

    #[test]
    fn trial_expires_after_30_days() {
        let status = trial_status(date(2026, 1, 1), date(2026, 2, 1));
        assert_eq!(status, LicenseStatus::TrialExpired);
        assert!(!status.allows_use());
    }

    #[test]
    fn perpetual_license_never_expires() {
        let license = LicenseInfo {
            license_id: "L-1".into(),
            licensed_to: "Acme Trading Co".into(),
            issued_at: date(2026, 1, 1),
            expires_at: None,
        };
        assert!(!license.is_expired(date(2099, 1, 1)));
        assert!(matches!(
            licensed_status(license, date(2099, 1, 1)),
            LicenseStatus::Active(_)
        ));
    }

    #[test]
    fn a_license_past_its_expiry_date_is_expired() {
        let license = LicenseInfo {
            license_id: "L-1".into(),
            licensed_to: "Acme Trading Co".into(),
            issued_at: date(2026, 1, 1),
            expires_at: Some(date(2026, 12, 31)),
        };
        let status = licensed_status(license, date(2027, 1, 1));
        assert!(matches!(status, LicenseStatus::Expired(_)));
        assert!(!status.allows_use());
    }
}
