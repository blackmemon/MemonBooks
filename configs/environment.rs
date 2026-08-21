//! Which runtime environment the app is executing in. Controls log
//! verbosity and (later) which safeguards are relaxed for testing.

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Environment {
    Development,
    Test,
    Production,
}

impl Environment {
    /// Reads `MEMONBOOKS_ENV` from the process environment.
    pub fn current() -> Self {
        Self::from_env_value(std::env::var("MEMONBOOKS_ENV").ok().as_deref())
    }

    /// Pure resolution logic, kept separate from `std::env::var` so it can
    /// be unit tested without mutating process-global state.
    fn from_env_value(value: Option<&str>) -> Self {
        match value.map(str::to_ascii_lowercase).as_deref() {
            Some("production") | Some("prod") => Environment::Production,
            Some("test") => Environment::Test,
            Some("development") | Some("dev") => Environment::Development,
            _ if cfg!(debug_assertions) => Environment::Development,
            _ => Environment::Production,
        }
    }

    pub fn is_production(self) -> bool {
        matches!(self, Environment::Production)
    }
}

impl fmt::Display for Environment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Environment::Development => "development",
            Environment::Test => "test",
            Environment::Production => "production",
        };
        f.write_str(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_explicit_values() {
        assert_eq!(Environment::from_env_value(Some("production")), Environment::Production);
        assert_eq!(Environment::from_env_value(Some("Prod")), Environment::Production);
        assert_eq!(Environment::from_env_value(Some("TEST")), Environment::Test);
        assert_eq!(Environment::from_env_value(Some("development")), Environment::Development);
        assert_eq!(Environment::from_env_value(Some("dev")), Environment::Development);
    }

    #[test]
    fn falls_back_by_build_profile_when_unset() {
        let resolved = Environment::from_env_value(None);
        if cfg!(debug_assertions) {
            assert_eq!(resolved, Environment::Development);
        } else {
            assert_eq!(resolved, Environment::Production);
        }
    }

    #[test]
    fn falls_back_by_build_profile_when_unrecognized() {
        let resolved = Environment::from_env_value(Some("not-a-real-environment"));
        if cfg!(debug_assertions) {
            assert_eq!(resolved, Environment::Development);
        } else {
            assert_eq!(resolved, Environment::Production);
        }
    }

    #[test]
    fn is_production_only_true_for_production() {
        assert!(Environment::Production.is_production());
        assert!(!Environment::Development.is_production());
        assert!(!Environment::Test.is_production());
    }
}
