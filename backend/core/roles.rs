//! A role is a named bundle of permissions assigned to users.

use crate::backend::core::errors::{AppError, AppResult};
use crate::backend::core::permissions::is_known_permission;
use crate::backend::core::validation::{require_max_length, require_non_empty, Validate};

#[derive(Debug, Clone, PartialEq)]
pub struct Role {
    pub id: Option<i64>,
    pub name: String,
    pub description: Option<String>,
    pub permissions: Vec<String>,
}

impl Role {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: None,
            name: name.into(),
            description: None,
            permissions: Vec::new(),
        }
    }

    pub fn has_permission(&self, permission: &str) -> bool {
        self.permissions.iter().any(|p| p == permission)
    }
}

impl Validate for Role {
    fn validate(&self) -> AppResult<()> {
        require_non_empty("role name", &self.name)?;
        require_max_length("role name", &self.name, 100)?;
        if let Some(description) = &self.description {
            require_max_length("role description", description, 500)?;
        }
        for permission in &self.permissions {
            if !is_known_permission(permission) {
                return Err(AppError::Validation(format!("'{permission}' is not a known permission")));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_rejects_unknown_permission() {
        let mut role = Role::new("Administrator");
        role.permissions.push("not.a.real.permission".into());
        assert!(role.validate().is_err());
    }

    #[test]
    fn validate_accepts_known_permissions() {
        let mut role = Role::new("Administrator");
        role.permissions.push("users.manage".into());
        assert!(role.validate().is_ok());
    }

    #[test]
    fn has_permission_checks_membership() {
        let mut role = Role::new("Administrator");
        role.permissions.push("users.manage".into());
        assert!(role.has_permission("users.manage"));
        assert!(!role.has_permission("roles.manage"));
    }
}
