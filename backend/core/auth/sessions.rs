//! A logged-in session: an opaque token tied to a user, with an
//! expiry and an optional revocation, so "who's logged in" is a durable,
//! auditable fact rather than just in-memory state.

use chrono::{DateTime, Duration, Utc};
use rand::RngCore;

pub const SESSION_DURATION_HOURS: i64 = 12;

#[derive(Debug, Clone, PartialEq)]
pub struct Session {
    pub id: Option<i64>,
    pub user_id: i64,
    pub token: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
}

impl Session {
    pub fn new(user_id: i64) -> Self {
        let created_at = Utc::now();
        Self {
            id: None,
            user_id,
            token: generate_token(),
            created_at,
            expires_at: created_at + Duration::hours(SESSION_DURATION_HOURS),
            revoked_at: None,
        }
    }

    pub fn is_valid_at(&self, at: DateTime<Utc>) -> bool {
        self.revoked_at.is_none() && at < self.expires_at
    }

    pub fn is_valid(&self) -> bool {
        self.is_valid_at(Utc::now())
    }
}

fn generate_token() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_session_is_valid_immediately_and_carries_a_unique_token() {
        let a = Session::new(1);
        let b = Session::new(1);
        assert!(a.is_valid());
        assert_ne!(a.token, b.token);
    }

    #[test]
    fn session_is_invalid_after_its_expiry() {
        let session = Session::new(1);
        let after_expiry = session.expires_at + Duration::seconds(1);
        assert!(!session.is_valid_at(after_expiry));
    }

    #[test]
    fn revoked_session_is_invalid_even_before_expiry() {
        let mut session = Session::new(1);
        session.revoked_at = Some(Utc::now());
        assert!(!session.is_valid());
    }
}
