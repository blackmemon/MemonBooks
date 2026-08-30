//! Page header (Section 6): a large bold title with a one-line gray
//! description beneath it, and a primary action button aligned to the
//! same row on the right.

use iced::widget::{button, column, row, text, Space};
use iced::{Alignment, Element, Font, Length};

use crate::frontend::styles::colors::Theme;
use crate::frontend::styles::typography;
use crate::frontend::styles::headers;
use crate::frontend::styles::button as button_style;

pub fn view<'a, Message: Clone + 'a>(theme: Theme, title: &'a str, description: &'a str, action_label: &'a str, on_action: Message) -> Element<'a, Message> {
    let titles = column![
        text(title).size(headers::PAGE_TITLE_SIZE).font(Font { weight: iced::font::Weight::Bold, ..Font::DEFAULT }),
        text(description).size(headers::PAGE_DESCRIPTION_SIZE).color(theme.neutral_front()),
    ]
    .spacing(4.0);

    let action = button(row![text("+").size(typography::EMPHASIS), text(action_label).size(typography::LABEL)].spacing(6.0).align_y(Alignment::Center))
        .padding([10.0, 16.0])
        .style(button_style::primary(theme))
        .on_press(on_action);

    row![titles, Space::with_width(Length::Fill), action].align_y(Alignment::Start).into()
}

/// Same as [`view`], except `switcher` renders inline on the title's own
/// row (title text, then switcher, side by side) rather than the title
/// sitting alone -- for a screen like "Maintain" or "Transactions" where
/// a type-switcher dropdown belongs right next to the title, not
/// stacked above or below it.
pub fn view_with_switcher<'a, Message: Clone + 'a>(theme: Theme, title: &'a str, description: &'a str, switcher: Element<'a, Message>, action_label: &'a str, on_action: Message) -> Element<'a, Message> {
    let title_row = row![text(title).size(headers::PAGE_TITLE_SIZE).font(Font { weight: iced::font::Weight::Bold, ..Font::DEFAULT }), switcher].spacing(12.0).align_y(Alignment::Center);

    let titles = column![title_row, text(description).size(headers::PAGE_DESCRIPTION_SIZE).color(theme.neutral_front())].spacing(4.0);

    let action = button(row![text("+").size(typography::EMPHASIS), text(action_label).size(typography::LABEL)].spacing(6.0).align_y(Alignment::Center))
        .padding([10.0, 16.0])
        .style(button_style::primary(theme))
        .on_press(on_action);

    row![titles, Space::with_width(Length::Fill), action].align_y(Alignment::Start).into()
}
