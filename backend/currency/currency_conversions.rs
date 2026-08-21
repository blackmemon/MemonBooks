//! Pure conversion arithmetic. Rate lookup/storage lives in
//! `exchange_rates.rs` and the organization repository; this module only
//! turns an amount + rate + target currency into a correctly-rounded
//! converted amount.

use rust_decimal::Decimal;

use crate::backend::currency::currencies::Currency;
use crate::backend::currency::exchange_rates::ExchangeRate;

/// Converts `amount` (denominated in `rate.from_currency`) into
/// `target` using `rate`, rounded to `target`'s usual decimal places.
///
/// Panics (via `debug_assert!`) in debug builds if `target`'s code
/// doesn't match `rate.to_currency` -- that mismatch is a caller bug,
/// not a recoverable runtime condition.
pub fn convert(amount: Decimal, rate: &ExchangeRate, target: &Currency) -> Decimal {
    debug_assert_eq!(
        rate.to_currency, target.code,
        "exchange rate's to_currency must match the target currency"
    );
    (amount * rate.rate).round_dp(target.decimal_places)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use rust_decimal_macros::dec;

    #[test]
    fn converts_and_rounds_to_target_decimal_places() {
        let usd = Currency::new("USD", "US Dollar", "$", 2);
        let rate = ExchangeRate::new(
            "PKR",
            "USD",
            dec!(0.0036),
            NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
        );
        // 100 PKR * 0.0036 = 0.3600 USD, rounded to 2dp.
        assert_eq!(convert(dec!(100), &rate, &usd), dec!(0.36));
    }

    #[test]
    fn rounds_down_to_zero_decimal_currency() {
        let jpy = Currency::new("JPY", "Japanese Yen", "\u{a5}", 0);
        let rate = ExchangeRate::new(
            "USD",
            "JPY",
            dec!(150.456),
            NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
        );
        assert_eq!(convert(dec!(10), &rate, &jpy), dec!(1505));
    }
}
