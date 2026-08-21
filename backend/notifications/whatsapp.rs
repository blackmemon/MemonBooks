//! The WhatsApp channel. Delivery requires an approved WhatsApp Business
//! API account -- even more vendor/account-specific than SMS. Same
//! validate-then-fail-clearly pattern as `email.rs`/`sms.rs`; reuses
//! `sms::validate_phone_number` since a WhatsApp recipient is a phone
//! number too.

use crate::backend::core::errors::{AppError, AppResult};
use crate::backend::notifications::notifications::{Notification, NotificationChannel, NotificationSender};
use crate::backend::notifications::sms::validate_phone_number;

pub struct UnconfiguredWhatsAppSender;

impl NotificationSender for UnconfiguredWhatsAppSender {
    fn send(&self, notification: &Notification) -> AppResult<()> {
        if notification.channel != NotificationChannel::WhatsApp {
            return Err(AppError::Internal("UnconfiguredWhatsAppSender received a non-WhatsApp notification".into()));
        }
        validate_phone_number(&notification.recipient)?;
        Err(AppError::Config("no WhatsApp Business API account is configured -- set one up in Settings before sending WhatsApp messages".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unconfigured_sender_validates_then_fails_clearly() {
        let sender = UnconfiguredWhatsAppSender;
        let valid = Notification::new(1, NotificationChannel::WhatsApp, "+14155552671", "Your invoice is due");
        assert!(matches!(sender.send(&valid), Err(AppError::Config(_))));

        let invalid = Notification::new(1, NotificationChannel::WhatsApp, "not-a-number", "Your invoice is due");
        assert!(matches!(sender.send(&invalid), Err(AppError::Validation(_))));
    }
}
