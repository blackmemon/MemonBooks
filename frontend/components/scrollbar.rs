//! Scrollbar placement: an 8 px bar in its own 8 px gutter beside the
//! content (iced reserves `width + spacing` whenever `spacing` is set),
//! so the bar never paints over what it scrolls. Anything aligned with
//! the scrolled content (a table header) must reserve
//! `spacing::SCROLLBAR_WIDTH` on its right.

use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::slider::Rail;
use iced::widget::vertical_slider::{self, Handle, HandleShape, Status};
use iced::{border, Background, Border, Theme as IcedTheme};

use crate::frontend::components::colors::Theme;
use crate::frontend::components::table;

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

/// The bar beside a [`row_snap`](crate::frontend::components::row_snap)
/// grid: a slim rail with a thumb that grows darker when hovered or
/// dragged.
pub fn row_track(theme: Theme) -> impl Fn(&IcedTheme, Status) -> vertical_slider::Style {
    move |_iced_theme: &IcedTheme, status: Status| {
        let thumb = match status {
            Status::Active => table::scrollbar_thumb(theme),
            Status::Hovered | Status::Dragged => table::scrollbar_thumb_hover(theme),
        };
        let track = Background::Color(table::scrollbar_track(theme));
        vertical_slider::Style {
            rail: Rail {
                backgrounds: (track, track),
                width: spacing::SCROLLBAR_WIDTH,
                border: Border {
                    radius: border::radius(spacing::SCROLLBAR_WIDTH),
                    ..Border::default()
                },
            },
            handle: Handle {
                shape: HandleShape::Rectangle {
                    width: 40,
                    border_radius: border::radius(spacing::SCROLLBAR_WIDTH),
                },
                background: Background::Color(thumb),
                border_width: 0.0,
                border_color: theme.transparent(),
            },
        }
    }
}
