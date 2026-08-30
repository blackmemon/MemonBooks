//! Bulk-action bar (Section 6): appears above a DataTable once 1+ rows
//! are selected via its checkbox column -- shows how many, and offers
//! the actions that apply to the whole selection. Callers decide when
//! to include this in their layout (typically `if selected_count > 0`);
//! this component always renders the bar itself.

use iced::widget::{button, row, text, Space};
use iced::{Alignment, Background, Color, Element, Length};

use crate::frontend::styles::colors::Theme;
use crate::frontend::styles::{borders, spacing, states};
use crate::frontend::styles::button as button_style;

pub fn view<'a, Message: Clone + 'a>(theme: Theme, selected_count: usize, on_delete: Message, on_export_csv: Message, on_clear_selection: Message) -> Element<'a, Message> {
    let label = if selected_count == 1 { "1 selected".to_string() } else { format!("{selected_count} selected") };

    let bar = row![
        text(label).size(13).color(theme.primary()),
        Space::with_width(Length::Fill),
        button(text("Export CSV").size(12)).style(button_style::secondary(theme)).padding([8.0, 14.0]).on_press(on_export_csv),
        button(text("Delete").size(12)).style(button_style::destructive(theme)).padding([8.0, 14.0]).on_press(on_delete),
        button(text("Clear").size(12)).style(button_style::plain(theme)).padding([8.0, 14.0]).on_press(on_clear_selection),
    ]
    .spacing(spacing::SM)
    .align_y(Alignment::Center)
    .padding([spacing::SM, spacing::MD]);

    iced::widget::container(bar)
        .style(move |_iced_theme: &iced::Theme| iced::widget::container::Style {
            background: Some(Background::Color(Color { a: states::TINTED_PANEL_ALPHA, ..theme.primary() })),
            border: borders::card(theme, borders::RADIUS_MD),
            ..iced::widget::container::Style::default()
        })
        .width(Length::Fill)
        .into()
}

