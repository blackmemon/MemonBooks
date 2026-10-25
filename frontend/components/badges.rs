//! Status pill tones (Section 6): maps a meaning to the existing White
//! Theme background/front color pair. No new colors -- this only picks
//! which existing pair applies, plus the one new neutral pair.

use iced::widget::text::IntoFragment;
use iced::widget::{container, row, text, Space};
use iced::{Background, Border, Color, Element, Length, Theme as IcedTheme};

use crate::frontend::components::colors::Theme;
use crate::frontend::components::{fonts, icons, spacing, states, typography};

/// Small tinted status pill: bold caption text on the tone's background.
pub fn status_pill<'a, M: 'a>(
    theme: Theme,
    label: impl IntoFragment<'a>,
    tone: BadgeTone,
) -> Element<'a, M> {
    container(
        text(label)
            .size(typography::caption())
            .font(fonts::BOLD)
            .color(tone_colors(theme, tone).1),
    )
    .padding([spacing::SPACE_2, spacing::SPACE_10])
    .style(pill(theme, tone))
    .into()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BadgeTone {
    Positive,
    Negative,
    Warning,
    Neutral,
}

/// Returns `(background, foreground)` for the given tone under the
/// active theme.
pub fn tone_colors(theme: Theme, tone: BadgeTone) -> (Color, Color) {
    match tone {
        BadgeTone::Positive => (theme.positive_bg(), theme.positive_front()),
        BadgeTone::Negative => (theme.negative_bg(), theme.negative_front()),
        BadgeTone::Warning => (theme.warning_bg(), theme.warning_front()),
        BadgeTone::Neutral => (theme.neutral_bg(), theme.neutral_front()),
    }
}

/// Pill for a count next to a tab label: primary tint when `active`,
/// neutral otherwise.
pub fn count(theme: Theme, active: bool) -> impl Fn(&IcedTheme) -> container::Style {
    move |_iced_theme: &IcedTheme| container::Style {
        background: Some(Background::Color(if active {
            theme.primary_tint(states::SELECTED_ALPHA)
        } else {
            theme.neutral_bg()
        })),
        border: Border {
            radius: states::RADIUS_PILL.into(),
            ..Border::default()
        },
        ..container::Style::default()
    }
}

/// Plain tinted status pill (no dot): the tone's background, fully
/// rounded. Pair the text with `tone_colors(..).1`.
pub fn pill(theme: Theme, tone: BadgeTone) -> impl Fn(&IcedTheme) -> container::Style {
    move |_iced_theme: &IcedTheme| container::Style {
        background: Some(Background::Color(tone_colors(theme, tone).0)),
        border: Border {
            radius: states::RADIUS_PILL.into(),
            ..Border::default()
        },
        ..container::Style::default()
    }
}

/// Round initials avatar: a primary tint disc. Size it with
/// `spacing::AVATAR_SIZE` and color the initials `theme.primary()`.
pub fn avatar(theme: Theme) -> impl Fn(&IcedTheme) -> container::Style {
    move |_iced_theme: &IcedTheme| container::Style {
        background: Some(Background::Color(
            theme.primary_tint(states::SELECTED_ALPHA),
        )),
        border: Border {
            radius: states::RADIUS_PILL.into(),
            ..Border::default()
        },
        ..container::Style::default()
    }
}

/// Up to two capital initials of `name`; `?` when it has none.
pub fn initials(name: &str) -> String {
    let letters: String = name
        .split_whitespace()
        .filter_map(|word| word.chars().next())
        .take(2)
        .flat_map(char::to_uppercase)
        .collect();
    if letters.is_empty() {
        "?".to_string()
    } else {
        letters
    }
}

/// A round avatar `size` across holding `name`'s initials.
pub fn initials_avatar<'a, M: 'a>(theme: Theme, name: &str, size: f32) -> Element<'a, M> {
    container(
        text(initials(name))
            .size(typography::subtitle())
            .font(fonts::BOLD)
            .color(theme.primary()),
    )
    .center_x(Length::Fixed(size))
    .center_y(Length::Fixed(size))
    .style(avatar(theme))
    .into()
}

/// The round marker of a timeline entry: `glyph` on `tone`'s tint.
pub fn timeline_dot<'a, M: 'a>(
    theme: Theme,
    glyph: &'static str,
    tone: BadgeTone,
) -> Element<'a, M> {
    container(
        icons::icon(glyph)
            .size(typography::label())
            .color(tone_colors(theme, tone).1),
    )
    .center_x(Length::Fixed(spacing::grown(spacing::TIMELINE_DOT)))
    .center_y(Length::Fixed(spacing::grown(spacing::TIMELINE_DOT)))
    .style(move |_iced_theme: &IcedTheme| container::Style {
        background: Some(Background::Color(tone_colors(theme, tone).0)),
        border: Border {
            radius: states::RADIUS_PILL.into(),
            ..Border::default()
        },
        ..container::Style::default()
    })
    .into()
}

/// The tone for a signed amount: positive up, negative down, neutral at
/// zero.
pub fn sign_tone(amount: rust_decimal::Decimal) -> BadgeTone {
    if amount.is_zero() {
        BadgeTone::Neutral
    } else if amount.is_sign_negative() {
        BadgeTone::Negative
    } else {
        BadgeTone::Positive
    }
}

/// A rounded square tinted with `tone`'s background, holding `glyph` in
/// the tone's text color -- the leading mark of a card or list row.
pub fn icon_box<'a, M: 'a>(theme: Theme, glyph: &'static str, tone: BadgeTone) -> Element<'a, M> {
    container(
        icons::icon(glyph)
            .size(typography::panel_title())
            .color(tone_colors(theme, tone).1),
    )
    .center_x(Length::Fixed(spacing::icon_box_size()))
    .center_y(Length::Fixed(spacing::icon_box_size()))
    .style(move |_iced_theme: &IcedTheme| container::Style {
        background: Some(Background::Color(tone_colors(theme, tone).0)),
        border: Border {
            radius: states::RADIUS_MD.into(),
            ..Border::default()
        },
        ..container::Style::default()
    })
    .into()
}

/// `(background, foreground)` for a filled cell or swatch: the tone's
/// pair, except that a neutral tone is the plain surface.
pub fn fill_colors(theme: Theme, tone: BadgeTone) -> (Color, Color) {
    match tone {
        BadgeTone::Neutral => (theme.surface(), theme.neutral_font()),
        tone => tone_colors(theme, tone),
    }
}

/// One legend entry: a small swatch in `tone`'s fill and its label.
pub fn legend<'a, M: 'a>(
    theme: Theme,
    tone: BadgeTone,
    label: impl IntoFragment<'a>,
) -> Element<'a, M> {
    row![
        container(Space::new(
            Length::Fixed(spacing::LEGEND_SWATCH_SIZE),
            Length::Fixed(spacing::LEGEND_SWATCH_SIZE)
        ))
        .style(move |_iced_theme: &IcedTheme| container::Style {
            background: Some(Background::Color(fill_colors(theme, tone).0)),
            border: crate::frontend::components::borders::card(theme, states::RADIUS_SM),
            ..container::Style::default()
        }),
        text(label)
            .size(typography::caption())
            .color(theme.neutral_front()),
    ]
    .spacing(spacing::SPACE_6)
    .align_y(iced::Alignment::Center)
    .into()
}

/// A coloured dot before a muted label -- a record's state at a glance.
pub fn status_dot<'a, M: 'a>(
    theme: Theme,
    label: impl IntoFragment<'a>,
    tone: BadgeTone,
) -> Element<'a, M> {
    row![
        icons::icon(icons::STATUS_DOT)
            .size(typography::emphasis())
            .color(tone_colors(theme, tone).1),
        text(label)
            .size(typography::label())
            .color(theme.neutral_front()),
    ]
    .spacing(spacing::SPACE_4)
    .align_y(iced::Alignment::Center)
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn legend_status_dot_and_fill_colors_build_in_both_themes() {
        for theme in [Theme::Light, Theme::Dark] {
            assert_eq!(fill_colors(theme, BadgeTone::Neutral).0, theme.surface());
            assert_eq!(
                fill_colors(theme, BadgeTone::Warning),
                tone_colors(theme, BadgeTone::Warning)
            );
            let _: Element<'_, ()> = legend(theme, BadgeTone::Negative, "Full");
            let _: Element<'_, ()> = status_dot(theme, "Active", BadgeTone::Positive);
        }
    }

    #[test]
    fn sign_tone_follows_the_sign() {
        assert_eq!(sign_tone(dec!(5)), BadgeTone::Positive);
        assert_eq!(sign_tone(dec!(-5)), BadgeTone::Negative);
        assert_eq!(sign_tone(dec!(0)), BadgeTone::Neutral);
    }

    #[test]
    fn icon_box_builds_for_every_tone_in_both_themes() {
        for theme in [Theme::Light, Theme::Dark] {
            for tone in [
                BadgeTone::Positive,
                BadgeTone::Negative,
                BadgeTone::Warning,
                BadgeTone::Neutral,
            ] {
                let _: Element<'_, ()> = icon_box(theme, icons::LANDMARK, tone);
            }
        }
    }
}
