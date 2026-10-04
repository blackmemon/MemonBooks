//! License key generation and verification, HMAC-SHA256 signed.
//!
//! This is symmetric (the same secret signs and verifies), which is the
//! inherent limit of any offline-verifiable desktop license scheme: the
//! secret has to ship inside the binary for verification to work without
//! a license server, so someone willing to extract it from the compiled
//! app could forge keys. That's a known, accepted trade-off for offline
//! activation (the alternative -- asymmetric signing or a license
//! server -- is a real architectural decision for whoever deploys this,
//! not something to bake in silently). `sign_license_key` is a
//! vendor-side tool (used to issue keys), not something the running app
//! calls; `verify_license_key` is what the app actually uses.

use chrono::NaiveDate;
use hmac::{Hmac, Mac};
use sha2::Sha256;

use crate::backend::core::auth::licensing::LicenseInfo;
use crate::backend::core::errors::{AppError, AppResult};

type HmacSha256 = Hmac<Sha256>;

fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn from_hex(hex: &str) -> Option<Vec<u8>> {
    if !hex.len().is_multiple_of(2) {
        return None;
    }
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).ok())
        .collect()
}

fn payload_string(
    license_id: &str,
    licensed_to: &str,
    issued_at: NaiveDate,
    expires_at: Option<NaiveDate>,
) -> String {
    format!(
        "{license_id}|{licensed_to}|{issued_at}|{}",
        expires_at.map(|d| d.to_string()).unwrap_or_default()
    )
}

/// Builds a signed key: `hex(payload_bytes).hex(signature)`. A vendor-side
/// tool, not called by the running app in normal operation.
pub fn sign_license_key(
    secret: &[u8],
    license_id: &str,
    licensed_to: &str,
    issued_at: NaiveDate,
    expires_at: Option<NaiveDate>,
) -> String {
    let payload = payload_string(license_id, licensed_to, issued_at, expires_at);

    let mut mac = HmacSha256::new_from_slice(secret).expect("HMAC accepts a key of any length");
    mac.update(payload.as_bytes());
    let signature = mac.finalize().into_bytes();

    format!("{}.{}", to_hex(payload.as_bytes()), to_hex(&signature))
}

/// Verifies a key against `secret` and, if valid, returns what it grants.
pub fn verify_license_key(secret: &[u8], key: &str) -> AppResult<LicenseInfo> {
    let (payload_hex, signature_hex) = key
        .split_once('.')
        .ok_or_else(|| AppError::Validation("license key is malformed".into()))?;

    let payload_bytes = from_hex(payload_hex)
        .ok_or_else(|| AppError::Validation("license key is malformed".into()))?;
    let signature_bytes = from_hex(signature_hex)
        .ok_or_else(|| AppError::Validation("license key is malformed".into()))?;
    let payload = String::from_utf8(payload_bytes)
        .map_err(|_| AppError::Validation("license key is malformed".into()))?;

    let mut mac = HmacSha256::new_from_slice(secret).expect("HMAC accepts a key of any length");
    mac.update(payload.as_bytes());
    mac.verify_slice(&signature_bytes)
        .map_err(|_| AppError::Validation("license key signature is invalid".into()))?;

    let mut parts = payload.splitn(4, '|');
    let license_id = parts.next().unwrap_or_default().to_string();
    let licensed_to = parts.next().unwrap_or_default().to_string();
    let issued_at: NaiveDate = parts
        .next()
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| AppError::Validation("license key payload is malformed".into()))?;
    let expires_at = parts
        .next()
        .filter(|s| !s.is_empty())
        .and_then(|s| s.parse().ok());

    Ok(LicenseInfo {
        license_id,
        licensed_to,
        issued_at,
        expires_at,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    #[test]
    fn a_key_signed_with_the_right_secret_verifies() {
        let key = sign_license_key(
            b"vendor-secret",
            "L-1",
            "Acme Trading Co",
            date(2026, 1, 1),
            None,
        );
        let license = verify_license_key(b"vendor-secret", &key).unwrap();
        assert_eq!(license.license_id, "L-1");
        assert_eq!(license.licensed_to, "Acme Trading Co");
        assert_eq!(license.expires_at, None);
    }

    #[test]
    fn a_key_with_an_expiry_round_trips() {
        let key = sign_license_key(
            b"vendor-secret",
            "L-2",
            "Acme Trading Co",
            date(2026, 1, 1),
            Some(date(2027, 1, 1)),
        );
        let license = verify_license_key(b"vendor-secret", &key).unwrap();
        assert_eq!(license.expires_at, Some(date(2027, 1, 1)));
    }

    #[test]
    fn a_key_signed_with_the_wrong_secret_is_rejected() {
        let key = sign_license_key(
            b"vendor-secret",
            "L-1",
            "Acme Trading Co",
            date(2026, 1, 1),
            None,
        );
        assert!(verify_license_key(b"wrong-secret", &key).is_err());
    }

    #[test]
    fn a_tampered_key_is_rejected() {
        let mut key = sign_license_key(
            b"vendor-secret",
            "L-1",
            "Acme Trading Co",
            date(2026, 1, 1),
            None,
        );
        // Flip the licensed_to name inside the (hex-encoded) payload by
        // corrupting a character -- the signature no longer matches.
        key.replace_range(10..12, "ff");
        assert!(verify_license_key(b"vendor-secret", &key).is_err());
    }

    #[test]
    fn malformed_keys_are_rejected_not_panicked_on() {
        assert!(verify_license_key(b"vendor-secret", "not-a-real-key").is_err());
        assert!(verify_license_key(b"vendor-secret", "").is_err());
        assert!(verify_license_key(b"vendor-secret", "zz.zz").is_err());
    }
}
