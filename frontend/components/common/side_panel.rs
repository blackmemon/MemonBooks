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

/// Bottom margin between the content area (table, and the panel when
/// open) and the window's bottom edge -- matches the `24.0` each view
/// already uses on the left/right/top of that same area, so all four
/// sides are consistent instead of just three.
const BOTTOM_MARGIN: f32 = 24.0;

/// Lays `base` and `panel` out side by side when `panel` is `Some` --
/// `base` (already `width(Length::Fill)` in every caller) narrows to
/// fill whatever's left after the panel's fixed width, so both render
/// normally in their own space, nothing on top of anything. Returns
/// `base` alone, full width, when `panel` is `None`. Either way, the
/// result gets the same bottom margin -- applied here, once, so it
/// can't end up present in one branch and missing in the other.
pub fn split<'a, Message: Clone + 'a>(theme: Theme, base: Element<'a, Message>, panel: Option<Element<'a, Message>>) -> Element<'a, Message> {
    let content: Element<'a, Message> = match panel {
        None => base,
        Some(content) => {
            let card = container(content)
                .padding(24.0)
                .width(Length::Fixed(WIDTH))
                .height(Length::Fill)
                .style(move |_iced_theme: &iced::Theme| container::Style { background: Some(Background::Color(theme.surface())), border: borders::card(theme, borders::RADIUS_LG), ..container::Style::default() });

            row![base, card].height(Length::Fill).into()
        }
    };

    container(content).width(Length::Fill).height(Length::Fill).padding(iced::Padding { top: 0.0, right: 0.0, bottom: BOTTOM_MARGIN, left: 0.0 }).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::widget::text;

    /// Building the widget tree exercises every constructor `split`
    /// calls -- both branches, since `None` and `Some` go through
    /// different code paths.
    #[test]
    fn split_builds_without_panicking_open_or_closed() {
        let theme = Theme::Light;

        let _: Element<'_, ()> = split(theme, text("table").into(), None);
        let _: Element<'_, ()> = split(theme, text("table").into(), Some(text("panel").into()));
    }
}
