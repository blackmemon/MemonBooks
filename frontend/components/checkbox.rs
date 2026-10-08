use iced::widget::checkbox::{Status, Style};
use iced::widget::button;
use iced::{Background, Border, Color};

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

/// A button drawn like [`style`]'s checkbox, for the tri-state header
/// box (iced's checkbox has no partial state): filled primary when
/// checked or partial, outlined when empty.
pub fn tri_state(
    theme: Theme,
    state: CheckState,
) -> impl Fn(&iced::Theme, button::Status) -> button::Style {
    move |_iced_theme: &iced::Theme, status: button::Status| {
        let filled = state != CheckState::Unchecked;
        let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
        button::Style {
            background: Some(Background::Color(match (filled, hovered) {
                (true, true) => Color {
                    a: states::SOLID_BUTTON_HOVER_ALPHA,
                    ..theme.primary()
                },
                (true, false) => theme.primary(),
                (false, true) => theme.surface_hover(),
                (false, false) => theme.surface(),
            })),
            text_color: theme.accent_front(),
            border: Border {
                color: if filled {
                    theme.primary()
                } else {
                    theme.neutral_border()
                },
                width: 1.0,
                radius: 2.0.into(),
            },
            shadow: iced::Shadow::default(),
        }
    }
}
