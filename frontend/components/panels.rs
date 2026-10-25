//! Generic card/panel container style (Section 6): white background,
//! rounded corners, a hairline border and a contained shadow. Used by the
//! data table card, the sidebar's user profile card, and any future
//! panel that wants the same "floating card" treatment.

use iced::widget::text::IntoFragment;
use iced::widget::text::Wrapping;
use iced::widget::{column, container, row, stack, text, Space, Text};
use iced::{Alignment, Background, Element, Length, Theme as IcedTheme};

use crate::frontend::components::badges::{self, BadgeTone};
use crate::frontend::components::colors::Theme;
use crate::frontend::components::{
    borders, fonts, icons, labels, shadows, spacing, states, table, typography,
};

/// The one outer card of a list page (Vendors, Bank Accounts,
/// Warehouses): `header` (title, toolbar), the `body` filling the middle,
/// and, when given, a divider and the `footer` pinned under it -- all
/// inside a single rounded, bordered card that fills the page area.
pub fn page_card<'a, M: 'a>(
    theme: Theme,
    header: Element<'a, M>,
    body: Element<'a, M>,
    footer: Option<Element<'a, M>>,
) -> Element<'a, M> {
    let mut card = column![header, body].height(Length::Fill);
    if let Some(footer) = footer {
        card = card.push(table::divider_line(theme)).push(footer);
    }
    container(card)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(spacing::SPACE_1)
        .style(table::card(theme, states::RADIUS_LG))
        .into()
}

/// Padded flat card spanning the width -- a block inside a panel.
pub fn padded<'a, M: 'a>(theme: Theme, content: impl Into<Element<'a, M>>) -> Element<'a, M> {
    container(content)
        .padding(spacing::MD)
        .width(Length::Fill)
        .style(flat_card(theme, states::RADIUS_MD))
        .into()
}

/// [`padded`] card with a bold primary title above its content.
pub fn titled_card<'a, M: 'a>(
    theme: Theme,
    title: impl IntoFragment<'a>,
    content: impl Into<Element<'a, M>>,
) -> Element<'a, M> {
    padded(
        theme,
        column![
            text(title)
                .size(typography::emphasis())
                .font(fonts::BOLD)
                .color(theme.primary()),
            content.into(),
        ]
        .spacing(spacing::SPACE_10),
    )
}

/// A side panel's own card: padded, filling the space it is given.
pub fn side<'a, M: 'a>(theme: Theme, content: impl Into<Element<'a, M>>) -> Element<'a, M> {
    container(content)
        .padding(spacing::MD)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(flat_card(theme, states::RADIUS_LG))
        .into()
}

pub fn card(theme: Theme, radius: f32) -> impl Fn(&IcedTheme) -> container::Style {
    move |_iced_theme: &IcedTheme| container::Style {
        background: Some(Background::Color(theme.page_bg())),
        border: borders::card(theme, radius),
        shadow: shadows::card(theme),
        ..container::Style::default()
    }
}

/// Same card, no shadow -- for panels nested inside an already-shadowed
/// ancestor (e.g. the profile card at the bottom of the sidebar).
pub fn flat_card(theme: Theme, radius: f32) -> impl Fn(&IcedTheme) -> container::Style {
    move |_iced_theme: &IcedTheme| container::Style {
        background: Some(Background::Color(theme.page_bg())),
        border: borders::card(theme, radius),
        ..container::Style::default()
    }
}

/// What a [`card_face`] shows, top to bottom.
pub struct CardFace<'a> {
    /// A muted caption over the big figure.
    pub caption: &'a str,
    pub figure: String,
    /// The masked line near the bottom.
    pub line: String,
    pub bottom_left: &'a str,
    pub bottom_right: &'a str,
    /// Fixed when the tile sits above something, `Fill` beside it.
    pub height: Length,
}

/// The tinted fill of a [`card_face`]: `tone`'s own background, so two
/// tones always look different and stay readable in both themes.
pub fn card_face_style(theme: Theme, tone: BadgeTone) -> impl Fn(&IcedTheme) -> container::Style {
    move |_iced_theme: &IcedTheme| container::Style {
        background: Some(Background::Color(badges::tone_colors(theme, tone).0)),
        border: borders::card(theme, states::RADIUS_LG),
        ..container::Style::default()
    }
}

/// A payment-card style tile: a two-circle logo mark and a chip on top,
/// a caption over a big figure in `tone`'s color, then a masked line and
/// two corner texts at the bottom. It fills the height it is given.
pub fn card_face<'a, M: 'a>(theme: Theme, tone: BadgeTone, face: CardFace<'a>) -> Element<'a, M> {
    let circle = || {
        icons::icon(icons::LOGO_CIRCLE)
            .size(typography::emphasis())
            .color(badges::tone_colors(theme, tone).1)
    };
    let logo = stack![
        circle(),
        container(circle()).padding(iced::Padding {
            left: spacing::SPACE_8,
            ..iced::Padding::ZERO
        }),
    ];
    let top = row![
        logo,
        Space::with_width(Length::Fill),
        icons::icon(icons::CHIP)
            .size(typography::subtitle())
            .color(theme.neutral_front()),
    ]
    .align_y(Alignment::Center);
    let clipped = |value: Text<'a>| {
        container(value.wrapping(Wrapping::None))
            .width(Length::Fill)
            .clip(true)
    };
    let bottom = row![
        clipped(labels::strong(
            theme,
            face.bottom_left,
            typography::caption()
        )),
        labels::strong(theme, face.bottom_right, typography::body()).wrapping(Wrapping::None),
    ]
    .spacing(spacing::SPACE_8);
    container(
        column![
            top,
            Space::with_height(Length::Fill),
            labels::muted(theme, face.caption, typography::caption()),
            clipped(labels::amount(
                theme,
                face.figure,
                tone,
                typography::subtitle()
            )),
            Space::with_height(Length::Fill),
            clipped(labels::muted(theme, face.line, typography::label())),
            bottom,
        ]
        .spacing(spacing::SPACE_2),
    )
    .padding(spacing::SPACE_10)
    .width(Length::Fill)
    .height(face.height)
    .style(card_face_style(theme, tone))
    .into()
}

/// A centered "nothing here yet" block: a muted icon, a bold message, an
/// optional hint and an optional action button under them.
pub fn empty_state<'a, M: 'a>(
    theme: Theme,
    glyph: &'static str,
    message: impl IntoFragment<'a>,
    hint: Option<&'a str>,
    action: Option<Element<'a, M>>,
) -> Element<'a, M> {
    let mut content = column![
        crate::frontend::components::icons::icon(glyph)
            .size(typography::title())
            .color(theme.neutral_front()),
        text(message)
            .size(typography::panel_title())
            .font(fonts::BOLD)
            .color(theme.neutral_font()),
    ]
    .spacing(spacing::SPACE_10)
    .align_x(iced::Alignment::Center);
    if let Some(hint) = hint {
        content = content.push(
            text(hint)
                .size(typography::body())
                .color(theme.neutral_front()),
        );
    }
    if let Some(action) = action {
        content = content.push(action);
    }
    container(content)
        .padding(spacing::SPACE_40)
        .center_x(Length::Fill)
        .style(flat_card(theme, states::RADIUS_LG))
        .into()
}

/// A thin rounded bar filled to `fraction` (0..=1) in `tone`'s text
/// color over the neutral track.
pub fn progress_bar<'a, M: 'a>(
    theme: Theme,
    fraction: f32,
    tone: crate::frontend::components::badges::BadgeTone,
) -> Element<'a, M> {
    iced::widget::progress_bar(0.0..=1.0, fraction.clamp(0.0, 1.0))
        .height(Length::Fixed(spacing::PROGRESS_BAR_HEIGHT))
        .style(
            move |_iced_theme: &IcedTheme| iced::widget::progress_bar::Style {
                background: Background::Color(theme.neutral_bg()),
                bar: Background::Color(match tone {
                    crate::frontend::components::badges::BadgeTone::Neutral => theme.primary(),
                    tone => crate::frontend::components::badges::tone_colors(theme, tone).1,
                }),
                border: iced::Border {
                    radius: states::RADIUS_PILL.into(),
                    ..iced::Border::default()
                },
            },
        )
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::components::badges::BadgeTone;

    #[test]
    fn a_card_face_tints_each_tone_differently_and_builds_in_both_themes() {
        use iced::Background;
        let fill = |theme, tone| {
            card_face_style(theme, tone)(&IcedTheme::Light)
                .background
                .map(|b| match b {
                    Background::Color(color) => color,
                    _ => theme.page_bg(),
                })
        };
        for theme in [Theme::Light, Theme::Dark] {
            assert_ne!(
                fill(theme, BadgeTone::Negative),
                fill(theme, BadgeTone::Positive)
            );
            for tone in [BadgeTone::Positive, BadgeTone::Negative] {
                let _: Element<'_, ()> = card_face(
                    theme,
                    tone,
                    CardFace {
                        caption: "Available",
                        figure: "PKR 1.00".into(),
                        line: "•••• 1234".into(),
                        bottom_left: "Bank",
                        bottom_right: "Debit",
                        height: Length::Fill,
                    },
                );
            }
        }
    }

    #[test]
    fn the_progress_bar_and_empty_state_build_in_both_themes() {
        for theme in [Theme::Light, Theme::Dark] {
            for fraction in [-1.0, 0.0, 0.5, 1.0, 2.5] {
                let _: Element<'_, ()> = progress_bar(theme, fraction, BadgeTone::Warning);
            }
            let _: Element<'_, ()> = empty_state(
                theme,
                crate::frontend::components::icons::LANDMARK,
                "Nothing yet",
                Some("hint"),
                None,
            );
        }
    }
}
