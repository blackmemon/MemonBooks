//! Scrollbar placement: an 8 px bar in its own 8 px gutter beside the
//! content (iced reserves `width + spacing` whenever `spacing` is set),
//! so the bar never paints over what it scrolls. Anything aligned with
//! the scrolled content (a table header) must reserve
//! `spacing::SCROLLBAR_WIDTH` on its right.

use iced::widget::scrollable::{Direction, Scrollbar};

use crate::frontend::components::spacing;

pub fn bar() -> Scrollbar {
    Scrollbar::new()
        .width(spacing::SCROLLBAR_WIDTH)
        .scroller_width(spacing::SCROLLBAR_WIDTH)
        .spacing(0.0)
}

pub fn vertical() -> Direction {
    Direction::Vertical(bar())
}

/// Right padding for content inside a vertical [`bar`] scroll area, so
/// the content keeps a clear gap from the bar's own gutter.
pub fn gutter() -> iced::Padding {
    iced::Padding {
        right: spacing::SCROLLBAR_WIDTH,
        ..iced::Padding::ZERO
    }
}
