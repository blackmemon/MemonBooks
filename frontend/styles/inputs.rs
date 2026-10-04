use iced::widget::text_editor::{Status as EditStatus, Style as EditStyle};
use iced::widget::text_input::{Status, Style};
use iced::{Background, Border, Color};

use crate::frontend::styles::colors::Theme;
use crate::frontend::styles::states;

pub fn text(theme: Theme) -> impl Fn(&iced::Theme, Status) -> Style {
    move |_iced_theme: &iced::Theme, status: Status| {
        let background = match status {
            Status::Hovered => theme.surface_hover(),
            Status::Focused => theme.surface(),
            Status::Disabled => theme.neutral_bg(),
            _ => theme.surface(),
        };

        let border_color = match status {
            Status::Hovered | Status::Focused => theme.neutral_border_hover(),
            _ => theme.neutral_border(),
        };

        let text_color = match status {
            Status::Disabled => theme.neutral_front(),
            _ => theme.text_primary(),
        };

        Style {
            background: Background::Color(background),

            border: Border {
                color: border_color,
                width: states::TEXTBOX_WIDTH_ALPHA,
                radius: states::RADIUS_SM.into(),
            },

            icon: theme.neutral_front(),
            placeholder: theme.neutral_front(),
            value: text_color,

            selection: Color {
                a: states::TEXTBOX_SELECTION_TINT_ALPHA,
                ..theme.primary()
            },
        }
    }
}

pub fn text_edit(theme: Theme) -> impl Fn(&iced::Theme, EditStatus) -> EditStyle {
    move |_iced_theme: &iced::Theme, status: EditStatus| {
        let background = match status {
            EditStatus::Hovered => theme.surface_hover(),
            EditStatus::Focused => theme.surface(),
            EditStatus::Disabled => theme.neutral_bg(),
            _ => theme.surface(),
        };

        let border_color = match status {
            EditStatus::Hovered | EditStatus::Focused => theme.neutral_border_hover(),
            _ => theme.neutral_border(),
        };

        let text_color = match status {
            EditStatus::Disabled => theme.neutral_front(),
            _ => theme.text_primary(),
        };

        EditStyle {
            background: Background::Color(background),

            border: Border {
                color: border_color,
                width: states::TEXTBOX_WIDTH_ALPHA,
                radius: states::RADIUS_SM.into(),
            },

            icon: theme.neutral_front(),
            placeholder: theme.neutral_front(),
            value: text_color,

            selection: Color {
                a: states::TEXTBOX_SELECTION_TINT_ALPHA,
                ..theme.primary()
            },
        }
    }
}
