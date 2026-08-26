//! Right-edge slide-over panel (Section 6): a themed, rounded-border
//! card pinned to the window's right edge over a dimmed, click-to-close
//! backdrop. Generalized from Chart of Accounts' original "Add Account"
//! panel so any view can open one over arbitrary content -- the caller
//! owns the open/closed state and builds the panel's inner content;
//! this only supplies the standard chrome (backdrop, anchoring, card
//! frame) around it.

use iced::widget::{container, mouse_area, opaque, row, stack, Space};
use iced::{Background, Color, Element, Length};

use crate::frontend::styles::borders;
use crate::frontend::styles::colors::Theme;

pub const WIDTH: f32 = 380.0;

/// Layers `content` as a right-anchored slide-over on top of `base`,
/// with a semi-opaque backdrop behind it that fires `on_close` when
/// clicked. Callers decide *whether* to call this at all (based on
/// their own open/closed state) -- there's no `open` flag here, since
/// the caller already branched on it to build `content` in the first
/// place.
pub fn overlay<'a, Message: Clone + 'a>(theme: Theme, base: Element<'a, Message>, content: Element<'a, Message>, on_close: Message) -> Element<'a, Message> {
    let backdrop = mouse_area(
        container(Space::new(Length::Fill, Length::Fill))
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_iced_theme: &iced::Theme| container::Style { background: Some(Background::Color(Color { a: 0.35, ..Color::BLACK })), ..container::Style::default() }),
    )
    .on_press(on_close);

    let panel = container(content)
        .padding(24.0)
        .width(Length::Fixed(WIDTH))
        .height(Length::Fill)
        .style(move |_iced_theme: &iced::Theme| container::Style { background: Some(Background::Color(theme.surface())), border: borders::card(theme, borders::RADIUS_LG), ..container::Style::default() });

    let panel_row = row![Space::with_width(Length::Fill), panel];

    stack(vec![base, backdrop.into(), opaque(panel_row)]).into()
}
