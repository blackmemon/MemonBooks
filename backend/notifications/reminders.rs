//! Building reminder notifications for overdue receivables
//! (`accounts_receivable::receivables`, Development Order step 9),
//! using a template (this step's `templates.rs`).

use chrono::NaiveDate;
use std::collections::HashMap;

use crate::backend::accounting::accounts_receivable::receivables::Receivable;
use crate::backend::notifications::notifications::{Notification, NotificationChannel};
use crate::backend::notifications::templates;

/// One reminder per overdue-and-unsettled receivable, addressed to
/// `customer_email(receivable.customer_id)`. A receivable whose customer
/// has no known email is skipped rather than failing the whole batch.
pub fn build_overdue_reminders(
    receivables: &[Receivable],
    as_of: NaiveDate,
    template_body: &str,
    customer_email: impl Fn(i64) -> Option<String>,
) -> Vec<Notification> {
    receivables
        .iter()
        .filter(|r| r.is_overdue(as_of))
        .filter_map(|r| {
            let email = customer_email(r.customer_id)?;
            let mut vars = HashMap::new();
            vars.insert(
                "reference".to_string(),
                r.reference.clone().unwrap_or_default(),
            );
            vars.insert("amount".to_string(), r.balance().to_string());
            vars.insert("due_date".to_string(), r.due_date.to_string());

            let body = templates::render(template_body, &vars);
            Some(Notification::new(
                r.company_id,
                NotificationChannel::Email,
                email,
                body,
            ))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    #[test]
    fn builds_one_reminder_per_overdue_receivable_with_a_known_email() {
        let mut overdue = Receivable::new(1, 10, date(2026, 1, 1), date(2026, 1, 15), dec!(500));
        overdue.reference = Some("INV-1".into());
        let mut not_yet_due =
            Receivable::new(1, 20, date(2026, 2, 1), date(2026, 2, 28), dec!(300));
        not_yet_due.customer_id = 20;

        let receivables = vec![overdue, not_yet_due];
        let reminders = build_overdue_reminders(
            &receivables,
            date(2026, 2, 1),
            "Please pay {{amount}} for {{reference}}, due {{due_date}}.",
            |customer_id| {
                if customer_id == 10 {
                    Some("jane@example.com".to_string())
                } else {
                    None
                }
            },
        );

        assert_eq!(reminders.len(), 1);
        assert_eq!(reminders[0].recipient, "jane@example.com");
        assert!(reminders[0].body.contains("500"));
        assert!(reminders[0].body.contains("INV-1"));
    }

    #[test]
    fn a_customer_with_no_known_email_is_skipped() {
        let overdue = Receivable::new(1, 10, date(2026, 1, 1), date(2026, 1, 15), dec!(500));
        let reminders = build_overdue_reminders(&[overdue], date(2026, 2, 1), "Reminder", |_| None);
        assert!(reminders.is_empty());
    }
}
