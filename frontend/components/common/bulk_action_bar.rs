//! Bulk-action bar (Section 6): appears above a DataTable once 1+ rows
//! are selected via its checkbox column -- shows how many, and offers
//! the actions that apply to the whole selection. Callers decide when
//! to include this in their layout (typically `if selected_count > 0`);
//! this component always renders the bar itself.

use iced::widget::{button, row, text, Space};
use iced::{Alignment, Background, Border, Color, Element, Length, Theme};

use crate::frontend::styles::{borders, colors, spacing};

pub fn view<'a, Message: Clone + 'a>(selected_count: usize, on_delete: Message, on_export_csv: Message, on_clear_selection: Message) -> Element<'a, Message> {
    let label = if selected_count == 1 { "1 selected".to_string() } else { format!("{selected_count} selected") };

    let bar = row![
        text(label).size(13).color(colors::WHITE_PRIMARY),
        Space::with_width(Length::Fill),
        button(text("Export CSV").size(12)).style(secondary_button).padding([8.0, 14.0]).on_press(on_export_csv),
        button(text("Delete").size(12)).style(destructive_button).padding([8.0, 14.0]).on_press(on_delete),
        button(text("Clear").size(12)).style(plain_button).padding([8.0, 14.0]).on_press(on_clear_selection),
    ]
    .spacing(spacing::SM)
    .align_y(Alignment::Center)
    .padding([spacing::SM, spacing::MD]);

    iced::widget::container(bar)
        .style(move |_theme: &Theme| iced::widget::container::Style {
            background: Some(Background::Color(Color { a: 0.08, ..colors::WHITE_PRIMARY })),
            border: borders::card(borders::RADIUS_MD),
            ..iced::widget::container::Style::default()
        })
        .width(Length::Fill)
        .into()
}

fn secondary_button(_theme: &Theme, status: iced::widget::button::Status) -> iced::widget::button::Style {
    let background = match status {
        iced::widget::button::Status::Hovered | iced::widget::button::Status::Pressed => Color { a: 0.6, ..colors::WHITE_NEUTRAL_BG },
        _ => Color::WHITE,
    };
    iced::widget::button::Style {
        background: Some(Background::Color(background)),
        text_color: colors::WHITE_NEUTRAL_FRONT,
        border: borders::card(borders::RADIUS_SM),
        shadow: iced::Shadow::default(),
    }
}

fn plain_button(_theme: &Theme, status: iced::widget::button::Status) -> iced::widget::button::Style {
    let background = if matches!(status, iced::widget::button::Status::Hovered) { Some(Background::Color(colors::WHITE_NEUTRAL_BG)) } else { None };
    iced::widget::button::Style { background, text_color: colors::WHITE_NEUTRAL_FRONT, border: Border { radius: borders::RADIUS_SM.into(), ..Border::default() }, shadow: iced::Shadow::default() }
}

fn destructive_button(_theme: &Theme, status: iced::widget::button::Status) -> iced::widget::button::Style {
    let background = match status {
        iced::widget::button::Status::Hovered | iced::widget::button::Status::Pressed => Color { a: 0.85, ..colors::WHITE_NEGATIVE_FRONT },
        _ => colors::WHITE_NEGATIVE_FRONT,
    };
    iced::widget::button::Style {
        background: Some(Background::Color(background)),
        text_color: Color::WHITE,
        border: Border { radius: borders::RADIUS_SM.into(), ..Border::default() },
        shadow: iced::Shadow::default(),
    }
}

