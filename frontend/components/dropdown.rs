use std::borrow::Borrow;

use iced::widget::overlay::menu as menu_widget;
use iced::widget::pick_list;
use iced::widget::pick_list::{Status, Style};
use iced::widget::PickList;
use iced::{Background, Color, Element, Length, Theme as IcedTheme};

use crate::frontend::components::colors::Theme;
use crate::frontend::components::{borders, spacing, states, typography};

/// A full-width detail-form dropdown, sized like the form's text inputs.
pub fn form_select<'a, T, L, V, M>(
    theme: Theme,
    options: L,
    selected: Option<V>,
    on_select: impl Fn(T) -> M + 'a,
) -> PickList<'a, T, L, V, M>
where
    T: ToString + PartialEq + Clone + 'a,
    L: Borrow<[T]> + 'a,
    V: Borrow<T> + 'a,
    M: Clone + 'a,
{
    pick_list(options, selected, on_select)
        .style(style(theme))
        .text_size(typography::form_input())
        .padding(spacing::MD)
        .width(Length::Fill)
}

#[derive(Debug, Clone)]
struct CountedOption<Message> {
    label: &'static str,
    count: usize,
    message: Message,
}

impl<Message> PartialEq for CountedOption<Message> {
    fn eq(&self, other: &Self) -> bool {
        self.label == other.label
    }
}

impl<Message> std::fmt::Display for CountedOption<Message> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.label, self.count)
    }
}

pub fn style(theme: Theme) -> impl Fn(&IcedTheme, Status) -> Style {
    styled(theme, theme.text_primary())
}

pub fn counted_filter<Message>(
    theme: Theme,
    placeholder: &'static str,
    options: Vec<(&'static str, usize, Message, bool)>,
) -> Element<'static, Message>
where
    Message: Clone + 'static,
{
    let selected = options.iter().position(|option| option.3);
    let entries: Vec<CountedOption<Message>> = options
        .into_iter()
        .map(|(label, count, message, _)| CountedOption {
            label,
            count,
            message,
        })
        .collect();
    let current = selected.map(|index| entries[index].clone());
    pick_list(entries, current, |option| option.message)
        .placeholder(placeholder)
        .style(style(theme))
        .menu_style(dropdown_menu(theme))
        .into()
}

pub fn dropdown_menu(theme: Theme) -> impl Fn(&IcedTheme) -> menu_widget::Style {
    move |_iced_theme: &IcedTheme| menu_widget::Style {
        background: Background::Color(theme.surface()),
        border: borders::card(theme, states::RADIUS_SM),
        text_color: theme.text_primary(),
        selected_text_color: theme.accent_front(),
        selected_background: Background::Color(theme.primary()),
    }
}

fn styled(theme: Theme, text_color: Color) -> impl Fn(&IcedTheme, Status) -> Style {
    move |_iced_theme: &IcedTheme, status: Status| {
        let border_color = match status {
            Status::Hovered | Status::Opened => theme.neutral_border_hover(),
            _ => theme.neutral_border(),
        };
        let background = match status {
            Status::Hovered | Status::Opened => theme.surface_hover(),
            Status::Active => theme.surface(),
        };
        Style {
            text_color,
            placeholder_color: theme.neutral_front(),
            handle_color: theme.neutral_front(),
            background: Background::Color(background),
            border: iced::Border {
                color: border_color,
                width: states::STYLES_BORDER_WIDTH_ALPHA,
                radius: states::RADIUS_SM.into(),
            },
        }
    }
}
