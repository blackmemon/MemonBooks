use iced::widget::checkbox::{Status, Style};
use iced::{Background, Border, Color};

use crate::frontend::styles::colors::Theme;
use crate::frontend::styles::{borders, states};

pub fn style(theme: Theme) -> impl Fn(&iced::Theme, Status) -> Style {
    move |_iced_theme: &iced::Theme, status: Status| {
        let is_checked = matches!(status, Status::Active { is_checked: true } | Status::Hovered { is_checked: true } | Status::Disabled { is_checked: true });
        let is_disabled = matches!(status, Status::Disabled { .. });
        let is_hovered = matches!(status, Status::Hovered { .. });

        let background = if is_disabled {
            theme.neutral_bg()
        } else if is_checked {
            let color = theme.primary();
            if is_hovered { Color { a: states::SOLID_BUTTON_HOVER_ALPHA, ..color } } else { color }
        } else {
            theme.surface()
        };

        let border_color = if is_disabled { theme.neutral_bg() } else if is_checked { theme.primary() } else { theme.neutral_bg() };
        let icon_color = if is_disabled { theme.neutral_front() } else { Color::WHITE };
        let text_color = if is_disabled { Some(theme.neutral_front()) } else { None };

        Style { background: Background::Color(background), icon_color, border: Border { color: border_color, width: 1.0, radius: states::RADIUS_SM.into() }, text_color }
    }
}
