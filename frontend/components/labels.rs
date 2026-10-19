//! Text labels (Section 6): the muted caption/label and the bold dark
//! heading text that views repeat everywhere, plus money figures.

use iced::widget::{column, text, Text};
use iced::{Element, Length};
use rust_decimal::Decimal;

use crate::frontend::components::badges::{self, BadgeTone};
use crate::frontend::components::colors::Theme;
use crate::frontend::components::{fonts, spacing, typography};

/// A muted caption over a bold value, sharing its row equally with its
/// neighbours -- one figure in a summary strip.
pub fn stat<'a, M: 'a>(
    theme: Theme,
    label: impl text::IntoFragment<'a>,
    value: impl text::IntoFragment<'a>,
) -> Element<'a, M> {
    figure(
        theme,
        label,
        value,
        BadgeTone::Neutral,
        typography::emphasis(),
    )
}

/// [`stat`] with the value drawn in `tone`'s color at `size`.
pub fn figure<'a, M: 'a>(
    theme: Theme,
    label: impl text::IntoFragment<'a>,
    value: impl text::IntoFragment<'a>,
    tone: BadgeTone,
    size: f32,
) -> Element<'a, M> {
    column![
        muted(theme, label, typography::caption()),
        amount(theme, value, tone, size),
    ]
    .spacing(spacing::SPACE_2)
    .width(Length::FillPortion(1))
    .into()
}

/// Small gray text: captions, field labels, secondary lines.
pub fn muted<'a>(theme: Theme, value: impl text::IntoFragment<'a>, size: f32) -> Text<'a> {
    text(value).size(size).color(theme.neutral_front())
}

/// Bold dark text: page and panel titles, names.
pub fn strong<'a>(theme: Theme, value: impl text::IntoFragment<'a>, size: f32) -> Text<'a> {
    text(value)
        .size(size)
        .font(fonts::BOLD)
        .color(theme.neutral_font())
}

/// A bold figure in `tone`'s text color -- success/danger for money
/// moving up/down; a neutral tone draws it like [`strong`].
pub fn amount<'a>(
    theme: Theme,
    value: impl text::IntoFragment<'a>,
    tone: BadgeTone,
    size: f32,
) -> Text<'a> {
    let color = match tone {
        BadgeTone::Neutral => theme.neutral_font(),
        tone => badges::tone_colors(theme, tone).1,
    };
    text(value).size(size).font(fonts::BOLD).color(color)
}

/// `1,234.50`, with a leading `-` when negative.
pub fn money(amount: Decimal) -> String {
    let fixed = format!("{:.2}", amount.abs());
    let (whole, cents) = fixed.split_once('.').unwrap_or((&fixed, "00"));
    let mut grouped = String::new();
    for (i, digit) in whole.chars().enumerate() {
        if i > 0 && (whole.len() - i) % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(digit);
    }
    let sign = if amount.is_sign_negative() && !amount.round_dp(2).is_zero() {
        "-"
    } else {
        ""
    };
    format!("{sign}{grouped}.{cents}")
}

/// [`money`] after a currency code: `USD 1,234.50`.
pub fn currency(code: &str, amount: Decimal) -> String {
    format!("{code} {}", money(amount))
}

/// The visible tail of a masked number: `•• 1234`.
pub fn masked(tail: &str) -> String {
    format!("{MASK} {tail}")
}

const MASK: &str = "••";

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn money_groups_thousands_and_keeps_the_sign() {
        assert_eq!(money(dec!(1234567.5)), "1,234,567.50");
        assert_eq!(money(dec!(-42)), "-42.00");
        assert_eq!(money(dec!(0)), "0.00");
        assert_eq!(money(dec!(-0.001)), "0.00");
        assert_eq!(currency("PKR", dec!(1000)), "PKR 1,000.00");
    }

    #[test]
    fn masked_shows_only_the_tail() {
        assert_eq!(masked("1234"), "•• 1234");
    }

    #[test]
    fn every_label_builds_in_both_themes() {
        for theme in [Theme::Light, Theme::Dark] {
            for tone in [
                BadgeTone::Positive,
                BadgeTone::Negative,
                BadgeTone::Warning,
                BadgeTone::Neutral,
            ] {
                let _: Element<'_, ()> = figure(theme, "Net", "1.00", tone, typography::body());
            }
            let _: Element<'_, ()> = stat(theme, "Net", "1.00");
        }
    }
}
