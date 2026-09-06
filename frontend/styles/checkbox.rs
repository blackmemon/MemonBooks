use iced::widget::checkbox::{Status, Style};
use iced::{Background, Border, Color};

use crate::frontend::styles::colors::Theme;
use crate::frontend::styles::states;

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
            (_, true, _) => theme.neutral_bg(),
            (true, false, _) => theme.primary(),
            (false, false, true) => theme.neutral_bg_hover(),
            (false, false, false) => theme.neutral_bg(),
        };

        Style {
            background: Background::Color(background),
            icon_color: Color::TRANSPARENT,
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