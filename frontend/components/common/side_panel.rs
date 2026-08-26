//! Right-side detail panel (Section 6): a themed, rounded-border card
//! taking its own fixed-width column at the right edge of the content
//! area, with the main content narrowing to make real room for it --
//! a side-by-side split, not a floating overlay. Generalized from Chart
//! of Accounts' original "Add Account" panel so any view can open one
//! over arbitrary content; the caller owns the open/closed state and
//! builds the panel's inner content (including its own way to close
//! it -- a Cancel button, re-clicking the control that opened it,
//! whatever fits), this only supplies the split layout and card chrome
//! around it.

use iced::widget::{container, row};
use iced::{Background, Element, Length};

use crate::frontend::styles::borders;
use crate::frontend::styles::colors::Theme;

pub const WIDTH: f32 = 380.0;

/// Lays `base` and `panel` out side by side when `panel` is `Some` --
/// `base` (already `width(Length::Fill)` in every caller) narrows to
/// fill whatever's left after the panel's fixed width, so both render
/// normally in their own space, nothing on top of anything. Returns
/// `base` alone, full width, when `panel` is `None`.
pub fn split<'a, Message: Clone + 'a>(theme: Theme, base: Element<'a, Message>, panel: Option<Element<'a, Message>>) -> Element<'a, Message> {
    let Some(content) = panel else {
        return base;
    };

    let card = container(content)
        .padding(24.0)
        .width(Length::Fixed(WIDTH))
        .height(Length::Fill)
        .style(move |_iced_theme: &iced::Theme| container::Style { background: Some(Background::Color(theme.surface())), border: borders::card(theme, borders::RADIUS_LG), ..container::Style::default() });

    row![base, card].height(Length::Fill).into()
}
