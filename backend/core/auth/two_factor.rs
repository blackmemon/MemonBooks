//! TOTP-based two-factor authentication (RFC 6238), via `totp-rs`.
//! `two_factor_secret` (base32) lives on `User`; this module only turns a
//! secret into a `TOTP` instance and checks codes against it.

use totp_rs::{Algorithm, Secret, TOTP};

use crate::backend::core::errors::{AppError, AppResult};

const DIGITS: usize = 6;
const STEP_SECONDS: u64 = 30;
/// How many steps of clock drift either side of "now" still count as valid.
const SKEW_STEPS: u8 = 1;

/// Generates a new random base32-encoded secret for a user enabling 2FA.
pub fn generate_secret() -> String {
    Secret::generate_secret().to_encoded().to_string()
}

fn totp_for(secret_base32: &str) -> AppResult<TOTP> {
    let secret_bytes = Secret::Encoded(secret_base32.to_string())
        .to_bytes()
        .map_err(|_| AppError::Validation("two-factor secret is not valid base32".into()))?;

    TOTP::new(Algorithm::SHA1, DIGITS, SKEW_STEPS, STEP_SECONDS, secret_bytes)
        .map_err(|err| AppError::Internal(format!("failed to build TOTP generator: {err}")))
}

/// Checks a 6-digit code the user typed against their stored secret.
pub fn verify_code(secret_base32: &str, code: &str) -> AppResult<bool> {
    let totp = totp_for(secret_base32)?;
    Ok(totp.check_current(code).unwrap_or(false))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_freshly_generated_code_verifies_against_its_own_secret() {
        let secret = generate_secret();
        let totp = totp_for(&secret).unwrap();
        let code = totp.generate_current().unwrap();
        assert!(verify_code(&secret, &code).unwrap());
    }

    #[test]
    fn wrong_code_does_not_verify() {
        let secret = generate_secret();
        assert!(!verify_code(&secret, "000000").unwrap());
    }

    #[test]
    fn malformed_secret_is_a_validation_error() {
        let result = verify_code("not-valid-base32!!", "123456");
        assert!(matches!(result, Err(AppError::Validation(_))));
    }
}
