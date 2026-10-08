use super::{colors::Theme, fonts, spacing, tabs, typography};
use iced::widget::{Space, button, column, container, row, text};
use iced::{Alignment, Element, Length};

/// A reusable underlined tab strip for form sections.
pub fn tab_bar<'a, Message: Clone + 'a>(
    theme: Theme,
    labels: &'a [&'static str],
    active: usize,
    on_select: impl Fn(usize) -> Message + 'a,
) -> Element<'a, Message> {
    let mut items = row![].spacing(spacing::LG);
    for (index, label) in labels.iter().enumerate() {
        let selected = index == active;
        let mut label_text = text(*label)
            .size(typography::label())
            .color(tabs::text_color(theme, selected));
        if selected {
            label_text = label_text.font(fonts::BOLD);
        }
        items = items.push(
            column![
                button(label_text)
                    .style(tabs::tab_button(theme))
                    .padding([spacing::SPACE_6, spacing::SPACE_4])
                    .on_press(on_select(index)),
                container(Space::with_height(Length::Fixed(2.0)))
                    .width(Length::Fill)
                    .style(tabs::underline(theme, selected)),
            ]
            .spacing(spacing::SPACE_2)
            .width(Length::Shrink),
        );
    }
    items.align_y(Alignment::Center).into()
}
