//! A notification queued to go out, and the `NotificationSender` trait
//! each channel (`email.rs`, `sms.rs`, `whatsapp.rs`) implements to
//! actually deliver one.

use chrono::{DateTime, Utc};

use crate::backend::core::errors::{AppError, AppResult};
use crate::backend::core::validation::require_non_empty;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NotificationChannel {
    Email,
    Sms,
    WhatsApp,
    InApp,
}

impl NotificationChannel {
    pub fn as_str(self) -> &'static str {
        match self {
            NotificationChannel::Email => "email",
            NotificationChannel::Sms => "sms",
            NotificationChannel::WhatsApp => "whatsapp",
            NotificationChannel::InApp => "in_app",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "email" => Some(NotificationChannel::Email),
            "sms" => Some(NotificationChannel::Sms),
            "whatsapp" => Some(NotificationChannel::WhatsApp),
            "in_app" => Some(NotificationChannel::InApp),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NotificationStatus {
    Pending,
    Sent,
    Failed,
}

impl NotificationStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            NotificationStatus::Pending => "pending",
            NotificationStatus::Sent => "sent",
            NotificationStatus::Failed => "failed",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "pending" => Some(NotificationStatus::Pending),
            "sent" => Some(NotificationStatus::Sent),
            "failed" => Some(NotificationStatus::Failed),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Notification {
    pub id: Option<i64>,
    pub company_id: i64,
    pub channel: NotificationChannel,
    pub recipient: String,
    pub subject: Option<String>,
    pub body: String,
    pub status: NotificationStatus,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub sent_at: Option<DateTime<Utc>>,
}

impl Notification {
    pub fn new(company_id: i64, channel: NotificationChannel, recipient: impl Into<String>, body: impl Into<String>) -> Self {
        Self {
            id: None,
            company_id,
            channel,
            recipient: recipient.into(),
            subject: None,
            body: body.into(),
            status: NotificationStatus::Pending,
            error_message: None,
            created_at: Utc::now(),
            sent_at: None,
        }
    }

    pub fn validate(&self) -> AppResult<()> {
        if self.company_id <= 0 {
            return Err(AppError::Validation("notification must reference a valid company".into()));
        }
        require_non_empty("notification recipient", &self.recipient)?;
        require_non_empty("notification body", &self.body)?;
        Ok(())
    }

    /// Only a `Pending` or `Failed` notification is eligible to (re)send;
    /// a `Sent` one is done.
    pub fn is_eligible_to_send(&self) -> bool {
        matches!(self.status, NotificationStatus::Pending | NotificationStatus::Failed)
    }
}

/// Implemented once per channel. Takes `&Notification` and either sends
/// it or returns why it couldn't -- the caller (a repository, once wired
/// up) updates status/sent_at/error_message from the result.
pub trait NotificationSender {
    fn send(&self, notification: &Notification) -> AppResult<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn as_str_and_parse_round_trip_for_every_channel() {
        for channel in [NotificationChannel::Email, NotificationChannel::Sms, NotificationChannel::WhatsApp, NotificationChannel::InApp] {
            assert_eq!(NotificationChannel::parse(channel.as_str()), Some(channel));
        }
    }

    #[test]
    fn as_str_and_parse_round_trip_for_every_status() {
        for status in [NotificationStatus::Pending, NotificationStatus::Sent, NotificationStatus::Failed] {
            assert_eq!(NotificationStatus::parse(status.as_str()), Some(status));
        }
    }

    #[test]
    fn only_pending_or_failed_are_eligible_to_send() {
        let mut notification = Notification::new(1, NotificationChannel::Email, "jane@example.com", "Hello");
        assert!(notification.is_eligible_to_send());
        notification.status = NotificationStatus::Sent;
        assert!(!notification.is_eligible_to_send());
        notification.status = NotificationStatus::Failed;
        assert!(notification.is_eligible_to_send());
    }

    #[test]
    fn validate_rejects_blank_recipient() {
        let notification = Notification::new(1, NotificationChannel::Email, "", "Hello");
        assert!(notification.validate().is_err());
    }
}
