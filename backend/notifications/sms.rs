//! The SMS channel. Unlike email (SMTP is a universal, vendor-neutral
//! protocol), SMS delivery requires a specific paid gateway (Twilio and
//! similar) with its own API and account -- there's no vendor-neutral
//! protocol to implement generically. `UnconfiguredSmsSender` validates
//! and fails clearly, the same pattern as `email.rs`.

use crate::backend::core::errors::{AppError, AppResult};
use crate::backend::notifications::notifications::{
    Notification, NotificationChannel, NotificationSender,
};

/// Loose E.164-ish check: a leading `+` and 8-15 digits. Real carriers
/// vary; this catches obvious typos without pretending to fully validate
/// number-plan rules.
pub fn validate_phone_number(number: &str) -> AppResult<()> {
    let digits = number.strip_prefix('+').unwrap_or(number);
    let is_valid = number.starts_with('+')
        && digits.len() >= 8
        && digits.len() <= 15
        && digits.chars().all(|c| c.is_ascii_digit());
    if !is_valid {
        return Err(AppError::Validation(format!(
            "'{number}' is not a valid phone number (expected E.164, e.g. +14155552671)"
        )));
    }
    Ok(())
}

pub struct UnconfiguredSmsSender;

impl NotificationSender for UnconfiguredSmsSender {
    fn send(&self, notification: &Notification) -> AppResult<()> {
        if notification.channel != NotificationChannel::Sms {
            return Err(AppError::Internal(
                "UnconfiguredSmsSender received a non-SMS notification".into(),
            ));
        }
        validate_phone_number(&notification.recipient)?;
        Err(AppError::Config(
            "no SMS gateway is configured -- set one up in Settings before sending SMS".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_a_well_formed_e164_number() {
        assert!(validate_phone_number("+14155552671").is_ok());
    }

    #[test]
    fn rejects_a_number_without_a_leading_plus() {
        assert!(validate_phone_number("14155552671").is_err());
    }

    #[test]
    fn rejects_a_number_that_is_too_short() {
        assert!(validate_phone_number("+123").is_err());
    }

    #[test]
    fn unconfigured_sender_fails_clearly() {
        let sender = UnconfiguredSmsSender;
        let notification = Notification::new(
            1,
            NotificationChannel::Sms,
            "+14155552671",
            "Your invoice is due",
        );
        assert!(matches!(
            sender.send(&notification),
            Err(AppError::Config(_))
        ));
    }
}
