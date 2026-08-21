//! The email channel. Actual delivery needs a configured SMTP server
//! (host, port, credentials) that only exists once the user sets one up
//! -- there's no way to verify a live send without one, so
//! `UnconfiguredEmailSender` is the honest default: it validates the
//! notification fully and then fails clearly, rather than pretending to
//! send. Wiring a real SMTP client in here (e.g. via `lettre`) is a
//! drop-in replacement once a server is configured -- `NotificationSender`
//! is the seam.

use crate::backend::core::errors::{AppError, AppResult};
use crate::backend::notifications::notifications::{Notification, NotificationChannel, NotificationSender};

pub fn validate_email_address(address: &str) -> AppResult<()> {
    let at_count = address.matches('@').count();
    if at_count != 1 {
        return Err(AppError::Validation(format!("'{address}' is not a valid email address")));
    }
    let (local, domain) = address.split_once('@').unwrap();
    if local.is_empty() || domain.is_empty() || !domain.contains('.') {
        return Err(AppError::Validation(format!("'{address}' is not a valid email address")));
    }
    Ok(())
}

pub struct UnconfiguredEmailSender;

impl NotificationSender for UnconfiguredEmailSender {
    fn send(&self, notification: &Notification) -> AppResult<()> {
        if notification.channel != NotificationChannel::Email {
            return Err(AppError::Internal("UnconfiguredEmailSender received a non-email notification".into()));
        }
        validate_email_address(&notification.recipient)?;
        Err(AppError::Config("no SMTP server is configured -- set one up in Settings before sending email".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_well_formed_addresses() {
        assert!(validate_email_address("jane@example.com").is_ok());
    }

    #[test]
    fn rejects_addresses_without_an_at_sign() {
        assert!(validate_email_address("jane.example.com").is_err());
    }

    #[test]
    fn rejects_addresses_with_a_domain_missing_a_dot() {
        assert!(validate_email_address("jane@localhost").is_err());
    }

    #[test]
    fn unconfigured_sender_validates_then_fails_clearly() {
        let sender = UnconfiguredEmailSender;
        let valid = Notification::new(1, NotificationChannel::Email, "jane@example.com", "Hello");
        assert!(matches!(sender.send(&valid), Err(AppError::Config(_))));

        let invalid = Notification::new(1, NotificationChannel::Email, "not-an-email", "Hello");
        assert!(matches!(sender.send(&invalid), Err(AppError::Validation(_))));
    }
}
