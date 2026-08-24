//! Section heading (Section 6): a smaller version of the page header
//! pattern -- bold heading, one-line gray description -- placed between
//! the toolbar and the data table.

use iced::widget::{column, text};
use iced::{Element, Font};

use crate::frontend::styles::{colors, headers};

pub fn view<'a, Message: 'a>(heading: &'a str, description: &'a str) -> Element<'a, Message> {
    column![
        text(heading).size(headers::SECTION_HEADING_SIZE).font(Font { weight: iced::font::Weight::Bold, ..Font::DEFAULT }),
        text(description).size(headers::SECTION_DESCRIPTION_SIZE).color(colors::WHITE_NEUTRAL_FRONT),
    ]
    .spacing(2.0)
    .into()
}
