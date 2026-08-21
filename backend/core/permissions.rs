//! Permission keys are plain `"<area>.<action>"` strings kept in one open
//! catalog, rather than a closed Rust enum, because every business domain
//! added in a later Development Order step will register a few more of
//! them here without needing to touch the roles/users machinery itself.

/// Permission keys defined so far. Extend this as each domain module is
/// built; `is_known_permission` and role validation both key off it, so a
/// role can never be granted a permission that doesn't exist yet.
pub const KNOWN_PERMISSIONS: &[&str] = &[
    "users.manage",
    "roles.manage",
    "organization.manage",
    "reports.view",
];

pub fn is_known_permission(key: &str) -> bool {
    KNOWN_PERMISSIONS.contains(&key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_catalog_entries() {
        assert!(is_known_permission("users.manage"));
    }

    #[test]
    fn rejects_unknown_keys() {
        assert!(!is_known_permission("nonexistent.permission"));
    }
}
