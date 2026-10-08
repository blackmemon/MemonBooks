//! Section heading (Section 6): a smaller version of the page header
//! pattern -- bold heading, one-line gray description -- placed between
//! the toolbar and the data table.

use iced::widget::{column, text};
use iced::{Element, Font};

use crate::frontend::components::colors::Theme;
use crate::frontend::components::headers;
use crate::frontend::components::spacing;

pub fn view<'a, Message: 'a>(
    theme: Theme,
    heading: &'a str,
    description: &'a str,
) -> Element<'a, Message> {
    column![
        text(heading)
            .size(headers::section_heading_size())
            .font(Font {
                weight: iced::font::Weight::Bold,
                ..Font::DEFAULT
            }),
        text(description)
            .size(headers::section_description_size())
            .color(theme.neutral_front()),
    ]
    .spacing(spacing::SPACE_2)
    .into()
}
