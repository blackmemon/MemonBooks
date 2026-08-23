//! A user account. Passwords are never stored or handled here as
//! plaintext -- `password_hash` is whatever `core::auth::passwords`
//! produced; this module only shapes and validates the record around it.

use crate::backend::core::errors::{AppError, AppResult};
use crate::backend::core::validation::{require_max_length, require_non_empty, Validate};

#[derive(Debug, Clone, PartialEq)]
pub struct User {
    pub id: Option<i64>,
    pub username: String,
    pub display_name: String,
    pub password_hash: String,
    pub role_id: i64,
    pub is_active: bool,
    pub two_factor_enabled: bool,
    pub two_factor_secret: Option<String>,
    /// Forces a password change before this account can use anything
    /// past the login screen -- set on the bootstrap-generated admin
    /// account so a freshly-installed system's freshly-logged
    /// credential doesn't stay live indefinitely.
    pub must_change_password: bool,
}

impl User {
    pub fn new(username: impl Into<String>, display_name: impl Into<String>, password_hash: impl Into<String>, role_id: i64) -> Self {
        Self {
            id: None,
            username: username.into().to_ascii_lowercase(),
            display_name: display_name.into(),
            password_hash: password_hash.into(),
            role_id,
            is_active: true,
            two_factor_enabled: false,
            two_factor_secret: None,
            must_change_password: false,
        }
    }
}

impl Validate for User {
    fn validate(&self) -> AppResult<()> {
        require_non_empty("username", &self.username)?;
        require_max_length("username", &self.username, 100)?;
        if self.username.chars().any(char::is_whitespace) {
            return Err(AppError::Validation("username must not contain whitespace".into()));
        }
        require_non_empty("display name", &self.display_name)?;
        require_max_length("display name", &self.display_name, 200)?;
        require_non_empty("password hash", &self.password_hash)?;
        if self.role_id <= 0 {
            return Err(AppError::Validation("user must reference a valid role".into()));
        }
        if self.two_factor_enabled && self.two_factor_secret.is_none() {
            return Err(AppError::Validation(
                "two-factor authentication cannot be enabled without a secret".into(),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_normalizes_username_to_lowercase() {
        let user = User::new("Alice", "Alice Smith", "hash", 1);
        assert_eq!(user.username, "alice");
    }

    #[test]
    fn validate_rejects_username_with_whitespace() {
        let user = User::new("alice smith", "Alice Smith", "hash", 1);
        assert!(user.validate().is_err());
    }

    #[test]
    fn validate_rejects_non_positive_role_id() {
        let user = User::new("alice", "Alice Smith", "hash", 0);
        assert!(user.validate().is_err());
    }

    #[test]
    fn validate_rejects_two_factor_enabled_without_secret() {
        let mut user = User::new("alice", "Alice Smith", "hash", 1);
        user.two_factor_enabled = true;
        assert!(user.validate().is_err());
    }

    #[test]
    fn validate_accepts_well_formed_user() {
        let user = User::new("alice", "Alice Smith", "hash", 1);
        assert!(user.validate().is_ok());
    }
}
