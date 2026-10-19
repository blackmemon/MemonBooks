//! The toolbar search box.

use iced::widget::{container, stack, text_input};
use iced::{Element, Length, Padding};

use crate::frontend::components::colors::Theme;
use crate::frontend::components::{icons, inputs, spacing, typography};

/// Search field with the magnifier glyph laid over its left inset; takes
/// all the width its row leaves over.
pub fn input<'a, M: Clone + 'a>(
    theme: Theme,
    placeholder: &'a str,
    value: &'a str,
    on_input: impl Fn(String) -> M + 'a,
) -> Element<'a, M> {
    let field = text_input(placeholder, value)
        .style(inputs::text(theme))
        .on_input(on_input)
        .padding(Padding {
            top: spacing::SM,
            right: spacing::SPACE_12,
            bottom: spacing::SM,
            left: spacing::SEARCH_ICON_INSET,
        })
        .size(typography::label())
        .width(Length::Fill);
    stack![
        field,
        container(
            icons::icon(icons::SEARCH)
                .size(typography::label())
                .color(theme.neutral_front()),
        )
        .padding([spacing::SPACE_10, spacing::SPACE_12])
    ]
    .width(Length::Fill)
    .into()
}
