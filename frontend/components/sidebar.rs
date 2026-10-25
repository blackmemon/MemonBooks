//! App shell sidebar: a black, rounded panel (black in both themes, see
//! the `sidebar_*` tokens in `colors.rs`) inset from the window edge. A
//! logo row sits on top, then the primary and secondary nav groups (line
//! icon plus label per row, active row on a lighter pill), and the
//! signed-in user block, theme toggle and collapse button at the bottom.
//! Collapsed, only the icons remain, centred.
//!
//! Labels never wrap: a label wider than its row is clipped, so the panel
//! width itself grows with the type scale (`spacing::sidebar_width`).

use iced::widget::button::{Status, Style as ButtonStyle};
use iced::widget::scrollable::{self, Direction, Scrollable, Scrollbar};
use iced::widget::text::Wrapping;
use iced::widget::{button, column, container, row, text, Space};
use iced::{Alignment, Background, Border, Element, Length, Theme as IcedTheme};

use crate::frontend::components::colors::Theme;
use crate::frontend::components::typography;
use crate::frontend::components::{fonts, icons, spacing, states};

/// Sidebar widths at the base type scale, outer margin included.
pub const EXPANDED_WIDTH: f32 = spacing::SIDEBAR_WIDTH;
pub const COLLAPSED_WIDTH: f32 = spacing::SIDEBAR_COLLAPSED_WIDTH;

/// The width the sidebar occupies now (type scale applied), margin included.
pub fn width(collapsed: bool) -> f32 {
    spacing::sidebar_width(collapsed)
}

#[derive(Clone, Copy)]
pub struct NavEntry<'a> {
    pub label: &'a str,
    /// A glyph from `icons.rs` (the Lucide font).
    pub icon: &'static str,
    pub active: bool,
}

impl<'a> NavEntry<'a> {
    pub fn new(label: &'a str, icon: &'static str, active: bool) -> Self {
        Self {
            label,
            icon,
            active,
        }
    }
}

/// Everything the sidebar shows and every message it can send.
pub struct Props<'a, Message> {
    pub app_name: &'a str,
    pub collapsed: bool,
    pub on_toggle_collapse: Message,
    pub primary_items: Vec<NavEntry<'a>>,
    pub on_select_primary: Box<dyn Fn(usize) -> Message + 'a>,
    pub secondary_items: Vec<NavEntry<'a>>,
    pub on_select_secondary: Box<dyn Fn(usize) -> Message + 'a>,
    pub user_name: &'a str,
    pub user_subtitle: &'a str,
    pub on_user_click: Message,
    pub on_toggle_theme: Message,
}

pub fn view<'a, Message: Clone + 'a>(
    theme: Theme,
    props: Props<'a, Message>,
) -> Element<'a, Message> {
    let Props {
        app_name,
        collapsed,
        on_toggle_collapse,
        primary_items,
        on_select_primary,
        secondary_items,
        on_select_secondary,
        user_name,
        user_subtitle,
        on_user_click,
        on_toggle_theme,
    } = props;

    let navigation = column![
        nav_group(theme, primary_items, collapsed, on_select_primary),
        divider(theme),
        nav_group(theme, secondary_items, collapsed, on_select_secondary),
    ]
    .spacing(spacing::SM);

    let body = column![
        logo_row(theme, app_name, collapsed),
        nav_scroll(theme, navigation),
        divider(theme),
        bottom(
            theme,
            collapsed,
            user_name,
            user_subtitle,
            on_user_click,
            on_toggle_theme,
            on_toggle_collapse,
        ),
    ]
    .spacing(spacing::SM)
    .width(Length::Fill)
    .height(Length::Fill);

    let panel = container(body)
        .padding(spacing::SM)
        .width(Length::Fill)
        .height(Length::Fill)
        .clip(true)
        .style(panel_style(theme));

    container(panel)
        .padding(spacing::SM)
        .width(Length::Fixed(width(collapsed)))
        .height(Length::Fill)
        .into()
}

/// Logo mark and app name; the name is dropped when collapsed.
fn logo_row<'a, Message: 'a>(
    theme: Theme,
    app_name: &'a str,
    collapsed: bool,
) -> Element<'a, Message> {
    let mark = icons::icon(icons::LOGO_BOOK)
        .size(typography::title())
        .color(theme.sidebar_text());
    let content: Element<'a, Message> = if collapsed {
        container(mark)
            .width(Length::Fill)
            .center_x(Length::Fill)
            .into()
    } else {
        row![
            mark,
            text(app_name)
                .size(typography::app_name())
                .color(theme.sidebar_text())
                .font(fonts::BOLD)
                .wrapping(Wrapping::None),
        ]
        .spacing(spacing::SM)
        .align_y(Alignment::Center)
        .into()
    };
    container(content)
        .padding([spacing::MD, spacing::SPACE_12])
        .width(Length::Fill)
        .into()
}

/// One group of nav rows; `on_select` maps a row index to its message.
fn nav_group<'a, Message: Clone + 'a>(
    theme: Theme,
    items: Vec<NavEntry<'a>>,
    collapsed: bool,
    on_select: Box<dyn Fn(usize) -> Message + 'a>,
) -> Element<'a, Message> {
    let mut group = column![].spacing(spacing::XS);
    for (index, item) in items.into_iter().enumerate() {
        group = group.push(nav_row(theme, item, collapsed, on_select(index)));
    }
    group.into()
}

/// A single menu row: icon slot, then the label (omitted when collapsed).
fn nav_row<'a, Message: Clone + 'a>(
    theme: Theme,
    item: NavEntry<'a>,
    collapsed: bool,
    on_press: Message,
) -> Element<'a, Message> {
    let glyph = icons::icon(item.icon)
        .size(typography::emphasis())
        .color(theme.sidebar_text())
        .width(Length::Fixed(spacing::sidebar_icon_slot()))
        .align_x(Alignment::Center);
    let content: Element<'a, Message> = if collapsed {
        container(glyph)
            .width(Length::Fill)
            .center_x(Length::Fill)
            .into()
    } else {
        row![
            glyph,
            text(item.label)
                .size(typography::label())
                .color(theme.sidebar_text())
                .wrapping(Wrapping::None),
        ]
        .spacing(spacing::SM)
        .align_y(Alignment::Center)
        .into()
    };
    button(content)
        .padding([spacing::SPACE_10, spacing::SPACE_12])
        .width(Length::Fill)
        .clip(true)
        .style(row_style(theme, item.active))
        .on_press(on_press)
        .into()
}

fn divider<'a, Message: 'a>(theme: Theme) -> Element<'a, Message> {
    container(Space::with_height(Length::Fixed(spacing::SPACE_1)))
        .width(Length::Fill)
        .style(move |_iced_theme: &IcedTheme| container::Style {
            background: Some(Background::Color(theme.sidebar_divider())),
            ..container::Style::default()
        })
        .into()
}

/// Keeps both nav groups in one scroll area so a short window can never
/// overlap them with the bottom block. The thin bar overlays the rows
/// (no gutter), so icons stay on the panel's centre line when collapsed.
fn nav_scroll<'a, Message: 'a>(
    theme: Theme,
    navigation: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    Scrollable::with_direction(
        navigation,
        Direction::Vertical(
            Scrollbar::new()
                .width(spacing::XS)
                .scroller_width(spacing::XS),
        ),
    )
    .height(Length::Fill)
    .style(scroll_style(theme))
    .into()
}

/// User block, then the theme toggle and collapse button. Collapsed, all
/// three stack as centred icons.
fn bottom<'a, Message: Clone + 'a>(
    theme: Theme,
    collapsed: bool,
    user_name: &'a str,
    user_subtitle: &'a str,
    on_user_click: Message,
    on_toggle_theme: Message,
    on_toggle_collapse: Message,
) -> Element<'a, Message> {
    let theme_glyph = if theme == Theme::Dark {
        icons::SUN
    } else {
        icons::MOON
    };
    let collapse_glyph = if collapsed {
        icons::CHEVRONS_RIGHT
    } else {
        icons::CHEVRONS_LEFT
    };
    let toggle = icon_button(theme, theme_glyph, on_toggle_theme);
    let collapse = icon_button(theme, collapse_glyph, on_toggle_collapse);
    let user = user_block(theme, collapsed, user_name, user_subtitle, on_user_click);

    if collapsed {
        column![user, toggle, collapse]
            .spacing(spacing::XS)
            .align_x(Alignment::Center)
            .width(Length::Fill)
            .into()
    } else {
        column![
            user,
            row![toggle, Space::with_width(Length::Fill), collapse].align_y(Alignment::Center),
        ]
        .spacing(spacing::XS)
        .width(Length::Fill)
        .into()
    }
}

/// The signed-in user: avatar glyph, name over subtitle, sign-out arrow.
fn user_block<'a, Message: Clone + 'a>(
    theme: Theme,
    collapsed: bool,
    user_name: &'a str,
    user_subtitle: &'a str,
    on_press: Message,
) -> Element<'a, Message> {
    let avatar = icons::icon(icons::USER)
        .size(typography::emphasis())
        .color(theme.sidebar_text())
        .width(Length::Fixed(spacing::sidebar_icon_slot()))
        .align_x(Alignment::Center);
    let content: Element<'a, Message> = if collapsed {
        container(avatar)
            .width(Length::Fill)
            .center_x(Length::Fill)
            .into()
    } else {
        row![
            avatar,
            column![
                text(user_name)
                    .size(typography::profile_name())
                    .color(theme.sidebar_text())
                    .font(fonts::BOLD)
                    .wrapping(Wrapping::None),
                text(user_subtitle)
                    .size(typography::caption())
                    .color(theme.sidebar_muted())
                    .wrapping(Wrapping::None),
            ]
            .spacing(spacing::SPACE_2),
            Space::with_width(Length::Fill),
            icons::icon(icons::LOGOUT)
                .size(typography::emphasis())
                .color(theme.sidebar_muted()),
        ]
        .spacing(spacing::SM)
        .align_y(Alignment::Center)
        .into()
    };
    button(content)
        .padding([spacing::SPACE_10, spacing::SPACE_12])
        .width(Length::Fill)
        .clip(true)
        .style(row_style(theme, false))
        .on_press(on_press)
        .into()
}

/// A square white-on-black icon button (theme toggle, collapse).
fn icon_button<'a, Message: Clone + 'a>(
    theme: Theme,
    glyph: &'static str,
    on_press: Message,
) -> Element<'a, Message> {
    button(
        icons::icon(glyph)
            .size(typography::emphasis())
            .color(theme.sidebar_text())
            .width(Length::Fixed(spacing::sidebar_icon_slot()))
            .align_x(Alignment::Center),
    )
    .padding([spacing::SPACE_10, spacing::SPACE_12])
    .style(row_style(theme, false))
    .on_press(on_press)
    .into()
}

/// The black rounded panel.
fn panel_style(theme: Theme) -> impl Fn(&IcedTheme) -> container::Style {
    move |_iced_theme: &IcedTheme| container::Style {
        background: Some(Background::Color(theme.sidebar_bg())),
        border: Border {
            radius: spacing::SIDEBAR_RADIUS.into(),
            ..Border::default()
        },
        ..container::Style::default()
    }
}

/// Row/button look: transparent, a lighter tint on hover, lighter still
/// when active (an active row keeps its tint under the pointer).
fn row_style(theme: Theme, active: bool) -> impl Fn(&IcedTheme, Status) -> ButtonStyle {
    move |_iced_theme: &IcedTheme, status: Status| {
        let tint = if active {
            Some(theme.sidebar_active())
        } else if matches!(status, Status::Hovered | Status::Pressed) {
            Some(theme.sidebar_hover())
        } else {
            None
        };
        ButtonStyle {
            background: tint.map(Background::Color),
            text_color: theme.sidebar_text(),
            border: Border {
                radius: states::RADIUS_LG.into(),
                ..Border::default()
            },
            shadow: iced::Shadow::default(),
        }
    }
}

/// Scroll bar for the black panel: no rail, a muted-grey thumb.
fn scroll_style(theme: Theme) -> impl Fn(&IcedTheme, scrollable::Status) -> scrollable::Style {
    move |iced_theme: &IcedTheme, status: scrollable::Status| {
        let mut style = scrollable::default(iced_theme, status);
        style.container = container::Style::default();
        style.vertical_rail.background = None;
        style.vertical_rail.scroller.color = theme.sidebar_active();
        style
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::components::font_scale;

    #[derive(Clone, Debug)]
    struct Msg;

    fn entries(count: usize) -> Vec<NavEntry<'static>> {
        (0..count)
            .map(|i| {
                NavEntry::new(
                    if i == 0 {
                        "Ledger Invoices / Bills"
                    } else {
                        "Point of Sale"
                    },
                    icons::NAV_HOME,
                    i == 1,
                )
            })
            .collect()
    }

    fn build(theme: Theme, collapsed: bool) -> Element<'static, Msg> {
        view(
            theme,
            Props {
                app_name: "MemonBooks",
                collapsed,
                on_toggle_collapse: Msg,
                primary_items: entries(30),
                on_select_primary: Box::new(|_| Msg),
                secondary_items: entries(4),
                on_select_secondary: Box::new(|_| Msg),
                user_name: "Administrator",
                user_subtitle: "Signed in",
                on_user_click: Msg,
                on_toggle_theme: Msg,
            },
        )
    }

    #[test]
    fn sidebar_builds_expanded_and_collapsed_at_every_width_in_both_themes() {
        let _guard = font_scale::TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let before = font_scale::window_width();
        for window in [640.0, 900.0, 1366.0, 1920.0] {
            font_scale::set_window_width(window);
            for theme in [Theme::Light, Theme::Dark] {
                for collapsed in [false, true] {
                    let _element = build(theme, collapsed);
                    assert!(width(collapsed) > 0.0);
                }
            }
        }
        font_scale::set_window_width(before);
    }

    #[test]
    fn rows_tint_on_hover_and_stay_lighter_when_active() {
        let theme = Theme::Light;
        let idle = row_style(theme, false)(&IcedTheme::Light, Status::Active);
        let hover = row_style(theme, false)(&IcedTheme::Light, Status::Hovered);
        let active = row_style(theme, true)(&IcedTheme::Light, Status::Hovered);
        assert_eq!(idle.background, None);
        assert_eq!(
            hover.background,
            Some(Background::Color(theme.sidebar_hover()))
        );
        assert_eq!(
            active.background,
            Some(Background::Color(theme.sidebar_active()))
        );
        assert_eq!(active.text_color, theme.sidebar_text());
    }

    #[test]
    fn the_collapsed_sidebar_is_narrower_than_the_expanded_one() {
        assert!(width(true) < width(false));
    }
}
