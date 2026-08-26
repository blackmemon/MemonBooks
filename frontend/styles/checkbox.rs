//! Canonical `checkbox` style (Section 6). Same situation as
//! `inputs.rs`: the audit found zero styled `checkbox`es anywhere (the
//! 3 in `data_table.rs`, including the Columns panel's, and the 1 in
//! `users_view.rs` all fall back to iced's own `theme.
//! extended_palette()`). A fresh design using this app's tokens --
//! user-confirmed direction, since there was nothing to preserve.
//!
//! A checked checkbox is visually a small solid-primary-fill control,
//! so its hover treatment reuses `states::SOLID_BUTTON_HOVER_ALPHA`
//! (the same "darken a solid fill on hover" constant `button::primary`
//! uses) rather than inventing a new value for something that's
//! already the same kind of thing.

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

        Style { background: Background::Color(background), icon_color, border: Border { color: border_color, width: 1.0, radius: borders::RADIUS_SM.into() }, text_color }
    }
}
