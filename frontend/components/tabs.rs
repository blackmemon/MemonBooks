//! Toolbar tab styling (Section 6): active tab is bold text plus a
//! bottom-border underline in `white_primary`; inactive tabs are plain
//! gray text with no border.

use iced::widget::{button, column, container, row, text, Space};
use iced::{Alignment, Background, Border, Color, Element, Length, Theme as IcedTheme};

use crate::frontend::components::colors::Theme;
use crate::frontend::components::{fonts, icons, spacing, typography};

/// A tab strip whose tabs each lead with an icon; the active tab is bold
/// in the primary color over an underline. `items` are `(glyph, label)`.
pub fn icon_bar<'a, Message: Clone + 'a>(
    theme: Theme,
    items: &'a [(&'static str, &'static str)],
    active: usize,
    on_select: impl Fn(usize) -> Message + 'a,
) -> Element<'a, Message> {
    let mut strip = row![].spacing(spacing::SPACE_8).align_y(Alignment::Center);
    for (index, (glyph, label)) in items.iter().enumerate() {
        let selected = index == active;
        let color = text_color(theme, selected);
        let mut label_text = text(*label).size(typography::label()).color(color);
        if selected {
            label_text = label_text.font(fonts::BOLD);
        }
        let content = row![
            icons::icon(glyph).size(typography::label()).color(color),
            label_text
        ]
        .spacing(spacing::SPACE_6)
        .align_y(Alignment::Center);
        strip = strip.push(
            column![
                button(content)
                    .style(tab_button(theme))
                    .padding([spacing::SPACE_6, spacing::SPACE_10])
                    .on_press(on_select(index)),
                container(Space::with_height(Length::Fixed(
                    spacing::TAB_UNDERLINE_HEIGHT
                )))
                .width(Length::Fill)
                .style(underline(theme, selected)),
            ]
            .spacing(spacing::SPACE_2)
            .width(Length::Shrink),
        );
    }
    strip.into()
}

pub fn text_color(theme: Theme, active: bool) -> Color {
    if active {
        theme.primary()
    } else {
        theme.neutral_front()
    }
}

pub fn underline(
    theme: Theme,
    active: bool,
) -> impl Fn(&IcedTheme) -> iced::widget::container::Style {
    move |_iced_theme: &IcedTheme| iced::widget::container::Style {
        background: Some(Background::Color(if active {
            theme.primary()
        } else {
            theme.transparent()
        })),
        ..iced::widget::container::Style::default()
    }
}

/// The tab is a plain (borderless, backgroundless) button so its label
/// and underline can be styled directly -- this only strips the default
/// button chrome.
pub fn tab_button(theme: Theme) -> impl Fn(&IcedTheme, button::Status) -> button::Style {
    move |_iced_theme: &IcedTheme, _status: button::Status| button::Style {
        background: None,
        text_color: theme.text_primary(),
        border: Border::default(),
        shadow: iced::Shadow::default(),
    }
}
