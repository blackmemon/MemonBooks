//! VAT-style tax arithmetic: net <-> gross conversions and the tax
//! amount itself, rounded to a currency's usual decimal places (Section
//! 6 of the currency domain, `Currency::decimal_places`).

use rust_decimal::Decimal;

/// The tax amount for a net amount at a given percent rate.
pub fn tax_amount(net_amount: Decimal, rate_percent: Decimal, decimal_places: u32) -> Decimal {
    (net_amount * rate_percent / Decimal::from(100)).round_dp(decimal_places)
}

/// Net amount plus its tax.
pub fn gross_amount(net_amount: Decimal, rate_percent: Decimal, decimal_places: u32) -> Decimal {
    net_amount + tax_amount(net_amount, rate_percent, decimal_places)
}

/// Backs out the net amount from a tax-inclusive (gross) amount.
pub fn net_amount_from_gross(
    gross_amount: Decimal,
    rate_percent: Decimal,
    decimal_places: u32,
) -> Decimal {
    let divisor = Decimal::ONE + rate_percent / Decimal::from(100);
    (gross_amount / divisor).round_dp(decimal_places)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn tax_amount_applies_the_percentage() {
        assert_eq!(tax_amount(dec!(100), dec!(15), 2), dec!(15.00));
    }

    #[test]
    fn zero_rate_produces_zero_tax() {
        assert_eq!(tax_amount(dec!(100), dec!(0), 2), dec!(0.00));
    }

    #[test]
    fn gross_amount_adds_tax_to_net() {
        assert_eq!(gross_amount(dec!(100), dec!(15), 2), dec!(115.00));
    }

    #[test]
    fn net_amount_from_gross_reverses_gross_amount() {
        let net = dec!(100);
        let rate = dec!(15);
        let gross = gross_amount(net, rate, 2);
        assert_eq!(net_amount_from_gross(gross, rate, 2), net);
    }

    #[test]
    fn rounds_to_the_given_decimal_places() {
        // 33.33 * 15% = 4.9995 -> rounds to 5.00 at 2dp.
        assert_eq!(tax_amount(dec!(33.33), dec!(15), 2), dec!(5.00));
    }
}
