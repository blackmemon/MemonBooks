//! Generic card/panel container style (Section 6): white background,
//! rounded corners, a hairline border and a contained shadow. Used by the
//! data table card, the sidebar's user profile card, and any future
//! panel that wants the same "floating card" treatment.

use iced::widget::text::IntoFragment;
use iced::widget::{column, container, text};
use iced::{Background, Element, Length, Theme as IcedTheme};

use crate::frontend::components::colors::Theme;
use crate::frontend::components::{borders, fonts, shadows, spacing, states, typography};

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
