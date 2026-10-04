use crate::frontend::styles::colors::Theme;
use iced::Border;

pub fn card(theme: Theme, radius: f32) -> Border {
    Border {
        color: theme.neutral_bg(),
        width: 1.0,
        radius: radius.into(),
    }
}

pub fn secondary_card(theme: Theme, radius: f32) -> Border {
    Border {
        color: theme.neutral_bg(),
        width: 1.0,
        radius: radius.into(),
    }
}
