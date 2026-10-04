//! App shell sidebar (Section 6): fixed-width nav rail with a logo row,
//! a primary nav list (active item gets a pill highlight), a secondary
//! nav list, and a user profile card at the bottom.
//!
//! Icons are monogram squares (first letter of the label/name), not a
//! real icon set -- there's no bundled icon font or SVG pipeline yet, so
//! this is a deliberate placeholder each `icon_slot` call can later be
//! swapped for a real glyph without touching layout.

use iced::widget::{button, column, container, row, scrollable, text, Space};
use iced::{Alignment, Background, Border, Color, Element, Font, Length, Theme as IcedTheme};

use crate::frontend::styles::colors::Theme;
use crate::frontend::styles::typography;
use crate::frontend::styles::{borders, icons, spacing, states};
use crate::frontend::styles::button as button_style;

pub const EXPANDED_WIDTH: f32 = 252.0;
pub const COLLAPSED_WIDTH: f32 = 72.0;

#[derive(Clone, Copy)]
pub struct NavEntry<'a> {
    pub label: &'a str,
    pub active: bool,
}

impl<'a> NavEntry<'a> {
    pub fn new(label: &'a str, active: bool) -> Self {
        Self { label, active }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn view<'a, Message: Clone + 'a>(
    theme: Theme,
    app_name: &'a str,
    collapsed: bool,
    on_toggle_collapse: Message,
    primary_items: Vec<NavEntry<'a>>,
    on_select_primary: impl Fn(usize) -> Message + 'a,
    secondary_items: Vec<NavEntry<'a>>,
    on_select_secondary: impl Fn(usize) -> Message + 'a,
    user_name: &'a str,
    user_subtitle: &'a str,
    on_user_click: Message,
    on_toggle_theme: Message,
) -> Element<'a, Message> {
    let logo_row = row![
        icon_slot(&app_name.chars().take(2).collect::<String>(), theme.primary(), theme.accent_front()),
        if collapsed { Space::with_width(Length::Fixed(0.0)).into() } else { bold_text(app_name, typography::app_name(), theme.text_primary()) },
        Space::with_width(Length::Fill),
        button(text(if collapsed { icons::CHEVRON_RIGHT } else { icons::CHEVRON_LEFT }).size(typography::emphasis()))
            .style(button_style::plain(theme))
            .on_press(on_toggle_collapse)
    ]
    .spacing(spacing::SM)
    .align_y(Alignment::Center)
    .padding([spacing::MD, spacing::SM]);

    let mut primary = column![].spacing(spacing::XS);
    for (index, item) in primary_items.into_iter().enumerate() {
        primary = primary.push(nav_row(theme, item, collapsed, on_select_primary(index)));
    }

    let mut secondary = column![].spacing(spacing::XS);
    for (index, item) in secondary_items.into_iter().enumerate() {
        secondary = secondary.push(nav_row(theme, item, collapsed, on_select_secondary(index)));
    }

    let profile_initial = user_name.chars().next().unwrap_or('?').to_uppercase().to_string();
    let profile_body: Element<'a, Message> = if collapsed {
        icon_slot(&profile_initial, theme.neutral_bg(), theme.neutral_front())
    } else {
        row![
            icon_slot(&profile_initial, theme.neutral_bg(), theme.neutral_front()),
            column![bold_text(user_name, typography::profile_name(), theme.text_primary()), text(user_subtitle).size(typography::caption()).color(theme.neutral_front())].spacing(spacing::SPACE_2),
            Space::with_width(Length::Fill),
            text(icons::CARET_DOWN).size(typography::caption()).color(theme.neutral_front()),
        ]
        .spacing(spacing::SM)
        .align_y(Alignment::Center)
        .into()
    };
    let profile_card = button(container(profile_body).padding(spacing::SM).width(Length::Fill))
        .style(button_style::profile_card(theme))
        .on_press(on_user_click)
        .width(Length::Fill);

    // Sun/moon toggle for the manual light/dark preference -- shows the
    // icon for the mode a click would switch *to*, sitting right beside
    // the profile card so it's always reachable regardless of collapse
    // state.
    let theme_toggle = button(text(if theme == Theme::Dark { icons::SUN } else { icons::MOON }).size(typography::emphasis()))
        .style(button_style::plain(theme))
        .on_press(on_toggle_theme)
        .width(Length::Fixed(36.0));

    let divider = container(Space::with_height(Length::Fixed(1.0))).width(Length::Fill).style(move |_iced_theme: &IcedTheme| container::Style {
            background: Some(Background::Color(theme.neutral_bg())),
            ..container::Style::default()
        });
    // Keep both navigation groups in the same scroll area so short windows
    // cannot overlap the secondary routes with the profile card.
    let navigation = column![
        container(primary).padding([0.0, spacing::SM]),
        divider,
        container(secondary).padding([spacing::SM, spacing::SM]),
    ]
    .spacing(spacing::SM);
    let bottom = column![
        container(row![profile_card, theme_toggle].spacing(spacing::XS).align_y(Alignment::Center))
            .padding(iced::Padding { top: 0.0, right: spacing::SM, bottom: spacing::SM, left: spacing::SM }),
    ]
    .spacing(spacing::SM);

    let body = column![logo_row, scrollable(navigation).height(Length::Fill), bottom].width(Length::Fill).height(Length::Fill);

    container(body)
        .width(Length::Fixed(if collapsed { COLLAPSED_WIDTH } else { EXPANDED_WIDTH }))
        .height(Length::Fill)
        .style(move |_iced_theme: &IcedTheme| container::Style {
            background: Some(Background::Color(theme.surface())),
            border: borders::card(theme, 0.0),
            ..container::Style::default()
        })
        .into()
}

fn nav_row<'a, Message: Clone + 'a>(theme: Theme, item: NavEntry<'a>, collapsed: bool, on_press: Message) -> Element<'a, Message> {
    let front = if item.active { theme.primary() } else { theme.neutral_front() };

    let monogram = item.label.chars().next().unwrap_or('?').to_uppercase().to_string();
    let content: Element<'a, Message> = if collapsed {
        container(text(monogram).size(typography::body()).color(front)).width(Length::Fill).align_x(iced::alignment::Horizontal::Center).into()
    } else {
        row![text(monogram).size(typography::body()).color(front).width(Length::Fixed(22.0)), text(item.label).size(typography::label()).color(front)].spacing(spacing::SM).align_y(Alignment::Center).into()
    };

    button(container(content).padding([spacing::SPACE_10, spacing::SPACE_12]).width(Length::Fill))
        .style(button_style::nav_item(theme, item.active))
        .on_press(on_press)
        .width(Length::Fill)
        .into()
}

fn icon_slot<'a, Message: 'a>(label: &str, bg: Color, front: Color) -> Element<'a, Message> {
    container(text(label.to_string()).size(typography::body()).color(front).font(Font { weight: iced::font::Weight::Bold, ..Font::DEFAULT }))
        .width(Length::Fixed(36.0))
        .height(Length::Fixed(36.0))
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center)
        .style(move |_iced_theme: &IcedTheme| container::Style { background: Some(Background::Color(bg)), border: Border { radius: states::RADIUS_SM.into(), ..Border::default() }, ..container::Style::default() })
        .into()
}

fn bold_text<'a, Message: 'a>(value: &str, size: f32, color: Color) -> Element<'a, Message> {
    text(value.to_string()).size(size).color(color).font(Font { weight: iced::font::Weight::Bold, ..Font::DEFAULT }).into()
}
