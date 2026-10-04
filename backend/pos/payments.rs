//! Tender for a POS sale -- a sale can be split across more than one
//! payment method (e.g. part cash, part card).

use rust_decimal::Decimal;

use crate::backend::core::errors::AppResult;
use crate::backend::core::validation::require_positive;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PaymentMethod {
    Cash,
    Card,
    Other,
}

impl PaymentMethod {
    pub fn as_str(self) -> &'static str {
        match self {
            PaymentMethod::Cash => "cash",
            PaymentMethod::Card => "card",
            PaymentMethod::Other => "other",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "cash" => Some(PaymentMethod::Cash),
            "card" => Some(PaymentMethod::Card),
            "other" => Some(PaymentMethod::Other),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PosPayment {
    pub method: PaymentMethod,
    pub amount: Decimal,
}

impl PosPayment {
    pub fn new(method: PaymentMethod, amount: Decimal) -> Self {
        Self { method, amount }
    }

    pub fn validate(&self) -> AppResult<()> {
        require_positive("POS payment amount", self.amount)
    }
}

pub fn total_tendered(payments: &[PosPayment]) -> Decimal {
    payments.iter().map(|p| p.amount).sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn total_tendered_sums_every_payment() {
        let payments = vec![
            PosPayment::new(PaymentMethod::Cash, dec!(20)),
            PosPayment::new(PaymentMethod::Card, dec!(80)),
        ];
        assert_eq!(total_tendered(&payments), dec!(100));
    }

    #[test]
    fn validate_rejects_non_positive_amount() {
        let payment = PosPayment::new(PaymentMethod::Cash, Decimal::ZERO);
        assert!(payment.validate().is_err());
    }

    #[test]
    fn as_str_and_parse_round_trip_for_every_method() {
        for method in [
            PaymentMethod::Cash,
            PaymentMethod::Card,
            PaymentMethod::Other,
        ] {
            assert_eq!(PaymentMethod::parse(method.as_str()), Some(method));
        }
    }
}
