//! Bulk-action bar (Section 6): appears above a DataTable once 1+ rows
//! are selected via its checkbox column -- shows how many, and offers
//! the actions that apply to the whole selection. Callers decide when
//! to include this in their layout (typically `if selected_count > 0`);
//! this component always renders the bar itself.

use iced::widget::{button, row, text, Space};
use iced::{Alignment, Background, Element, Length};

use crate::frontend::styles::button as button_style;
use crate::frontend::styles::colors::Theme;
use crate::frontend::styles::typography;
use crate::frontend::styles::{borders, spacing, states};

pub fn view<'a, Message: Clone + 'a>(
    theme: Theme,
    selected_count: usize,
    on_delete: Message,
    on_export_csv: Message,
    on_clear_selection: Message,
) -> Element<'a, Message> {
    let label = if selected_count == 1 {
        "1 selected".to_string()
    } else {
        format!("{selected_count} selected")
    };

    let bar = row![
        text(label).size(typography::label()).color(theme.primary()),
        Space::with_width(Length::Fill),
        button(text("Export CSV").size(typography::body()))
            .style(button_style::secondary(theme))
            .padding([spacing::SM, spacing::SPACE_14])
            .on_press(on_export_csv),
        button(text("Delete").size(typography::body()))
            .style(button_style::destructive(theme))
            .padding([spacing::SM, spacing::SPACE_14])
            .on_press(on_delete),
        button(text("Clear").size(typography::body()))
            .style(button_style::plain(theme))
            .padding([spacing::SM, spacing::SPACE_14])
            .on_press(on_clear_selection),
    ]
    .spacing(spacing::SM)
    .align_y(Alignment::Center)
    .padding([spacing::SM, spacing::MD]);

    iced::widget::container(bar)
        .style(
            move |_iced_theme: &iced::Theme| iced::widget::container::Style {
                background: Some(Background::Color(
                    theme.primary_tint(states::TINTED_PANEL_ALPHA),
                )),
                border: borders::card(theme, states::RADIUS_MD),
                ..iced::widget::container::Style::default()
            },
        )
        .width(Length::Fill)
        .into()
}
