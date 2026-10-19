use iced::widget::{button, column, container, pick_list, row, text, text_input, Row, Space};
use iced::{Alignment, Element, Length};

use crate::frontend::components::button as button_style;
use crate::frontend::components::colors::Theme;
use crate::frontend::components::dropdown;
use crate::frontend::components::fonts;
use crate::frontend::components::inputs;
use crate::frontend::components::typography;
use crate::frontend::components::{spacing, tabs};

/// Toolbar row layout constant (Section 6) -- the shared height every
/// control in a list-view toolbar row (search input, buttons,
/// pick_lists) lines up against. The bordered dropdown-button look
/// toolbar controls use is `button::secondary` (Batch 3 of the Section
/// 6 consistency audit consolidated it there).
pub const CONTROL_HEIGHT: f32 = 36.0;

/// [`CONTROL_HEIGHT`] grown with the type scale.
pub fn control_height() -> f32 {
    spacing::grown(CONTROL_HEIGHT)
}

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
        let font = if active { fonts::BOLD } else { fonts::DEFAULT };
        let tab = column![
            button(
                text(*label)
                    .size(typography::label())
                    .color(tabs::text_color(theme, active))
                    .font(font)
            )
            .style(tabs::tab_button(theme))
            .padding([spacing::SPACE_6, 0.0])
            .on_press(on_tab_select(index)),
            container(Space::with_height(Length::Fixed(2.0)))
                .width(Length::Fill)
                .style(tabs::underline(theme, active)),
        ]
        .spacing(spacing::SPACE_6)
        .width(Length::Shrink);
        tab_row = tab_row.push(tab);
    }

    let search = text_input(search_placeholder, search_value)
        .style(inputs::text(theme))
        .on_input(on_search_change)
        .padding([spacing::SM, spacing::SPACE_12])
        .size(typography::label())
        .width(Length::Fixed(220.0));

    let filters_button = button(text(filters_label).size(typography::label()))
        .padding([spacing::SM, spacing::SPACE_14])
        .style(button_style::secondary(theme))
        .on_press(on_filters_click);

    let sort_button = button(text(sort_label).size(typography::label()))
        .padding([spacing::SM, spacing::SPACE_14])
        .style(button_style::secondary(theme))
        .on_press(on_sort_click);

    let controls = row![search, filters_button, sort_button]
        .spacing(spacing::SM)
        .align_y(Alignment::Center)
        .height(Length::Fixed(control_height()));

    row![tab_row, Space::with_width(Length::Fill), controls]
        .align_y(Alignment::Center)
        .into()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StatusFilter {
    #[default]
    All,
    Active,
    Inactive,
}

pub const STATUS_FILTERS: [StatusFilter; 3] = [
    StatusFilter::All,
    StatusFilter::Active,
    StatusFilter::Inactive,
];

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
    let search = text_input(search_placeholder, search_value)
        .style(inputs::text(theme))
        .on_input(on_search_change)
        .padding([spacing::SM, spacing::SPACE_12])
        .size(typography::label())
        .width(Length::Fill);
    let status = pick_list(STATUS_FILTERS, Some(status_selected), on_status_select)
        .padding([spacing::SM, spacing::SPACE_12])
        .text_size(typography::label())
        .style(dropdown::style(theme))
        .menu_style(dropdown::dropdown_menu(theme));
    let reset_button = button(
        text("Reset All")
            .size(typography::label())
            .font(fonts::BOLD),
    )
    .padding([spacing::SM, spacing::SPACE_14])
    .style(button_style::neutral(theme))
    .on_press(on_reset_all);
    let columns_button = button(text("Columns").size(typography::label()).font(fonts::BOLD))
        .padding([spacing::SM, spacing::SPACE_14])
        .style(button_style::neutral(theme))
        .on_press(on_toggle_columns);
    let export_button = button(text("Export").size(typography::label()).font(fonts::BOLD))
        .padding([spacing::SM, spacing::SPACE_14])
        .style(button_style::neutral(theme))
        .on_press(on_export);
    let delete_button = button(text("Delete").size(typography::label()))
        .padding([spacing::SM, spacing::SPACE_14])
        .style(button_style::negative(theme))
        .on_press_maybe(delete_enabled.then_some(on_delete));

    row![
        search,
        status,
        reset_button,
        columns_button,
        export_button,
        delete_button
    ]
    .spacing(spacing::SM)
    .align_y(Alignment::Center)
    .height(Length::Fixed(control_height()))
    .width(Length::Fill)
    .into()
}

/// A table card's top block, two rows `SPACE_20` apart. Row 1 is the
/// title block with the Add button on the right; row 2 is the toolbar:
/// search (taking the spare width), the filter dropdowns, then the tool
/// buttons. A narrow card wraps the toolbar: search on its own line,
/// filters and tools below it.
pub fn card_header<'a, M: 'a>(
    class: spacing::LayoutClass,
    title: Element<'a, M>,
    search: Element<'a, M>,
    filters: Element<'a, M>,
    tools: Element<'a, M>,
    add: Element<'a, M>,
) -> Element<'a, M> {
    let page_header = line(vec![
        container(title).width(Length::Fill).clip(true).into(),
        add,
    ]);
    column![page_header, filter_bar(class, search, filters, tools)]
        .spacing(spacing::SPACE_20)
        .padding(spacing::SPACE_20)
        .into()
}

/// The toolbar row of [`card_header`] on its own, for a page whose title
/// row sits elsewhere: search (taking the spare width), the filter
/// dropdowns, then the tool buttons; when narrow, search on its own line
/// with filters and tools below it.
pub fn filter_bar<'a, M: 'a>(
    class: spacing::LayoutClass,
    search: Element<'a, M>,
    filters: Element<'a, M>,
    tools: Element<'a, M>,
) -> Element<'a, M> {
    if class == spacing::LayoutClass::Narrow {
        column![search, line(vec![filters, tools])]
            .spacing(spacing::SPACE_12)
            .into()
    } else {
        line(vec![search, filters, tools]).into()
    }
}

fn line<'a, M: 'a>(items: Vec<Element<'a, M>>) -> Row<'a, M> {
    Row::with_children(items)
        .spacing(spacing::SPACE_12)
        .align_y(Alignment::Center)
}
