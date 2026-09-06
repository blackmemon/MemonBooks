use iced::widget::pick_list::{Status, Style};
use iced::{Background, Border, Color, Theme as IcedTheme};

use crate::frontend::styles::colors::Theme;
use crate::frontend::styles::{borders, states};

pub fn style(theme: Theme) -> impl Fn(&IcedTheme, Status) -> Style {
    styled(theme, theme.text_primary())
}

pub fn muted(theme: Theme) -> impl Fn(&IcedTheme, Status) -> Style {
    styled(theme, theme.neutral_front())
}

fn styled(theme: Theme, text_color: Color) -> impl Fn(&IcedTheme, Status) -> Style {
    move |_iced_theme: &IcedTheme, status: Status| {
        let background = match status {
            Status::Hovered | Status::Opened => {
                Color {
                    a: states::NEUTRAL_CONTROL_HOVER_ALPHA,
                    ..theme.neutral_bg()
                }
            }

            Status::Active => theme.surface(),
        };

        Style {
            text_color,
            placeholder_color: theme.neutral_front(),
            handle_color: theme.neutral_front(),
            background: Background::Color(background),
            border: borders::card(theme, states::RADIUS_SM),
        }
    }
}