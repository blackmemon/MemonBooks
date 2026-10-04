pub mod email;
// The frozen project structure intentionally nests notifications.rs here.
#[allow(clippy::module_inception)]
pub mod notifications;
pub mod reminders;
pub mod sms;
pub mod templates;
pub mod whatsapp;
