//! List-view toolbar (Section 6): left side is a row of tabs (active tab
//! bold + underlined in `white_primary`), right side is a search input
//! plus "Filters" and "Sort by" dropdown buttons, all sharing one height.

use iced::widget::{button, column, container, row, text, text_input, Space};
use iced::{Alignment, Element, Font, Length};

use crate::frontend::styles::{spacing, tabs, toolbar as toolbar_style};

#[allow(clippy::too_many_arguments)]
pub fn view<'a, Message: Clone + 'a>(
    tab_labels: &'a [&'a str],
    active_tab: usize,
    on_tab_select: impl Fn(usize) -> Message + 'a,
    search_value: &'a str,
    search_placeholder: &'a str,
    on_search_change: impl Fn(String) -> Message + 'a,
    filters_label: &'a str,
    on_filters_click: Message,
    sort_label: &'a str,
    on_sort_click: Message,
) -> Element<'a, Message> {
    let mut tab_row = row![].spacing(spacing::LG).align_y(Alignment::Center);
    for (index, label) in tab_labels.iter().enumerate() {
        let active = index == active_tab;
        let font = if active { Font { weight: iced::font::Weight::Bold, ..Font::DEFAULT } } else { Font::DEFAULT };
        let tab = column![
            button(text(*label).size(13).color(tabs::text_color(active)).font(font)).style(tabs::tab_button()).padding([6.0, 0.0]).on_press(on_tab_select(index)),
            container(Space::with_height(Length::Fixed(2.0))).width(Length::Fill).style(tabs::underline(active)),
        ]
        .spacing(6.0)
        .width(Length::Shrink);
        tab_row = tab_row.push(tab);
    }

    let search = text_input(search_placeholder, search_value)
        .on_input(on_search_change)
        .padding([8.0, 12.0])
        .size(13)
        .width(Length::Fixed(220.0));

    let filters_button = button(text(filters_label).size(13))
        .padding([8.0, 14.0])
        .style(toolbar_style::dropdown_button())
        .on_press(on_filters_click);

    let sort_button = button(text(sort_label).size(13))
        .padding([8.0, 14.0])
        .style(toolbar_style::dropdown_button())
        .on_press(on_sort_click);

    let controls = row![search, filters_button, sort_button].spacing(spacing::SM).align_y(Alignment::Center).height(Length::Fixed(toolbar_style::CONTROL_HEIGHT));

    row![tab_row, Space::with_width(Length::Fill), controls].align_y(Alignment::Center).into()
}
