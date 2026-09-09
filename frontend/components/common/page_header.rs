use iced::widget::{button, column, row, text, Space};
use iced::{Alignment, Element, Font, Length};
use crate::frontend::styles::colors::Theme;
use crate::frontend::styles::spacing;
use crate::frontend::styles::typography;
use crate::frontend::styles::headers;
use crate::frontend::styles::button as button_style;

pub fn view<'a, Message: Clone + 'a>(theme: Theme, title: &'a str, description: &'a str, action_label: &'a str, on_action: Message) -> Element<'a, Message> {
    view_with_action_style(theme, title, description, action_label, on_action, false)
}

pub fn view_with_action_style<'a, Message: Clone + 'a>(theme: Theme, title: &'a str, description: &'a str, action_label: &'a str, on_action: Message, secondary_action: bool) -> Element<'a, Message> {
    let titles = column![
        text(title).size(headers::page_title_size()).font(Font { weight: iced::font::Weight::Bold, ..Font::DEFAULT }),
        text(description).size(headers::page_description_size()).color(theme.neutral_front()),
    ]
    .spacing(spacing::XS);

    let action = button(row![text(if secondary_action { "" } else { "+" }).size(typography::emphasis()), text(action_label).size(typography::label())].spacing(spacing::SPACE_6).align_y(Alignment::Center))
        .padding([spacing::SPACE_10, spacing::MD])
        .style(move |iced_theme, status| {
            if secondary_action { button_style::secondary(theme)(iced_theme, status) } else { button_style::primary(theme)(iced_theme, status) }
        })
        .on_press(on_action);

    row![titles, Space::with_width(Length::Fill), action].align_y(Alignment::Start).into()
}

pub fn view_with_switcher<'a, Message: Clone + 'a>(theme: Theme, title: &'a str, description: &'a str, switcher: Element<'a, Message>, action_label: &'a str, on_action: Message) -> Element<'a, Message> {
    view_with_switcher_action_style(theme, title, description, switcher, action_label, on_action, false)
}

pub fn view_with_switcher_action_style<'a, Message: Clone + 'a>(theme: Theme, title: &'a str, description: &'a str, switcher: Element<'a, Message>, action_label: &'a str, on_action: Message, secondary_action: bool) -> Element<'a, Message> {
    let title_row = row![text(title).size(headers::page_title_size()).font(Font { weight: iced::font::Weight::Bold, ..Font::DEFAULT }), switcher].spacing(spacing::SPACE_12).align_y(Alignment::Center);
    let titles = column![title_row, text(description).size(headers::page_description_size()).color(theme.neutral_front())].spacing(spacing::XS);
    let action = button(row![text(if secondary_action { "" } else { "+" }).size(typography::emphasis()), text(action_label).size(typography::label())].spacing(spacing::SPACE_6).align_y(Alignment::Center))
        .padding([spacing::SPACE_10, spacing::MD])
        .style(move |iced_theme, status| {
            if secondary_action { button_style::secondary(theme)(iced_theme, status) } else { button_style::primary(theme)(iced_theme, status) }
        })
        .on_press(on_action);
    row![titles, Space::with_width(Length::Fill), action].align_y(Alignment::Start).into()
}

pub fn page_header_view_with_switcher<'a, Message: Clone + 'a>(theme: Theme, title: &'a str, description: &'a str, switcher: Element<'a, Message>, action_label: &'a str, on_action: Message) -> Element<'a, Message> {
    let title_row = row![text(title).size(headers::page_title_size()).font(Font { weight: iced::font::Weight::Bold, ..Font::DEFAULT }), switcher].spacing(spacing::SPACE_12).align_y(Alignment::Center);

    let titles = column![title_row, text(description).size(headers::page_description_size()).color(theme.neutral_front())].spacing(spacing::XS);

    let action = button(row![text("+").size(typography::emphasis()), text(action_label).size(typography::label())].spacing(spacing::SPACE_6).align_y(Alignment::Center))
        .padding([spacing::SPACE_10, spacing::MD])
        .style(button_style::primary(theme))
        .on_press(on_action);

    row![titles, Space::with_width(Length::Fill), action].align_y(Alignment::Start).into()
}