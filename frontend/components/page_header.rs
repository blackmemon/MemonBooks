use crate::frontend::components::button as button_style;
use crate::frontend::components::colors::Theme;
use crate::frontend::components::fonts;
use crate::frontend::components::headers;
use crate::frontend::components::spacing;
use crate::frontend::components::typography;
use iced::widget::{button, column, row, text, Space};
use iced::{Alignment, Element, Length};

pub fn view<'a, Message: Clone + 'a>(
    theme: Theme,
    title: &'a str,
    description: &'a str,
    action_label: &'a str,
    on_action: Message,
) -> Element<'a, Message> {
    view_with_action_style(theme, title, description, action_label, on_action, false)
}

pub fn view_with_action_style<'a, Message: Clone + 'a>(
    theme: Theme,
    title: &'a str,
    description: &'a str,
    action_label: &'a str,
    on_action: Message,
    secondary_action: bool,
) -> Element<'a, Message> {
    let titles = column![
        text(title)
            .size(headers::page_title_size())
            .font(fonts::BOLD),
        text(description)
            .size(headers::page_description_size())
            .color(theme.neutral_front()),
    ]
    .spacing(spacing::XS);

    let action = button(
        row![
            text(if secondary_action { "" } else { "+" }).size(typography::emphasis()),
            text(action_label).size(typography::label())
        ]
        .spacing(spacing::SPACE_6)
        .align_y(Alignment::Center),
    )
    .padding([spacing::SPACE_10, spacing::MD])
    .style(move |iced_theme, status| {
        if secondary_action {
            button_style::secondary(theme)(iced_theme, status)
        } else {
            button_style::primary(theme)(iced_theme, status)
        }
    })
    .on_press(on_action);

    row![titles, Space::with_width(Length::Fill), action]
        .align_y(Alignment::Start)
        .into()
}

pub fn view_with_switcher<'a, Message: Clone + 'a>(
    theme: Theme,
    title: &'a str,
    description: &'a str,
    switcher: Element<'a, Message>,
    action_label: &'a str,
    on_action: Message,
) -> Element<'a, Message> {
    view_with_switcher_action_style(
        theme,
        title,
        description,
        switcher,
        action_label,
        on_action,
        false,
    )
}

pub fn view_with_switcher_action_style<'a, Message: Clone + 'a>(
    theme: Theme,
    title: &'a str,
    description: &'a str,
    switcher: Element<'a, Message>,
    action_label: &'a str,
    on_action: Message,
    secondary_action: bool,
) -> Element<'a, Message> {
    let title_row = row![
        text(title)
            .size(headers::page_title_size())
            .font(fonts::BOLD),
        switcher
    ]
    .spacing(spacing::SPACE_12)
    .align_y(Alignment::Center);
    let titles = column![
        title_row,
        text(description)
            .size(headers::page_description_size())
            .color(theme.neutral_front())
    ]
    .spacing(spacing::XS);
    let action = button(
        row![
            text(if secondary_action { "" } else { "+" }).size(typography::emphasis()),
            text(action_label).size(typography::label())
        ]
        .spacing(spacing::SPACE_6)
        .align_y(Alignment::Center),
    )
    .padding([spacing::SPACE_10, spacing::MD])
    .style(move |iced_theme, status| {
        if secondary_action {
            button_style::secondary(theme)(iced_theme, status)
        } else {
            button_style::primary(theme)(iced_theme, status)
        }
    })
    .on_press(on_action);
    row![titles, Space::with_width(Length::Fill), action]
        .align_y(Alignment::Start)
        .into()
}

pub fn page_header_view_with_switcher<'a, Message: Clone + 'a>(
    theme: Theme,
    title: &'a str,
    description: &'a str,
    switcher: Element<'a, Message>,
    action_label: &'a str,
    on_action: Message,
) -> Element<'a, Message> {
    let title_row = row![
        text(title)
            .size(headers::page_title_size())
            .font(fonts::BOLD),
        switcher
    ]
    .spacing(spacing::SPACE_12)
    .align_y(Alignment::Center);

    let titles = column![
        title_row,
        text(description)
            .size(headers::page_description_size())
            .color(theme.neutral_front())
    ]
    .spacing(spacing::XS);

    let action = button(
        row![
            text("+").size(typography::emphasis()),
            text(action_label).size(typography::label())
        ]
        .spacing(spacing::SPACE_6)
        .align_y(Alignment::Center),
    )
    .padding([spacing::SPACE_10, spacing::MD])
    .style(button_style::primary(theme))
    .on_press(on_action);

    row![titles, Space::with_width(Length::Fill), action]
        .align_y(Alignment::Start)
        .into()
}

/// The top of a full-page record view: a Back button with the record's
/// actions on the right, then the record's title beside its badges and a
/// muted line under it.
pub fn detail<'a, Message: Clone + 'a>(
    theme: Theme,
    title: &'a str,
    subtitle: String,
    badges: Vec<Element<'a, Message>>,
    on_back: Message,
    actions: Vec<Element<'a, Message>>,
) -> Element<'a, Message> {
    let mut top = row![
        button_style::back(theme, on_back),
        Space::with_width(Length::Fill)
    ]
    .spacing(spacing::SPACE_8)
    .align_y(Alignment::Center);
    for action in actions {
        top = top.push(action);
    }
    let mut title_row = row![text(title)
        .size(headers::page_title_size())
        .font(fonts::BOLD)
        .color(theme.neutral_font())]
    .spacing(spacing::SPACE_10)
    .align_y(Alignment::Center);
    for badge in badges {
        title_row = title_row.push(badge);
    }
    column![
        top,
        title_row,
        text(subtitle)
            .size(headers::page_description_size())
            .color(theme.neutral_front()),
    ]
    .spacing(spacing::SPACE_10)
    .width(Length::Fill)
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_detail_header_builds_with_and_without_badges_and_actions() {
        for theme in [Theme::Light, Theme::Dark] {
            let _: Element<'_, ()> = detail(theme, "Record", "Line".into(), vec![], (), vec![]);
            let _: Element<'_, ()> = detail(
                theme,
                "Record",
                String::new(),
                vec![text("badge").into()],
                (),
                vec![text("edit").into()],
            );
        }
    }
}
