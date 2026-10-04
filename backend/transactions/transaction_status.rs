//! A transaction's lifecycle state. Per Architecture Rules (Section 3),
//! posted entries are immutable -- there is no "edit a posted
//! transaction"; corrections are a new `Reversal` transaction, after
//! which the original is marked `Reversed` (a status/link change, not a
//! change to its financial content).

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TransactionStatus {
    /// Not yet posted; has no effect on the general ledger.
    Draft,
    /// Committed. Immutable financial content from this point on.
    Posted,
    /// Posted, and a reversal of it has since been posted too.
    Reversed,
}

impl TransactionStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            TransactionStatus::Draft => "draft",
            TransactionStatus::Posted => "posted",
            TransactionStatus::Reversed => "reversed",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "draft" => Some(TransactionStatus::Draft),
            "posted" => Some(TransactionStatus::Posted),
            "reversed" => Some(TransactionStatus::Reversed),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn as_str_and_parse_round_trip_for_every_variant() {
        for status in [
            TransactionStatus::Draft,
            TransactionStatus::Posted,
            TransactionStatus::Reversed,
        ] {
            assert_eq!(TransactionStatus::parse(status.as_str()), Some(status));
        }
    }
}
