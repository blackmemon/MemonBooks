//! When the next backup is due. Pure calculation -- actually triggering
//! a backup at startup (or on a timer while the app runs) is the app
//! loop's job (Development Order step 19), not this module's.

use chrono::{DateTime, Duration, Utc};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BackupFrequency {
    Daily,
    Weekly,
    Monthly,
}

impl BackupFrequency {
    fn interval(self) -> Duration {
        match self {
            BackupFrequency::Daily => Duration::days(1),
            BackupFrequency::Weekly => Duration::days(7),
            BackupFrequency::Monthly => Duration::days(30),
        }
    }
}

/// `None` (never backed up) is always due immediately.
pub fn next_backup_due(last_backup: Option<DateTime<Utc>>, frequency: BackupFrequency) -> DateTime<Utc> {
    match last_backup {
        Some(last) => last + frequency.interval(),
        None => Utc::now(),
    }
}

pub fn is_backup_due(last_backup: Option<DateTime<Utc>>, frequency: BackupFrequency, now: DateTime<Utc>) -> bool {
    match last_backup {
        None => true,
        Some(_) => now >= next_backup_due(last_backup, frequency),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn never_backed_up_is_always_due() {
        assert!(is_backup_due(None, BackupFrequency::Daily, Utc::now()));
    }

    #[test]
    fn daily_backup_is_not_due_before_a_day_has_passed() {
        let last = Utc::now();
        assert!(!is_backup_due(Some(last), BackupFrequency::Daily, last + Duration::hours(12)));
        assert!(is_backup_due(Some(last), BackupFrequency::Daily, last + Duration::hours(25)));
    }

    #[test]
    fn weekly_and_monthly_use_their_own_intervals() {
        let last = Utc::now();
        assert!(!is_backup_due(Some(last), BackupFrequency::Weekly, last + Duration::days(3)));
        assert!(is_backup_due(Some(last), BackupFrequency::Weekly, last + Duration::days(8)));
        assert!(!is_backup_due(Some(last), BackupFrequency::Monthly, last + Duration::days(15)));
        assert!(is_backup_due(Some(last), BackupFrequency::Monthly, last + Duration::days(31)));
    }
}
