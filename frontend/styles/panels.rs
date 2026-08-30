//! Generic card/panel container style (Section 6): white background,
//! rounded corners, a hairline border and a contained shadow. Used by the
//! data table card, the sidebar's user profile card, and any future
//! panel that wants the same "floating card" treatment.

use iced::widget::container;
use iced::{Background, Theme as IcedTheme};

use crate::frontend::styles::colors::Theme;
use crate::frontend::styles::{borders, shadows};

pub fn card(theme: Theme, radius: f32) -> impl Fn(&IcedTheme) -> container::Style {
    move |_iced_theme: &IcedTheme| {
        eprintln!("panels::card closure called"); // TEMP DEBUG
        container::Style {
        // TEMP DEBUG -- bright, impossible-to-miss background + border so
        // we can tell from a screenshot whether this style is reaching the
        // screen at all (even faintly/partially clipped), instead of
        // guessing again. Only caller is data_table.rs's table card
        // (confirmed via grep), so this only affects the one element under
        // diagnosis. Revert both lines back to `theme.surface()` /
        // `borders::card(theme, radius)` once the screenshots are in.
        background: Some(Background::Color(iced::Color::from_rgb8(255, 0, 0))),
        border: iced::Border { color: iced::Color::from_rgb8(255, 0, 255), width: 4.0, radius: radius.into() },
        shadow: shadows::card(theme),
        ..container::Style::default()
        }
    }
}

/// Same card, no shadow -- for panels nested inside an already-shadowed
/// ancestor (e.g. the profile card at the bottom of the sidebar).
pub fn flat_card(theme: Theme, radius: f32) -> impl Fn(&IcedTheme) -> container::Style {
    move |_iced_theme: &IcedTheme| container::Style { background: Some(Background::Color(theme.surface())), border: borders::card(theme, radius), ..container::Style::default() }
}
