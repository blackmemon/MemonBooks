//! Text labels (Section 6): the muted caption/label and the bold dark
//! heading text that views repeat everywhere, plus money figures.

use iced::widget::{column, text, Text};
use iced::{Element, Length};
use rust_decimal::Decimal;

use crate::frontend::components::badges::{self, BadgeTone};
use crate::frontend::components::colors::Theme;
use crate::frontend::components::{fonts, spacing, typography};

/// Which side of the books a figure sits on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Debit,
    Credit,
}

/// The one rule for money colors by side: a debit is always success
/// (green) and a credit always danger (red), whether the figure raises
/// or lowers a net balance. A net balance takes the tone of its
/// account's own side; its sign stays visible in the text.
pub fn side_tone(side: Side) -> BadgeTone {
    match side {
        Side::Debit => BadgeTone::Positive,
        Side::Credit => BadgeTone::Negative,
    }
}

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

/// A muted label on the left and a bold value on the right, one line of
/// a summary list.
pub fn pair<'a, M: 'a>(
    theme: Theme,
    label: impl text::IntoFragment<'a>,
    value: impl text::IntoFragment<'a>,
) -> Element<'a, M> {
    entry(theme, label, strong(theme, value, typography::body()))
}

/// [`pair`] with any element as the value -- a pill, a dot, a colored
/// figure or a value with a button beside it.
pub fn entry<'a, M: 'a>(
    theme: Theme,
    label: impl text::IntoFragment<'a>,
    value: impl Into<Element<'a, M>>,
) -> Element<'a, M> {
    iced::widget::row![
        muted(theme, label, typography::body()).width(Length::Fill),
        value.into(),
    ]
    .spacing(spacing::SPACE_8)
    .align_y(iced::Alignment::Center)
    .into()
}

/// A muted caption over any element -- one cell of a dense details grid.
pub fn field<'a, M: 'a>(
    theme: Theme,
    label: impl text::IntoFragment<'a>,
    value: impl Into<Element<'a, M>>,
) -> Element<'a, M> {
    column![muted(theme, label, typography::caption()), value.into()]
        .spacing(spacing::SPACE_2)
        .into()
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
const MASK_GROUP: &str = "••••";
const MASK_GROUPS: usize = 3;
const TAIL_LEN: usize = 4;

/// An account number as `•••• •••• •••• 1234` (its last four
/// characters), or in full when `shown`.
pub fn masked_number(number: &str, shown: bool) -> String {
    let number = number.trim();
    if shown {
        return number.to_string();
    }
    let count = number.chars().count();
    let tail: String = number
        .chars()
        .skip(count.saturating_sub(TAIL_LEN))
        .collect();
    let mut groups = vec![MASK_GROUP; MASK_GROUPS];
    groups.push(&tail);
    groups.join(" ")
}

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
    fn side_tone_is_green_for_debit_and_red_for_credit_in_both_themes() {
        assert_eq!(side_tone(Side::Debit), BadgeTone::Positive);
        assert_eq!(side_tone(Side::Credit), BadgeTone::Negative);
        for theme in [Theme::Light, Theme::Dark] {
            let debit = badges::tone_colors(theme, side_tone(Side::Debit));
            let credit = badges::tone_colors(theme, side_tone(Side::Credit));
            assert_ne!(debit.1, credit.1);
            assert_eq!(debit, badges::tone_colors(theme, BadgeTone::Positive));
            assert_eq!(credit, badges::tone_colors(theme, BadgeTone::Negative));
        }
    }

    #[test]
    fn masked_shows_only_the_tail() {
        assert_eq!(masked("1234"), "•• 1234");
    }

    #[test]
    fn a_masked_number_shows_its_last_four_until_revealed() {
        assert_eq!(
            masked_number(" PK36HABB0001 ", false),
            "•••• •••• •••• 0001"
        );
        assert_eq!(masked_number("PK36HABB0001", true), "PK36HABB0001");
        assert_eq!(masked_number("12", false), "•••• •••• •••• 12");
        assert_ne!(
            masked_number("1234567", false),
            masked_number("1234567", true)
        );
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
            let _: Element<'_, ()> = pair(theme, "Total bins", "12");
            let _: Element<'_, ()> = field(theme, "Net", strong(theme, "1", 12.0));
            let _: Element<'_, ()> =
                entry(theme, "Net", amount(theme, "1", BadgeTone::Positive, 12.0));
        }
    }
}
