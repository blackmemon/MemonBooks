//! A postal address belonging to a customer or vendor. A party can have
//! more than one (billing, shipping, ...), distinguished by `label`.

use crate::backend::core::errors::{AppError, AppResult};
use crate::backend::core::validation::{require_max_length, require_non_empty, Validate};
use crate::backend::parties::PartyType;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AddressLabel {
    Billing,
    Shipping,
    Other,
}

impl AddressLabel {
    pub fn as_str(self) -> &'static str {
        match self {
            AddressLabel::Billing => "billing",
            AddressLabel::Shipping => "shipping",
            AddressLabel::Other => "other",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "billing" => Some(AddressLabel::Billing),
            "shipping" => Some(AddressLabel::Shipping),
            "other" => Some(AddressLabel::Other),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Address {
    pub id: Option<i64>,
    pub party_type: PartyType,
    pub party_id: i64,
    pub label: AddressLabel,
    pub line1: String,
    pub line2: Option<String>,
    pub city: String,
    pub state: Option<String>,
    pub postal_code: Option<String>,
    pub country: String,
}

impl Address {
    pub fn new(party_type: PartyType, party_id: i64, label: AddressLabel, line1: impl Into<String>, city: impl Into<String>, country: impl Into<String>) -> Self {
        Self {
            id: None,
            party_type,
            party_id,
            label,
            line1: line1.into(),
            line2: None,
            city: city.into(),
            state: None,
            postal_code: None,
            country: country.into(),
        }
    }
}

impl Validate for Address {
    fn validate(&self) -> AppResult<()> {
        if self.party_id <= 0 {
            return Err(AppError::Validation("address must reference a valid party".into()));
        }
        require_non_empty("address line 1", &self.line1)?;
        require_max_length("address line 1", &self.line1, 200)?;
        require_non_empty("city", &self.city)?;
        require_max_length("city", &self.city, 100)?;
        require_non_empty("country", &self.country)?;
        require_max_length("country", &self.country, 100)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn as_str_and_parse_round_trip_for_every_label() {
        for label in [AddressLabel::Billing, AddressLabel::Shipping, AddressLabel::Other] {
            assert_eq!(AddressLabel::parse(label.as_str()), Some(label));
        }
    }

    #[test]
    fn validate_rejects_blank_required_fields() {
        let address = Address::new(PartyType::Customer, 1, AddressLabel::Billing, "", "Karachi", "PK");
        assert!(address.validate().is_err());
    }

    #[test]
    fn validate_accepts_a_well_formed_address() {
        let address = Address::new(PartyType::Customer, 1, AddressLabel::Billing, "123 Main St", "Karachi", "PK");
        assert!(address.validate().is_ok());
    }
}
