use iced::widget::button::{Status, Style};
use iced::{Background, Border, Color, Theme as IcedTheme};

use crate::frontend::styles::colors::Theme;
use crate::frontend::styles::{states};

pub fn neutral(theme: Theme) -> impl Fn(&IcedTheme, Status) -> Style {
    move |_iced_theme: &IcedTheme, status: Status| {
        let background = match status {
            Status::Hovered | Status::Pressed => theme.neutral_bg_hover(),
            _ => theme.neutral_bg(),
        };

        Style {
            background: Some(Background::Color(background)),
            text_color: theme.neutral_font(),
            border: Border {
                width: states::STYLES_BORDER_WIDTH_ALPHA,
                color: background,
                radius: states::RADIUS_SM.into(),
                ..Border::default()
            },
            shadow: iced::Shadow::default(),
        }
    }
}

pub fn primary(theme: Theme) -> impl Fn(&IcedTheme, Status) -> Style {
    move |_iced_theme: &IcedTheme, status: Status| {
        let background = match status {
            Status::Hovered | Status::Pressed => theme.primary_hover(),
            _ => 
                theme.primary()
        };
        Style {
            background: Some(Background::Color(background)),
            text_color: theme.accent_front(),
            border: Border {
                width: states::STYLES_BORDER_WIDTH_ALPHA,
                radius: states::RADIUS_SM.into(),
                ..Border::default()
            },
            shadow: iced::Shadow::default(),
        }
    }
}

pub fn secondary(theme: Theme) -> impl Fn(&IcedTheme, Status) -> Style {
    move |_iced_theme: &IcedTheme, status: Status| {
        let background = match status {
            Status::Hovered | Status::Pressed =>theme.secondary_hover(),
            _ => 
                theme.secondary()
        };

        Style {
            background: Some(Background::Color(background)),
            text_color: theme.accent_front(),
            border: Border {
                width: states::STYLES_BORDER_WIDTH_ALPHA,
                radius: states::RADIUS_SM.into(),
                ..Border::default()
            },
            shadow: iced::Shadow::default(),
        }
    }
}

pub fn mode(theme: Theme, active: bool) -> impl Fn(&IcedTheme, Status) -> Style {
    move |_iced_theme: &IcedTheme, status: Status| {
        let (normal, hover) = match active {
            true => (theme.primary(), theme.primary_hover()),
            false => (theme.secondary(), theme.secondary_hover()),
        };
        let background = match status {
            Status::Hovered | Status::Pressed => hover,
            _ => normal,
        };

        Style {
            background: Some(Background::Color(background)),
            text_color: theme.accent_front(),
            border: Border {
                width: states::STYLES_BORDER_WIDTH_ALPHA,
                radius: states::RADIUS_SM.into(),
                ..Border::default()
            },
            shadow: iced::Shadow::default(),
        }
    }
}

pub fn positive(theme: Theme) -> impl Fn(&IcedTheme, Status) -> Style {
    move |_iced_theme: &IcedTheme, status: Status| {
        let background = match status {
            Status::Hovered | Status::Pressed => theme.positive_bg_hover(),
            _ => theme.positive_bg(),
        };

        Style {
            background: Some(Background::Color(background)),
            text_color: theme.accent_front(),
            border: Border {
                width: states::STYLES_BORDER_WIDTH_ALPHA,
                radius: states::RADIUS_SM.into(),
                ..Border::default()
            },
            shadow: iced::Shadow::default(),
        }
    }
}

pub fn negative(theme: Theme) -> impl Fn(&IcedTheme, Status) -> Style {
    move |_iced_theme: &IcedTheme, status: Status| {
        let background = match status {
            Status::Hovered | Status::Pressed => theme.negative_bg_hover(),
            _ => theme.negative_bg(),
        };

        Style {
            background: Some(Background::Color(background)),
            text_color: theme.accent_front(),
            border: Border {
                width: states::STYLES_BORDER_WIDTH_ALPHA,
                radius: states::RADIUS_SM.into(),
                ..Border::default()
            },
            shadow: iced::Shadow::default(),
        }
    }
}

pub fn destructive(theme: Theme) -> impl Fn(&IcedTheme, Status) -> Style {
    move |_iced_theme: &IcedTheme, status: Status| {
        let background = match status {
            Status::Disabled => theme.neutral_bg(),
            Status::Hovered | Status::Pressed => Color { a: states::SOLID_BUTTON_HOVER_ALPHA, ..theme.negative_front() },
            Status::Active => theme.negative_front(),
        };
        let text_color = if matches!(status, Status::Disabled) { theme.neutral_front() } else { theme.accent_front() };
        Style { background: Some(Background::Color(background)), text_color, border: Border { radius: states::RADIUS_SM.into(), ..Border::default() }, shadow: iced::Shadow::default() }
    }
}

pub fn plain(theme: Theme) -> impl Fn(&IcedTheme, Status) -> Style {
    move |_iced_theme: &IcedTheme, status: Status| {
        let background = if matches!(status, Status::Hovered) { Some(Background::Color(theme.neutral_bg())) } else { None };
        Style { background, text_color: theme.neutral_front(), border: Border { radius: states::RADIUS_SM.into(), ..Border::default() }, shadow: iced::Shadow::default() }
    }
}

pub fn outline_primary(theme: Theme) -> impl Fn(&IcedTheme, Status) -> Style {
    move |_iced_theme: &IcedTheme, status: Status| {
        let background = match status {
            Status::Hovered | Status::Pressed => Color { a: states::OUTLINE_BUTTON_HOVER_ALPHA, ..theme.primary() },
            _ => theme.transparent(),
        };
        Style {
            background: Some(Background::Color(background)),
            text_color: theme.primary(),
            border: Border { color: theme.primary(), width: states::STYLES_BORDER_WIDTH_ALPHA, radius: states::RADIUS_SM.into() },
            shadow: iced::Shadow::default(),
        }
    }
}

pub fn nav_item(theme: Theme, active: bool) -> impl Fn(&IcedTheme, Status) -> Style {
    move |_iced_theme: &IcedTheme, status: Status| {
        let background = if active {
            Color { a: states::SELECTED_ALPHA, ..theme.primary() }
        } else if matches!(status, Status::Hovered) {
            Color { a: states::GHOST_ROW_HOVER_ALPHA, ..theme.neutral_front() }
        } else {
            theme.transparent()
        };
        let text_color = if active { theme.primary() } else { theme.neutral_front() };
        Style { background: Some(Background::Color(background)), text_color, border: Border { radius: states::RADIUS_MD.into(), ..Border::default() }, shadow: iced::Shadow::default() }
    }
}

pub fn profile_card(theme: Theme) -> impl Fn(&IcedTheme, Status) -> Style {
    move |_iced_theme: &IcedTheme, status: Status| {
        let background = if matches!(status, Status::Hovered) { Color { a: states::NEUTRAL_CONTROL_HOVER_ALPHA, ..theme.neutral_bg() } } else { theme.neutral_bg() };
        Style { background: Some(Background::Color(background)), text_color: theme.text_primary(), border: Border { radius: states::RADIUS_MD.into(), ..Border::default() }, shadow: iced::Shadow::default() }
    }
}
