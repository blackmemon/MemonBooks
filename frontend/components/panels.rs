//! Generic card/panel container style (Section 6): white background,
//! rounded corners, a hairline border and a contained shadow. Used by the
//! data table card, the sidebar's user profile card, and any future
//! panel that wants the same "floating card" treatment.

use iced::widget::text::IntoFragment;
use iced::widget::{column, container, text};
use iced::{Background, Element, Length, Theme as IcedTheme};

use crate::frontend::components::colors::Theme;
use crate::frontend::components::{borders, fonts, shadows, spacing, states, table, typography};

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
