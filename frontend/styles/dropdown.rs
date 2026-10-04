use iced::widget::overlay::menu as menu_widget;
use iced::widget::pick_list::{Status, Style};
use iced::{Background, Color, Theme as IcedTheme};

use crate::frontend::styles::colors::Theme;
use crate::frontend::styles::{borders, states};

pub fn style(theme: Theme) -> impl Fn(&IcedTheme, Status) -> Style {
    styled(theme, theme.text_primary())
}

pub fn dropdown_menu(theme: Theme) -> impl Fn(&IcedTheme) -> menu_widget::Style {
    move |_iced_theme: &IcedTheme| menu_widget::Style {
        background: Background::Color(theme.surface()),
        border: borders::card(theme, states::RADIUS_SM),
        text_color: theme.text_primary(),
        selected_text_color: theme.accent_front(),
        selected_background: Background::Color(theme.primary()),
    }
}

fn styled(theme: Theme, text_color: Color) -> impl Fn(&IcedTheme, Status) -> Style {
    move |_iced_theme: &IcedTheme, status: Status| {
        let border_color = match status {
            Status::Hovered | Status::Opened => theme.neutral_border_hover(),
            _ => theme.neutral_border(),
        };
        let background = match status {
            Status::Hovered | Status::Opened => theme.surface_hover(),
            Status::Active => theme.surface(),
        };
        Style {
            text_color,
            placeholder_color: theme.neutral_front(),
            handle_color: theme.neutral_front(),
            background: Background::Color(background),
            border: iced::Border {
                color: border_color,
                width: states::STYLES_BORDER_WIDTH_ALPHA,
                radius: states::RADIUS_SM.into(),
            },
        }
    }
}
