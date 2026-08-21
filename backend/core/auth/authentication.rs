//! The login flow: verify credentials, then -- if the account has 2FA
//! enabled -- require a valid TOTP code before a session is issued.
//! Deliberately returns the same generic error for "no such user" and
//! "wrong password" so login failures don't reveal which usernames exist.

use crate::backend::core::auth::passwords::verify_password;
use crate::backend::core::auth::two_factor::verify_code;
use crate::backend::core::errors::{AppError, AppResult};
use crate::backend::core::users::User;

const INVALID_CREDENTIALS: &str = "invalid username or password";

#[derive(Debug, Clone, PartialEq)]
pub enum AuthOutcome {
    Authenticated(User),
    TwoFactorRequired(User),
}

/// Step one of login: username + password. If the account has 2FA
/// enabled, this succeeds with `TwoFactorRequired` rather than granting
/// access -- the caller must then call `verify_two_factor`.
pub fn authenticate(user: Option<&User>, password: &str) -> AppResult<AuthOutcome> {
    let user = user.filter(|u| u.is_active).ok_or_else(|| AppError::Validation(INVALID_CREDENTIALS.into()))?;

    if !verify_password(password, &user.password_hash)? {
        return Err(AppError::Validation(INVALID_CREDENTIALS.into()));
    }

    if user.two_factor_enabled {
        Ok(AuthOutcome::TwoFactorRequired(user.clone()))
    } else {
        Ok(AuthOutcome::Authenticated(user.clone()))
    }
}

/// Step two of login, only reached when `authenticate` returned
/// `TwoFactorRequired`.
pub fn verify_two_factor(user: &User, code: &str) -> AppResult<User> {
    let secret = user
        .two_factor_secret
        .as_deref()
        .ok_or_else(|| AppError::Internal("two-factor enabled without a stored secret".into()))?;

    if verify_code(secret, code)? {
        Ok(user.clone())
    } else {
        Err(AppError::Validation("invalid two-factor code".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::core::auth::passwords::hash_password;
    use crate::backend::core::auth::two_factor::generate_secret;

    fn user_with_password(password: &str) -> User {
        User::new("alice", "Alice Smith", hash_password(password).unwrap(), 1)
    }

    #[test]
    fn rejects_missing_user_with_generic_error() {
        let result = authenticate(None, "whatever password");
        assert!(matches!(result, Err(AppError::Validation(msg)) if msg == INVALID_CREDENTIALS));
    }

    #[test]
    fn rejects_inactive_user_with_generic_error() {
        let mut user = user_with_password("correct horse battery staple");
        user.is_active = false;
        let result = authenticate(Some(&user), "correct horse battery staple");
        assert!(matches!(result, Err(AppError::Validation(msg)) if msg == INVALID_CREDENTIALS));
    }

    #[test]
    fn rejects_wrong_password_with_generic_error() {
        let user = user_with_password("correct horse battery staple");
        let result = authenticate(Some(&user), "wrong password entirely");
        assert!(matches!(result, Err(AppError::Validation(msg)) if msg == INVALID_CREDENTIALS));
    }

    #[test]
    fn succeeds_directly_when_two_factor_is_disabled() {
        let user = user_with_password("correct horse battery staple");
        let outcome = authenticate(Some(&user), "correct horse battery staple").unwrap();
        assert!(matches!(outcome, AuthOutcome::Authenticated(_)));
    }

    #[test]
    fn requires_two_factor_when_enabled_then_succeeds_with_a_valid_code() {
        let mut user = user_with_password("correct horse battery staple");
        let secret = generate_secret();
        user.two_factor_enabled = true;
        user.two_factor_secret = Some(secret.clone());

        let outcome = authenticate(Some(&user), "correct horse battery staple").unwrap();
        let pending_user = match outcome {
            AuthOutcome::TwoFactorRequired(u) => u,
            AuthOutcome::Authenticated(_) => panic!("expected two-factor to be required"),
        };

        let code = totp_rs::TOTP::new(
            totp_rs::Algorithm::SHA1,
            6,
            1,
            30,
            totp_rs::Secret::Encoded(secret).to_bytes().unwrap(),
        )
        .unwrap()
        .generate_current()
        .unwrap();

        let authenticated = verify_two_factor(&pending_user, &code).unwrap();
        assert_eq!(authenticated.username, "alice");
    }
}
