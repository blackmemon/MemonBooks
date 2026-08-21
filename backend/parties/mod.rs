pub mod addresses;
pub mod contacts;
pub mod customers;
pub mod vendors;

/// Which kind of party a record belongs to -- shared by `addresses` and
/// `contacts`, which both attach to either one via `(party_type, party_id)`
/// rather than two separate foreign key columns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PartyType {
    Customer,
    Vendor,
}

impl PartyType {
    pub fn as_str(self) -> &'static str {
        match self {
            PartyType::Customer => "customer",
            PartyType::Vendor => "vendor",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "customer" => Some(PartyType::Customer),
            "vendor" => Some(PartyType::Vendor),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn as_str_and_parse_round_trip_for_every_variant() {
        for party_type in [PartyType::Customer, PartyType::Vendor] {
            assert_eq!(PartyType::parse(party_type.as_str()), Some(party_type));
        }
    }
}
