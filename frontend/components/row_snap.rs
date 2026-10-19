//! A card grid that scrolls by whole rows.
//!
//! Instead of a pixel scroll area (which can stop with a card cut at the
//! top or bottom), only the rows that fit completely are built: the
//! window starts at [`RowSnap::first_row`] and holds as many fixed-height
//! rows as the available height allows (at least one). The mouse wheel,
//! a trackpad and the scrollbar all move that window a whole row at a
//! time, so snapping is exact by construction and never fights a drag.
//!
//! The page keeps one [`RowSnap`] in its state, forwards [`Scroll`]
//! messages to [`RowSnap::scroll`], and draws the grid with [`view`] in a
//! region of bounded height (a `responsive` fill between the toolbar and
//! the footer).

use std::cell::Cell;

use iced::mouse::ScrollDelta;
use iced::widget::{column, container, mouse_area, row, vertical_slider, Space};
use iced::{Element, Length};

use crate::frontend::components::colors::Theme;
use crate::frontend::components::{scrollbar, spacing};

/// Pixels of trackpad travel that count as one wheel notch.
const PIXELS_PER_STEP: f32 = 40.0;

/// A scroll request: wheel travel, or a jump to a row from the bar.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Scroll {
    Lines(f32),
    Pixels(f32),
    ToRow(usize),
}

impl From<ScrollDelta> for Scroll {
    fn from(delta: ScrollDelta) -> Self {
        match delta {
            ScrollDelta::Lines { y, .. } => Scroll::Lines(y),
            ScrollDelta::Pixels { y, .. } => Scroll::Pixels(y),
        }
    }
}

/// The window's position, plus the geometry the last view measured so an
/// update can clamp without knowing the window size.
#[derive(Debug, Clone, Default)]
pub struct RowSnap {
    first_row: usize,
    pixels: f32,
    total_rows: Cell<usize>,
    visible_rows: Cell<usize>,
}

impl RowSnap {
    pub fn first_row(&self) -> usize {
        self.first_row
    }

    /// Back to the first row, e.g. after paging or filtering.
    pub fn reset(&mut self) {
        self.first_row = 0;
        self.pixels = 0.0;
    }

    pub fn scroll(&mut self, scroll: Scroll) {
        let steps = match scroll {
            Scroll::ToRow(row) => {
                self.pixels = 0.0;
                self.first_row = row.min(self.last_first_row());
                return;
            }
            Scroll::Lines(y) => whole_steps(y),
            Scroll::Pixels(y) => {
                self.pixels += y;
                let steps = if self.pixels.abs() >= PIXELS_PER_STEP {
                    whole_steps(self.pixels)
                } else {
                    0
                };
                if steps != 0 {
                    self.pixels = 0.0;
                }
                steps
            }
        };
        // Wheel up (positive) moves toward the first row.
        let target = self.first_row as i64 - steps;
        self.first_row = target.clamp(0, self.last_first_row() as i64) as usize;
    }

    fn last_first_row(&self) -> usize {
        last_first_row(self.total_rows.get(), self.visible_rows.get())
    }
}

/// The direction of `y` as one step: at most one row per event.
fn whole_steps(y: f32) -> i64 {
    if y > 0.0 {
        1
    } else if y < 0.0 {
        -1
    } else {
        0
    }
}

/// How many whole rows of `row_height`, `gap` apart, fit in `available`;
/// never fewer than one.
pub fn visible_rows(available: f32, row_height: f32, gap: f32) -> usize {
    if !available.is_finite() || available <= 0.0 {
        return 1;
    }
    (((available + gap) / (row_height + gap)).floor() as usize).max(1)
}

/// The furthest the window can start while still showing full rows.
pub fn last_first_row(total_rows: usize, visible_rows: usize) -> usize {
    total_rows.saturating_sub(visible_rows.max(1))
}

/// The card grid for `cells` laid `columns` to a row, in a region
/// `available` pixels tall, scrolled to `state`'s first row. Each row is
/// `spacing::card_height()` tall; cells should fill that height.
pub fn view<'a, M: Clone + 'a>(
    state: &RowSnap,
    theme: Theme,
    cells: Vec<Element<'a, M>>,
    columns: usize,
    available: f32,
    on_scroll: impl Fn(Scroll) -> M + Copy + 'a,
) -> Element<'a, M> {
    let columns = columns.max(1);
    let row_height = spacing::card_height();
    let gap = spacing::ROW_GAP;
    let total_rows = cells.len().div_ceil(columns);
    let visible = visible_rows(available, row_height, gap);
    let last = last_first_row(total_rows, visible);
    state.total_rows.set(total_rows);
    state.visible_rows.set(visible);
    let first = state.first_row.min(last);

    let mut window = column![].spacing(gap).width(Length::Fill);
    let mut remaining = cells.into_iter().skip(first * columns);
    for _ in 0..visible {
        let mut line = row![].spacing(spacing::SM).width(Length::Fill);
        let mut placed = 0;
        for cell in remaining.by_ref().take(columns) {
            line = line.push(cell);
            placed += 1;
        }
        if placed == 0 {
            break;
        }
        for _ in placed..columns {
            line = line.push(Space::with_width(Length::FillPortion(1)));
        }
        window = window.push(container(line).height(Length::Fixed(row_height)));
    }

    let shown = visible.min(total_rows - first.min(total_rows));
    let height = shown as f32 * (row_height + gap) - gap;
    let grid = mouse_area(container(window).padding(scrollbar::gutter()))
        .on_scroll(move |delta| on_scroll(delta.into()));

    let mut area = row![container(grid).width(Length::Fill)].height(Length::Fixed(height.max(0.0)));
    if last > 0 {
        // The slider's top is its maximum, so it runs from `last` down.
        let bar = vertical_slider(0..=last as u32, (last - first) as u32, move |value| {
            on_scroll(Scroll::ToRow(last - value as usize))
        })
        .step(1u32)
        .width(spacing::SCROLLBAR_WIDTH)
        .height(Length::Fill)
        .style(scrollbar::row_track(theme));
        area = area.push(bar);
    } else {
        area = area.push(Space::with_width(Length::Fixed(spacing::SCROLLBAR_WIDTH)));
    }
    area.into()
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEIGHT: f32 = 300.0;
    const GAP: f32 = 16.0;

    fn snap(total: usize, visible: usize) -> RowSnap {
        let snap = RowSnap::default();
        snap.total_rows.set(total);
        snap.visible_rows.set(visible);
        snap
    }

    #[test]
    fn the_window_holds_only_whole_rows_and_at_least_one() {
        assert_eq!(visible_rows(0.0, HEIGHT, GAP), 1);
        assert_eq!(visible_rows(50.0, HEIGHT, GAP), 1);
        assert_eq!(visible_rows(HEIGHT, HEIGHT, GAP), 1);
        // Two rows need 2 * 300 + 16.
        assert_eq!(visible_rows(615.0, HEIGHT, GAP), 1);
        assert_eq!(visible_rows(616.0, HEIGHT, GAP), 2);
        assert_eq!(visible_rows(931.0, HEIGHT, GAP), 2);
        assert_eq!(visible_rows(932.0, HEIGHT, GAP), 3);
        assert_eq!(visible_rows(f32::NAN, HEIGHT, GAP), 1);
    }

    #[test]
    fn the_last_window_start_leaves_no_partial_row() {
        assert_eq!(last_first_row(4, 2), 2);
        assert_eq!(last_first_row(1, 2), 0);
        assert_eq!(last_first_row(0, 2), 0);
        assert_eq!(last_first_row(7, 0), 6);
    }

    #[test]
    fn one_wheel_notch_moves_exactly_one_row_and_stays_in_range() {
        let mut state = snap(4, 2);
        state.scroll(Scroll::Lines(-1.0));
        assert_eq!(state.first_row(), 1);
        state.scroll(Scroll::Lines(-3.0));
        assert_eq!(state.first_row(), 2);
        state.scroll(Scroll::Lines(-1.0));
        assert_eq!(state.first_row(), 2, "cannot pass the last full window");
        state.scroll(Scroll::Lines(1.0));
        assert_eq!(state.first_row(), 1);
        state.scroll(Scroll::Lines(5.0));
        state.scroll(Scroll::Lines(5.0));
        assert_eq!(state.first_row(), 0);
    }

    #[test]
    fn trackpad_pixels_add_up_to_a_row_and_the_bar_jumps_to_a_row() {
        let mut state = snap(6, 2);
        state.scroll(Scroll::Pixels(-10.0));
        assert_eq!(state.first_row(), 0, "a small drag is not yet a row");
        state.scroll(Scroll::Pixels(-PIXELS_PER_STEP));
        assert_eq!(state.first_row(), 1);
        state.scroll(Scroll::ToRow(99));
        assert_eq!(state.first_row(), 4);
        state.reset();
        assert_eq!(state.first_row(), 0);
    }

    #[test]
    fn the_grid_builds_at_every_height_and_column_count() {
        for theme in [Theme::Light, Theme::Dark] {
            for columns in [1, 2, 3] {
                for count in [0, 1, 6, 7, 40] {
                    for available in [0.0, 200.0, 640.0, 1400.0] {
                        let state = RowSnap::default();
                        let cells: Vec<Element<'_, Scroll>> = (0..count)
                            .map(|_| Space::with_height(Length::Fill).into())
                            .collect();
                        let _: Element<'_, Scroll> =
                            view(&state, theme, cells, columns, available, |scroll| scroll);
                    }
                }
            }
        }
    }
}
