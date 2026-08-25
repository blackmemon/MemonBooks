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
use crate::frontend::styles::{borders, spacing};

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
        icon_slot(&app_name.chars().take(2).collect::<String>(), theme.primary(), Color::WHITE),
        if collapsed { Space::with_width(Length::Fixed(0.0)).into() } else { bold_text(app_name, 15.0, theme.text_primary()) },
        Space::with_width(Length::Fill),
        button(text(if collapsed { "\u{203A}" } else { "\u{2039}" }).size(14))
            .style(move |_iced_theme: &IcedTheme, status| plain_button(theme, status))
            .on_press(on_toggle_collapse)
    ]
    .spacing(spacing::SM)
    .align_y(Alignment::Center)
    .padding([spacing::MD, spacing::SM]);

    let mut primary = column![].spacing(4.0);
    for (index, item) in primary_items.into_iter().enumerate() {
        primary = primary.push(nav_row(theme, item, collapsed, on_select_primary(index)));
    }

    let mut secondary = column![].spacing(4.0);
    for (index, item) in secondary_items.into_iter().enumerate() {
        secondary = secondary.push(nav_row(theme, item, collapsed, on_select_secondary(index)));
    }

    let profile_initial = user_name.chars().next().unwrap_or('?').to_uppercase().to_string();
    let profile_body: Element<'a, Message> = if collapsed {
        icon_slot(&profile_initial, theme.neutral_bg(), theme.neutral_front())
    } else {
        row![
            icon_slot(&profile_initial, theme.neutral_bg(), theme.neutral_front()),
            column![bold_text(user_name, 12.5, theme.text_primary()), text(user_subtitle).size(11).color(theme.neutral_front())].spacing(2.0),
            Space::with_width(Length::Fill),
            text("\u{25BE}").size(11).color(theme.neutral_front()),
        ]
        .spacing(spacing::SM)
        .align_y(Alignment::Center)
        .into()
    };
    let profile_card = button(container(profile_body).padding(spacing::SM).width(Length::Fill))
        .style(move |_iced_theme: &IcedTheme, status| profile_card_style(theme, status))
        .on_press(on_user_click)
        .width(Length::Fill);

    // Sun/moon toggle for the manual light/dark preference -- shows the
    // icon for the mode a click would switch *to*, sitting right beside
    // the profile card so it's always reachable regardless of collapse
    // state.
    let theme_toggle = button(text(if theme == Theme::Dark { "\u{2600}" } else { "\u{1F319}" }).size(14))
        .style(move |_iced_theme: &IcedTheme, status| plain_button(theme, status))
        .on_press(on_toggle_theme)
        .width(Length::Fixed(36.0));

    let bottom = column![
        container(Space::with_height(Length::Fixed(1.0))).width(Length::Fill).style(move |_iced_theme: &IcedTheme| container::Style {
            background: Some(Background::Color(theme.neutral_bg())),
            ..container::Style::default()
        }),
        container(secondary).padding([spacing::SM, spacing::SM]),
        container(row![profile_card, theme_toggle].spacing(spacing::XS).align_y(Alignment::Center))
            .padding(iced::Padding { top: 0.0, right: spacing::SM, bottom: spacing::SM, left: spacing::SM }),
    ]
    .spacing(spacing::SM);

    let body = column![logo_row, scrollable(container(primary).padding([0.0, spacing::SM])).height(Length::Fill), bottom].width(Length::Fill).height(Length::Fill);

    container(body)
        .width(Length::Fixed(if collapsed { COLLAPSED_WIDTH } else { EXPANDED_WIDTH }))
        .height(Length::Fill)
        .style(move |_iced_theme: &IcedTheme| container::Style {
            background: Some(Background::Color(theme.surface())),
            border: Border { color: theme.neutral_bg(), width: 1.0, radius: 0.0.into() },
            ..container::Style::default()
        })
        .into()
}

fn nav_row<'a, Message: Clone + 'a>(theme: Theme, item: NavEntry<'a>, collapsed: bool, on_press: Message) -> Element<'a, Message> {
    let (bg, front) = if item.active { (Color { a: 0.14, ..theme.primary() }, theme.primary()) } else { (Color::TRANSPARENT, theme.neutral_front()) };

    let monogram = item.label.chars().next().unwrap_or('?').to_uppercase().to_string();
    let content: Element<'a, Message> = if collapsed {
        container(text(monogram).size(12).color(front)).width(Length::Fill).align_x(iced::alignment::Horizontal::Center).into()
    } else {
        row![text(monogram).size(12).color(front).width(Length::Fixed(22.0)), text(item.label).size(13).color(front)].spacing(spacing::SM).align_y(Alignment::Center).into()
    };

    button(container(content).padding([10.0, 12.0]).width(Length::Fill))
        .style(move |_iced_theme: &IcedTheme, status: iced::widget::button::Status| {
            let background = if item.active {
                bg
            } else if matches!(status, iced::widget::button::Status::Hovered) {
                Color { a: 0.06, ..theme.neutral_front() }
            } else {
                Color::TRANSPARENT
            };
            iced::widget::button::Style {
                background: Some(Background::Color(background)),
                text_color: front,
                border: Border { radius: borders::RADIUS_MD.into(), ..Border::default() },
                shadow: iced::Shadow::default(),
            }
        })
        .on_press(on_press)
        .width(Length::Fill)
        .into()
}

fn icon_slot<'a, Message: 'a>(label: &str, bg: Color, front: Color) -> Element<'a, Message> {
    container(text(label.to_string()).size(12).color(front).font(Font { weight: iced::font::Weight::Bold, ..Font::DEFAULT }))
        .width(Length::Fixed(36.0))
        .height(Length::Fixed(36.0))
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center)
        .style(move |_iced_theme: &IcedTheme| container::Style { background: Some(Background::Color(bg)), border: Border { radius: borders::RADIUS_SM.into(), ..Border::default() }, ..container::Style::default() })
        .into()
}

fn bold_text<'a, Message: 'a>(value: &str, size: f32, color: Color) -> Element<'a, Message> {
    text(value.to_string()).size(size).color(color).font(Font { weight: iced::font::Weight::Bold, ..Font::DEFAULT }).into()
}

fn plain_button(theme: Theme, status: iced::widget::button::Status) -> iced::widget::button::Style {
    let background = if matches!(status, iced::widget::button::Status::Hovered) { Some(Background::Color(theme.neutral_bg())) } else { None };
    iced::widget::button::Style { background, text_color: theme.neutral_front(), border: Border { radius: borders::RADIUS_SM.into(), ..Border::default() }, shadow: iced::Shadow::default() }
}

fn profile_card_style(theme: Theme, status: iced::widget::button::Status) -> iced::widget::button::Style {
    let background = if matches!(status, iced::widget::button::Status::Hovered) { Color { a: 0.6, ..theme.neutral_bg() } } else { theme.neutral_bg() };
    iced::widget::button::Style {
        background: Some(Background::Color(background)),
        text_color: theme.text_primary(),
        border: Border { radius: borders::RADIUS_MD.into(), ..Border::default() },
        shadow: iced::Shadow::default(),
    }
}
