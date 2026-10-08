use iced::widget::{container, row};
use iced::{Background, Element, Length};

use crate::frontend::components::colors::Theme;
use crate::frontend::components::spacing;
use crate::frontend::components::{borders, states};

pub const WIDTH: f32 = 380.0;

pub fn card<'a, Message: Clone + 'a>(
    theme: Theme,
    content: Element<'a, Message>,
) -> Element<'a, Message> {
    container(content)
        .padding(spacing::LG)
        .width(Length::Fixed(WIDTH))
        .height(Length::Fill)
        .style(move |_iced_theme: &iced::Theme| container::Style {
            background: Some(Background::Color(theme.surface())),
            border: borders::card(theme, states::RADIUS_LG),
            ..container::Style::default()
        })
        .into()
}

pub fn split<'a, Message: Clone + 'a>(
    theme: Theme,
    base: Element<'a, Message>,
    panel: Option<Element<'a, Message>>,
) -> Element<'a, Message> {
    let content: Element<'a, Message> = match panel {
        None => base,
        Some(content) => {
            let card = container(card(theme, content)).height(Length::Fill);

            row![base, card].height(Length::Fill).into()
        }
    };

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(iced::Padding {
            top: 0.0,
            right: 0.0,
            bottom: spacing::LG,
            left: 0.0,
        })
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::widget::text;
    #[test]
    fn split_builds_without_panicking_open_or_closed() {
        let theme = Theme::Light;

        let _: Element<'_, ()> = split(theme, text("table").into(), None);
        let _: Element<'_, ()> = split(theme, text("table").into(), Some(text("panel").into()));
    }
}
