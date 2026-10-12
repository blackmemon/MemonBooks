use iced::widget::checkbox::{Status, Style};
use iced::widget::{button, container, Space};
use iced::{Background, Border, Color, Element, Length};

use crate::frontend::components::colors::Theme;
use crate::frontend::components::states;

pub fn style(theme: Theme) -> impl Fn(&iced::Theme, Status) -> Style {
    move |_iced_theme: &iced::Theme, status: Status| {
        let is_checked = matches!(
            status,
            Status::Active { is_checked: true }
                | Status::Hovered { is_checked: true }
                | Status::Disabled { is_checked: true }
        );

        let is_disabled = matches!(status, Status::Disabled { .. });
        let is_hovered = matches!(status, Status::Hovered { .. });
        let background = match (is_checked, is_disabled, is_hovered) {
            (_, true, _) => theme.neutral_bg(),

            (true, false, true) => Color {
                a: states::SOLID_BUTTON_HOVER_ALPHA,
                ..theme.primary()
            },

            (true, false, false) => theme.primary(),
            (false, false, true) => theme.surface_hover(),
            (false, false, false) => theme.surface(),
        };

        let border_color = match (is_checked, is_disabled, is_hovered) {
            (_, true, _) => theme.neutral_border(),
            (true, false, _) => theme.primary(),
            (false, false, true) => theme.neutral_border_hover(),
            (false, false, false) => theme.neutral_border(),
        };

        Style {
            background: Background::Color(background),
            icon_color: theme.transparent(),
            border: Border {
                color: border_color,
                width: 1.0,
                radius: 2.0.into(),
            },
            text_color: if is_disabled {
                Some(theme.neutral_front())
            } else {
                None
            },
        }
    }
}

/// State of a select-all checkbox over `selected` of `total` rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckState {
    Unchecked,
    Partial,
    Checked,
}

impl CheckState {
    pub fn from_counts(selected: usize, total: usize) -> Self {
        if total == 0 || selected == 0 {
            CheckState::Unchecked
        } else if selected >= total {
            CheckState::Checked
        } else {
            CheckState::Partial
        }
    }
}

/// Width and height of the flat table indicator (the reference
/// design's 26x12 `table_checkbox`).
pub const INDICATOR_WIDTH: f32 = 26.0;
pub const INDICATOR_HEIGHT: f32 = 15.0;
pub const COLUMN_WIDTH: f32 = 32.0;

/// Flat pill-less indicator style: primary (#2C8FBD) when checked or
/// partial, grey (#D1D5DB) when empty.
fn indicator_style(
    theme: Theme,
    state: CheckState,
) -> impl Fn(&iced::Theme, button::Status) -> button::Style {
    move |_iced_theme: &iced::Theme, status: button::Status| {
        let filled = state != CheckState::Unchecked;
        let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
        let light = matches!(theme, Theme::Light);
        let empty = if light {
            Color::from_rgb8(0xD1, 0xD5, 0xDB)
        } else {
            theme.neutral_border()
        };
        let fill = if filled { theme.primary() } else { empty };
        let background = if hovered && filled {
            Color {
                a: states::SOLID_BUTTON_HOVER_ALPHA,
                ..fill
            }
        } else {
            fill
        };
        let outline = if filled { theme.primary() } else { empty };
        button::Style {
            background: Some(Background::Color(background)),
            text_color: theme.accent_front(),
            border: Border {
                color: outline,
                width: 0.0,
                radius: 1.0.into(),
            },
            shadow: iced::Shadow::default(),
        }
    }
}

/// The select box every table uses (header and rows): a flat indicator
/// inside the fixed-width checkbox column.
pub fn select_box<'a, M: Clone + 'a>(
    theme: Theme,
    state: CheckState,
    on_press: M,
) -> Element<'a, M> {
    container(
        button(Space::new(Length::Fill, Length::Fill))
            .width(Length::Fixed(INDICATOR_WIDTH))
            .height(Length::Fixed(INDICATOR_HEIGHT))
            .padding(0)
            .style(indicator_style(theme, state))
            .on_press(on_press),
    )
    .width(Length::Fixed(COLUMN_WIDTH))
    .into()
}
