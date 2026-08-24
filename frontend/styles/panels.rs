//! Generic card/panel container style (Section 6): white background,
//! rounded corners, a hairline border and a contained shadow. Used by the
//! data table card, the sidebar's user profile card, and any future
//! panel that wants the same "floating card" treatment.

use iced::widget::container;
use iced::{Background, Color, Theme};

use crate::frontend::styles::{borders, shadows};

pub fn card(radius: f32) -> impl Fn(&Theme) -> container::Style {
    move |_theme: &Theme| container::Style {
        background: Some(Background::Color(Color::WHITE)),
        border: borders::card(radius),
        shadow: shadows::card(),
        ..container::Style::default()
    }
}

/// Same card, no shadow -- for panels nested inside an already-shadowed
/// ancestor (e.g. the profile card at the bottom of the sidebar).
pub fn flat_card(radius: f32) -> impl Fn(&Theme) -> container::Style {
    move |_theme: &Theme| container::Style { background: Some(Background::Color(Color::WHITE)), border: borders::card(radius), ..container::Style::default() }
}
