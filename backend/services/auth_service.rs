//! The auth domain's service layer: login and session concerns, for
//! `auth/login_view.rs` (Architecture Rules, Section 3). Not part of the
//! frozen `services/` file list in `PROJECT_SPEC.md` Section 2 -- that
//! list had no dedicated auth/user service, unlike every other domain.
//! Added here rather than folding login into `organization_service.rs`,
//! since authentication is its own concern, not organization data; see
//! `CHANGELOG.md` for the record of this deviation.
//!
//! Wraps `UserRepository` for lookup plus the free functions in
//! `core::auth::authentication` that decide the login outcome.

use crate::backend::core::auth::authentication::{authenticate, verify_two_factor, AuthOutcome};
use crate::backend::core::errors::AppResult;
use crate::backend::core::users::User;
use crate::backend::repositories::user_repository::UserRepository;
use crate::database::database::Database;

#[derive(Clone)]
pub struct AuthService {
    repo: UserRepository,
}

impl AuthService {
    pub fn new(db: Database) -> Self {
        Self { repo: UserRepository::new(db) }
    }

    /// Step one of login: username + password. Looks the user up and
    /// delegates the outcome to `authenticate` -- see its doc comment
    /// for why "no such user" and "wrong password" share one error.
    pub fn login_with_credentials(&self, username: &str, password: &str) -> AppResult<AuthOutcome> {
        let user = self.repo.find_user_by_username(username)?;
        authenticate(user.as_ref(), password)
    }

    /// Step two of login, only reached when `login_with_credentials`
    /// returned `TwoFactorRequired`.
    pub fn login_with_two_factor_code(&self, user: &User, code: &str) -> AppResult<User> {
        verify_two_factor(user, code)
    }
}
