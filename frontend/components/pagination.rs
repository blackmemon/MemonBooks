//! Table footer pagination: Previous, numbered pages, Next, the result
//! count and the rows-per-page selector.

use iced::widget::text::Wrapping;
use iced::widget::{button, column, pick_list, row, text, Space};
use iced::{Alignment, Element, Length};

use crate::frontend::components::button as button_style;
use crate::frontend::components::colors::Theme;
use crate::frontend::components::{dropdown, labels, spacing, typography};

/// Everything [`footer`] shows and the messages its controls send.
pub struct Footer<'a, M, P, S> {
    pub theme: Theme,
    pub class: spacing::LayoutClass,
    /// The current page (zero-based) and how many there are.
    pub page: usize,
    pub pages: usize,
    /// Rows on this page and rows across every page.
    pub shown: usize,
    pub total: usize,
    pub page_size: usize,
    pub page_sizes: &'a [usize],
    pub on_previous: M,
    pub on_page: P,
    pub on_page_size: S,
}

/// The footer line under a table. A narrow card stacks the pager and the
/// summary on two lines instead of overflowing.
pub fn footer<'a, M, P, S>(config: Footer<'a, M, P, S>) -> Element<'a, M>
where
    M: Clone + 'a,
    P: Fn(usize) -> M + 'a,
    S: Fn(usize) -> M + 'a,
{
    let Footer {
        theme,
        class,
        page,
        pages,
        shown,
        total,
        page_size,
        page_sizes,
        on_previous,
        on_page,
        on_page_size,
    } = config;
    let mut numbers = row![].spacing(spacing::SPACE_4).align_y(Alignment::Center);
    for item in page_items(page, pages) {
        numbers = numbers.push(match item {
            Some(index) => page_number(theme, index, index == page, on_page(index)),
            None => labels::muted(theme, "...", typography::label()).into(),
        });
    }
    let next = (page + 1 < pages).then(|| on_page(page + 1));
    let pager = row![
        pager_button(theme, "Previous", (page > 0).then_some(on_previous)),
        numbers,
        pager_button(theme, "Next", next),
    ]
    .spacing(spacing::SPACE_12)
    .align_y(Alignment::Center);
    let summary = row![
        labels::muted(theme, results_label(shown, total), typography::label())
            .wrapping(Wrapping::None),
        pick_list(page_sizes, Some(page_size), on_page_size)
            .padding([spacing::SPACE_6, spacing::SPACE_12])
            .text_size(typography::label())
            .style(dropdown::style(theme))
            .menu_style(dropdown::dropdown_menu(theme)),
    ]
    .spacing(spacing::SPACE_12)
    .align_y(Alignment::Center);
    if class == spacing::LayoutClass::Narrow {
        column![pager, summary]
            .spacing(spacing::SPACE_12)
            .padding(spacing::SPACE_20)
            .into()
    } else {
        row![pager, Space::with_width(Length::Fill), summary]
            .align_y(Alignment::Center)
            .padding(spacing::SPACE_20)
            .into()
    }
}

pub fn results_label(shown: usize, total: usize) -> String {
    format!("Showing {shown} of {total} results")
}

/// Page numbers to show: every page when there are few, otherwise the
/// first, last and the neighbours of the current page, with `None`
/// standing for a gap ("...").
pub fn page_items(page: usize, pages: usize) -> Vec<Option<usize>> {
    if pages <= 7 {
        return (0..pages).map(Some).collect();
    }
    let mut items = Vec::new();
    let mut last_pushed = None;
    for index in 0..pages {
        let near = index == 0 || index == pages - 1 || index.abs_diff(page) <= 1;
        if near {
            if let Some(last) = last_pushed {
                if index > last + 1 {
                    items.push(None);
                }
            }
            items.push(Some(index));
            last_pushed = Some(index);
        }
    }
    items
}

/// Outlined Previous / Next button, dimmed when there is nowhere to go.
fn pager_button<'a, M: Clone + 'a>(
    theme: Theme,
    label: &'static str,
    target: Option<M>,
) -> Element<'a, M> {
    button_style::bold(label, [spacing::SPACE_6, spacing::SPACE_14])
        .style(button_style::outlined(theme, false))
        .on_press_maybe(target)
        .into()
}

/// A numbered page button: primary tint box when current, bare otherwise.
fn page_number<'a, M: Clone + 'a>(
    theme: Theme,
    index: usize,
    current: bool,
    on_press: M,
) -> Element<'a, M> {
    button(
        text((index + 1).to_string())
            .size(typography::label())
            .center(),
    )
    .width(Length::Fixed(spacing::PAGE_BUTTON_WIDTH))
    .padding([spacing::SPACE_4, 0.0])
    .style(button_style::outlined_or_plain(theme, current, true))
    .on_press(on_press)
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn few_pages_are_all_listed_and_many_collapse_around_the_current_one() {
        assert_eq!(page_items(1, 3), vec![Some(0), Some(1), Some(2)]);
        assert_eq!(
            page_items(5, 12),
            vec![Some(0), None, Some(4), Some(5), Some(6), None, Some(11)]
        );
    }
}
