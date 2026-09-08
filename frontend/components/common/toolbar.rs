use iced::widget::{button, column, container, pick_list, row, text, text_input, Space};
use iced::{Alignment, Element, Font, Length};

use crate::frontend::styles::colors::Theme;
use crate::frontend::styles::typography;
use crate::frontend::styles::fonts;
use crate::frontend::styles::{spacing, tabs, toolbar as toolbar_style};
use crate::frontend::styles::button as button_style;
use crate::frontend::styles::dropdown;
use crate::frontend::styles::inputs;

#[allow(clippy::too_many_arguments)]
pub fn view<'a, Message: Clone + 'a>(
    theme: Theme,
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
            button(text(*label).size(typography::LABEL).color(tabs::text_color(theme, active)).font(font)).style(tabs::tab_button(theme)).padding([spacing::SPACE_6, 0.0]).on_press(on_tab_select(index)),
            container(Space::with_height(Length::Fixed(2.0))).width(Length::Fill).style(tabs::underline(theme, active)),
        ]
        .spacing(spacing::SPACE_6)
        .width(Length::Shrink);
        tab_row = tab_row.push(tab);
    }

    let search = text_input(search_placeholder, search_value)
        .style(inputs::text(theme))
        .on_input(on_search_change)
        .padding([spacing::SM, spacing::SPACE_12])
        .size(typography::LABEL)
        .width(Length::Fixed(220.0));

    let filters_button = button(text(filters_label).size(typography::LABEL))
        .padding([spacing::SM, spacing::SPACE_14])
        .style(button_style::secondary(theme))
        .on_press(on_filters_click);

    let sort_button = button(text(sort_label).size(typography::LABEL))
        .padding([spacing::SM, spacing::SPACE_14])
        .style(button_style::secondary(theme))
        .on_press(on_sort_click);

    let controls = row![search, filters_button, sort_button].spacing(spacing::SM).align_y(Alignment::Center).height(Length::Fixed(toolbar_style::CONTROL_HEIGHT));

    row![tab_row, Space::with_width(Length::Fill), controls].align_y(Alignment::Center).into()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StatusFilter {
    #[default]
    All,
    Active,
    Inactive,
}

pub const STATUS_FILTERS: [StatusFilter; 3] = [StatusFilter::All, StatusFilter::Active, StatusFilter::Inactive];

impl StatusFilter {
    /// Whether a record with this `is_active` flag passes the filter.
    pub fn matches(self, is_active: bool) -> bool {
        match self {
            StatusFilter::All => true,
            StatusFilter::Active => is_active,
            StatusFilter::Inactive => !is_active,
        }
    }
}

impl std::fmt::Display for StatusFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            StatusFilter::All => "All",
            StatusFilter::Active => "Active",
            StatusFilter::Inactive => "Inactive",
        })
    }
}

#[allow(clippy::too_many_arguments)]
pub fn list_toolbar<'a, Message: Clone + 'a>(
    theme: Theme,
    search_value: &'a str,
    search_placeholder: &'a str,
    on_search_change: impl Fn(String) -> Message + 'a,
    status_selected: StatusFilter,
    on_status_select: impl Fn(StatusFilter) -> Message + 'a,
    on_reset_all: Message,
    on_toggle_columns: Message,
    on_export: Message,
    delete_enabled: bool,
    on_delete: Message,
) -> Element<'a, Message> {
    let search = text_input(search_placeholder, search_value).style(inputs::text(theme)).on_input(on_search_change).padding([spacing::SM, spacing::SPACE_12]).size(typography::LABEL).width(Length::Fill);
    let status = pick_list(STATUS_FILTERS, Some(status_selected), on_status_select).padding([spacing::SM, spacing::SPACE_12]).text_size(typography::LABEL).style(dropdown::muted(theme));
    let reset_button = button(text("Reset All").size(typography::LABEL).font(fonts::BOLD)).padding([spacing::SM, spacing::SPACE_14]).style(button_style::neutral(theme)).on_press(on_reset_all);
    let columns_button = button(text("Columns").size(typography::LABEL).font(fonts::BOLD)).padding([spacing::SM, spacing::SPACE_14]).style(button_style::neutral(theme)).on_press(on_toggle_columns);
    let export_button = button(text("Export").size(typography::LABEL).font(fonts::BOLD)).padding([spacing::SM, spacing::SPACE_14]).style(button_style::neutral(theme)).on_press(on_export);
    let delete_button = button(text("Delete").size(typography::LABEL)).padding([spacing::SM, spacing::SPACE_14]).style(button_style::negative(theme)).on_press_maybe(delete_enabled.then_some(on_delete));

    row![search, status, reset_button, columns_button, export_button, delete_button].spacing(spacing::SM).align_y(Alignment::Center).height(Length::Fixed(toolbar_style::CONTROL_HEIGHT)).width(Length::Fill).into()
}