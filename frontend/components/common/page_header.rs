//! Page header (Section 6): a large bold title with a one-line gray
//! description beneath it, and a primary action button aligned to the
//! same row on the right.

use iced::widget::{button, column, row, text, Space};
use iced::{Alignment, Background, Border, Color, Element, Font, Length, Theme};

use crate::frontend::styles::{borders, colors, headers};

pub fn view<'a, Message: Clone + 'a>(title: &'a str, description: &'a str, action_label: &'a str, on_action: Message) -> Element<'a, Message> {
    let titles = column![
        text(title).size(headers::PAGE_TITLE_SIZE).font(Font { weight: iced::font::Weight::Bold, ..Font::DEFAULT }),
        text(description).size(headers::PAGE_DESCRIPTION_SIZE).color(colors::WHITE_NEUTRAL_FRONT),
    ]
    .spacing(4.0);

    let action = button(row![text("+").size(14), text(action_label).size(13)].spacing(6.0).align_y(Alignment::Center))
        .padding([10.0, 16.0])
        .style(action_button)
        .on_press(on_action);

    row![titles, Space::with_width(Length::Fill), action].align_y(Alignment::Start).into()
}

fn action_button(_theme: &Theme, status: iced::widget::button::Status) -> iced::widget::button::Style {
    let background = match status {
        iced::widget::button::Status::Hovered | iced::widget::button::Status::Pressed => Color { a: 0.85, ..colors::WHITE_PRIMARY },
        _ => colors::WHITE_PRIMARY,
    };
    iced::widget::button::Style {
        background: Some(Background::Color(background)),
        text_color: Color::WHITE,
        border: Border { radius: borders::RADIUS_SM.into(), ..Border::default() },
        shadow: iced::Shadow::default(),
    }
}
