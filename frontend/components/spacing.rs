//! Shared spacing scale (Section 6). Every new shell/list component uses
//! these instead of ad-hoc padding/spacing numbers, so the app-wide
//! rhythm stays consistent as more views adopt the pattern.
//!
//! `XS` through `XL` are the original five tokens. The Phase 1
//! consistency audit found six more distinct values in real use across
//! the app -- `2`, `6`, `10`, `12`, `20`, and `40` -- that don't land on
//! any of the five. Each is genuinely different from its neighbors, not
//! typo drift (e.g. `20` alone accounts for 29 of the 258 raw calls the
//! audit found, as the standard content-column padding), so each keeps
//! its own constant rather than being rounded to the nearest existing
//! tier. Named by pixel value (`SPACE_N`) rather than invented T-shirt
//! sizes, since there's no honest "between XS and SM" word for six more
//! sizes -- the number is the name.
//!
//! A seventh, `SPACE_14`, surfaced only while migrating array-form
//! `.padding([vertical, horizontal])` calls (Batch 9) -- it never
//! appears as a bare single-value `.padding(N)`, only as the
//! horizontal half of a button's `[8.0, 14.0]` pair (11 call sites), so
//! the original single-value audit pass didn't catch it. It numerically
//! matches `typography::emphasis()` (`14.0`), but that's coincidence, not
//! shared meaning -- one is a padding distance, the other a font size --
//! so it gets its own constant here rather than reusing that one.

use iced::Padding;

pub const XS: f32 = 4.0;
pub const SM: f32 = 8.0;
pub const MD: f32 = 16.0;
pub const LG: f32 = 24.0;
pub const XL: f32 = 32.0;

pub const SPACE_2: f32 = 2.0;
pub const SPACE_4: f32 = 4.0;
pub const SPACE_6: f32 = 6.0;
pub const SPACE_8: f32 = 8.0;
pub const SPACE_10: f32 = 10.0;
pub const SPACE_12: f32 = 12.0;
pub const SPACE_14: f32 = 14.0;
pub const SPACE_18: f32 = 18.0;
pub const SPACE_24: f32 = 24.0;
pub const SPACE_30: f32 = 30.0;
pub const SPACE_5: f32 = 5.0;
pub const SPACE_1: f32 = 1.0;
pub const SPACE_20: f32 = 20.0;
pub const SPACE_40: f32 = 40.0;
pub const ICON_BUTTON_SIZE: f32 = 36.0;
pub const PAGE_BUTTON_WIDTH: f32 = 32.0;
pub const FIELD_LABEL_WIDTH_100: f32 = 100.0;
pub const FIELD_LABEL_WIDTH_130: f32 = 130.0;
pub const FIELD_LABEL_WIDTH_160: f32 = 160.0;
pub const TAB_UNDERLINE_HEIGHT: f32 = 2.0;
pub const SEARCH_ICON_INSET: f32 = 36.0;
pub const TABLE_ROW_HEIGHT: f32 = 52.0;
pub const AVATAR_SIZE: f32 = 36.0;
pub const CHECKBOX_COLUMN_WIDTH: f32 = 32.0;
pub const ACTION_COLUMN_WIDTH: f32 = 48.0;
/// The select-box column of a wide table, the box plus its gutter.
pub const SELECT_COLUMN_WIDTH: f32 = 44.0;
/// Height of a multi-line text input (e.g. a Description editor).
pub const TEXT_AREA_HEIGHT: f32 = 96.0;
pub const SEARCH_WIDTH: f32 = 260.0;
pub const SCROLLBAR_WIDTH: f32 = 8.0;
pub const HEADER_PADDING: Padding = Padding {
    top: LG,
    right: LG,
    bottom: 0.0,
    left: LG,
};
pub const CONTENT_PADDING: Padding = Padding {
    top: 0.0,
    right: LG,
    bottom: LG,
    left: LG,
};

/// Responsive breakpoints, defined once for every page. A width below
/// `BREAKPOINT_NARROW` is [`LayoutClass::Narrow`], one below
/// `BREAKPOINT_WIDE` is [`LayoutClass::Medium`], anything else is
/// [`LayoutClass::Wide`].
pub const BREAKPOINT_NARROW: f32 = 800.0;
pub const BREAKPOINT_WIDE: f32 = 1200.0;
/// From here (about a 1920 screen) the type scale grows once more.
pub const BREAKPOINT_XWIDE: f32 = 1700.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutClass {
    Narrow,
    Medium,
    Wide,
}

pub fn layout_class(width: f32) -> LayoutClass {
    if width < BREAKPOINT_NARROW {
        LayoutClass::Narrow
    } else if width < BREAKPOINT_WIDE {
        LayoutClass::Medium
    } else {
        LayoutClass::Wide
    }
}

/// Side panel next to a table: never narrower than `PANEL_MIN_WIDTH` (so
/// inputs never clip), one third of the page when wide, and
/// `PANEL_MEDIUM_SHARE` of it when medium.
pub const PANEL_MIN_WIDTH: f32 = 420.0;
pub const PANEL_WIDE_SHARE: f32 = 1.0 / 3.0;
pub const PANEL_MEDIUM_SHARE: f32 = 0.45;
/// A panel at least this wide lays its fields out in two columns.
pub const PANEL_TWO_COLUMN_MIN_WIDTH: f32 = 520.0;
/// Smallest width of a header or footer button, so its label never wraps.
pub const BUTTON_MIN_WIDTH: f32 = 72.0;
/// Narrowest the header search box is allowed to shrink to.
pub const SEARCH_MIN_WIDTH: f32 = 140.0;
/// Side of the tinted icon square leading a card or list row.
pub const ICON_BOX_SIZE: f32 = 44.0;

/// Side of one numbered cell in a layout grid.
pub const GRID_CELL_SIZE: f32 = 28.0;
/// Width of a fixed summary column beside a page's main area.
pub const SUMMARY_PANEL_WIDTH: f32 = 260.0;
/// Side of the colour swatch in a legend entry.
pub const LEGEND_SWATCH_SIZE: f32 = 12.0;
/// Height of a thin progress bar.
pub const PROGRESS_BAR_HEIGHT: f32 = 6.0;

/// Height of one card in a card grid, at the base type size; grids scroll
/// by whole rows of this height. See [`card_height`].
pub const CARD_HEIGHT: f32 = 300.0;
/// Vertical space between two rows of a card grid.
pub const ROW_GAP: f32 = MD;

/// A control size or card height grown with the type scale, so what it
/// holds never clips when the text gets larger.
pub fn grown(base: f32) -> f32 {
    base * crate::frontend::components::typography::body()
        / crate::frontend::components::typography::BASE_BODY
}

pub fn card_height() -> f32 {
    grown(CARD_HEIGHT)
}

pub fn icon_button_size() -> f32 {
    grown(ICON_BUTTON_SIZE)
}

pub fn icon_box_size() -> f32 {
    grown(ICON_BOX_SIZE)
}

pub fn grid_cell_size() -> f32 {
    grown(GRID_CELL_SIZE)
}

pub fn page_button_width() -> f32 {
    grown(PAGE_BUTTON_WIDTH)
}

pub fn table_row_height() -> f32 {
    grown(TABLE_ROW_HEIGHT)
}

/// Cards per line of a card grid: three when wide, two when medium, one
/// when narrow.
pub fn grid_columns(class: LayoutClass) -> usize {
    match class {
        LayoutClass::Wide => 3,
        LayoutClass::Medium => 2,
        LayoutClass::Narrow => 1,
    }
}

/// How a page's width is shared between its main content and a side
/// panel; `None` means that part is not shown.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PanelSplit {
    pub content: Option<f32>,
    pub panel: Option<f32>,
}

/// Wide and medium pages show the content beside the open panel (the
/// panel taking [`PANEL_WIDE_SHARE`] or [`PANEL_MEDIUM_SHARE`], never
/// narrower than [`PANEL_MIN_WIDTH`]); a narrow page shows only one of
/// them, the panel replacing the content while it is open. `gap` is the
/// space between the two.
pub fn panel_split(inner: f32, panel_open: bool, gap: f32) -> PanelSplit {
    match (panel_open, layout_class(inner)) {
        (false, _) => PanelSplit {
            content: Some(inner),
            panel: None,
        },
        (true, LayoutClass::Narrow) => PanelSplit {
            content: None,
            panel: Some(inner),
        },
        (true, class) => {
            let share = if class == LayoutClass::Wide {
                PANEL_WIDE_SHARE
            } else {
                PANEL_MEDIUM_SHARE
            };
            let panel = (inner * share).max(PANEL_MIN_WIDTH);
            PanelSplit {
                content: Some((inner - panel - gap).max(0.0)),
                panel: Some(panel),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_grid_has_three_two_and_one_columns_across_the_breakpoints() {
        assert_eq!(grid_columns(layout_class(1920.0)), 3);
        assert_eq!(grid_columns(layout_class(1366.0)), 3);
        assert_eq!(grid_columns(layout_class(900.0)), 2);
        assert_eq!(grid_columns(layout_class(640.0)), 1);
    }

    #[test]
    fn the_panel_takes_a_share_when_wide_and_the_whole_page_when_narrow() {
        let closed = panel_split(1500.0, false, SPACE_20);
        assert_eq!(closed.content, Some(1500.0));
        assert_eq!(closed.panel, None);

        let wide = panel_split(1500.0, true, SPACE_20);
        assert_eq!(wide.panel, Some(500.0));
        assert_eq!(wide.content, Some(980.0));

        let medium = panel_split(900.0, true, SPACE_20);
        assert_eq!(medium.panel, Some(PANEL_MIN_WIDTH));

        let narrow = panel_split(600.0, true, SPACE_20);
        assert_eq!(narrow.content, None);
        assert_eq!(narrow.panel, Some(600.0));
    }
}
