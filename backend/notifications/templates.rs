//! Notification message templates with `{{variable}}` placeholder
//! substitution.

use std::collections::HashMap;

use crate::backend::core::errors::{AppError, AppResult};
use crate::backend::core::validation::{require_non_empty, Validate};
use crate::backend::notifications::notifications::NotificationChannel;

#[derive(Debug, Clone, PartialEq)]
pub struct Template {
    pub id: Option<i64>,
    pub company_id: i64,
    pub name: String,
    pub channel: NotificationChannel,
    /// Only meaningful for channels that have one (email).
    pub subject: Option<String>,
    pub body: String,
}

impl Template {
    pub fn new(
        company_id: i64,
        name: impl Into<String>,
        channel: NotificationChannel,
        body: impl Into<String>,
    ) -> Self {
        Self {
            id: None,
            company_id,
            name: name.into(),
            channel,
            subject: None,
            body: body.into(),
        }
    }
}

impl Validate for Template {
    fn validate(&self) -> AppResult<()> {
        if self.company_id <= 0 {
            return Err(AppError::Validation(
                "template must reference a valid company".into(),
            ));
        }
        require_non_empty("template name", &self.name)?;
        require_non_empty("template body", &self.body)?;
        Ok(())
    }
}

/// Replaces every `{{key}}` in `body` with `variables[key]`. A key with
/// no matching variable is left as-is (visible, not silently dropped --
/// easier to spot a missing variable than a blank).
pub fn render(body: &str, variables: &HashMap<String, String>) -> String {
    let mut result = String::with_capacity(body.len());
    let mut rest = body;
    while let Some(start) = rest.find("{{") {
        result.push_str(&rest[..start]);
        let after_open = &rest[start + 2..];
        match after_open.find("}}") {
            Some(end) => {
                let key = after_open[..end].trim();
                match variables.get(key) {
                    Some(value) => result.push_str(value),
                    None => {
                        result.push_str("{{");
                        result.push_str(key);
                        result.push_str("}}");
                    }
                }
                rest = &after_open[end + 2..];
            }
            None => {
                result.push_str("{{");
                rest = after_open;
            }
        }
    }
    result.push_str(rest);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_substitutes_known_variables() {
        let mut vars = HashMap::new();
        vars.insert("invoice_number".to_string(), "INV-42".to_string());
        vars.insert("due_date".to_string(), "2026-02-01".to_string());

        let rendered = render("Invoice {{invoice_number}} is due on {{due_date}}.", &vars);
        assert_eq!(rendered, "Invoice INV-42 is due on 2026-02-01.");
    }

    #[test]
    fn render_leaves_unknown_placeholders_visible() {
        let rendered = render("Hello {{name}}", &HashMap::new());
        assert_eq!(rendered, "Hello {{name}}");
    }

    #[test]
    fn render_handles_a_body_with_no_placeholders() {
        let rendered = render("Plain text.", &HashMap::new());
        assert_eq!(rendered, "Plain text.");
    }

    #[test]
    fn validate_rejects_blank_body() {
        let template = Template::new(1, "Overdue reminder", NotificationChannel::Email, "");
        assert!(template.validate().is_err());
    }
}
