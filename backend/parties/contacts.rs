//! A contact person at a customer or vendor. A party can have more than
//! one; at most one should be `is_primary` (enforced by the repository,
//! not here -- that's a cross-row constraint).

use crate::backend::core::errors::{AppError, AppResult};
use crate::backend::core::validation::{require_max_length, require_non_empty, Validate};
use crate::backend::parties::PartyType;

#[derive(Debug, Clone, PartialEq)]
pub struct Contact {
    pub id: Option<i64>,
    pub party_type: PartyType,
    pub party_id: i64,
    pub name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub is_primary: bool,
}

impl Contact {
    pub fn new(party_type: PartyType, party_id: i64, name: impl Into<String>) -> Self {
        Self {
            id: None,
            party_type,
            party_id,
            name: name.into(),
            email: None,
            phone: None,
            is_primary: false,
        }
    }
}

impl Validate for Contact {
    fn validate(&self) -> AppResult<()> {
        if self.party_id <= 0 {
            return Err(AppError::Validation(
                "contact must reference a valid party".into(),
            ));
        }
        require_non_empty("contact name", &self.name)?;
        require_max_length("contact name", &self.name, 200)?;
        if let Some(email) = &self.email {
            if !email.contains('@') {
                return Err(AppError::Validation(
                    "contact email is not a valid email address".into(),
                ));
            }
            require_max_length("contact email", email, 200)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_rejects_blank_name() {
        let contact = Contact::new(PartyType::Vendor, 1, "");
        assert!(contact.validate().is_err());
    }

    #[test]
    fn validate_rejects_malformed_email() {
        let mut contact = Contact::new(PartyType::Vendor, 1, "Jane Doe");
        contact.email = Some("not-an-email".into());
        assert!(contact.validate().is_err());
    }

    #[test]
    fn validate_accepts_a_well_formed_contact() {
        let mut contact = Contact::new(PartyType::Vendor, 1, "Jane Doe");
        contact.email = Some("jane@example.com".into());
        assert!(contact.validate().is_ok());
    }
}
