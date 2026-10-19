//! Named icon constants (Section 6). Every constant is one code point of
//! the bundled Lucide icon font (`assets/fonts/lucide.ttf`, ISC licence,
//! see `assets/fonts/LICENSE-lucide.txt`), so an icon looks the same on
//! every machine instead of depending on which system fonts carry the
//! glyph. Draw them with [`icon`] (or `.font(fonts::ICONS)`), never with
//! the text font. The `tests` module checks each constant is a single
//! character that the font file really contains.

use iced::widget::{text, Text};

use super::fonts;

/// An icon glyph drawn in the icon font; chain `.size(..)`/`.color(..)`.
pub fn icon<'a>(glyph: &'static str) -> Text<'a> {
    text(glyph).font(fonts::ICONS)
}

pub const CHEVRON_RIGHT: &str = "\u{e073}";
pub const CHEVRON_LEFT: &str = "\u{e072}";
/// Dropdown/expandable indicator -- today, the sidebar profile card.
pub const CARET_DOWN: &str = "\u{e071}";
/// Theme toggle, shown while dark mode is active (a click switches to light).
pub const SUN: &str = "\u{e17b}";
/// Theme toggle, shown while light mode is active (a click switches to dark).
pub const MOON: &str = "\u{e121}";
pub const SORT_ASCENDING: &str = "\u{e074}";
pub const SORT_DESCENDING: &str = "\u{e071}";
/// Vertical "more options" trigger on a data table row.
pub const KEBAB: &str = "\u{e0bb}";
/// Horizontal three dots.
pub const MORE: &str = "\u{e0ba}";
/// Leading dot on a data table status pill.
pub const STATUS_DOT: &str = "\u{e453}";
/// Tree row toggles: expanded points down, collapsed points right.
pub const TREE_EXPANDED: &str = "\u{e071}";
pub const TREE_COLLAPSED: &str = "\u{e073}";
pub const SEARCH: &str = "\u{e154}";
pub const RESET: &str = "\u{e14c}";
pub const COLUMNS: &str = "\u{e09d}";
pub const EXPORT: &str = "\u{e0b6}";
pub const CLOSE: &str = "\u{e1b1}";
pub const ADD: &str = "\u{e140}";
/// Detail-panel field glyphs (Maintain Customers/Vendors).
pub const FIELD_NAME: &str = "\u{e19e}";
pub const FIELD_COMPANY: &str = "\u{e28f}";
pub const FIELD_PHONE: &str = "\u{e136}";
pub const FIELD_EMAIL: &str = "\u{e112}";
pub const FIELD_WEBSITE: &str = "\u{e0eb}";
pub const FIELD_ADDRESS: &str = "\u{e114}";
pub const FIELD_PAYMENT: &str = "\u{e203}";
pub const FIELD_NOTE: &str = "\u{e302}";
pub const FIELD_STATUS: &str = "\u{e348}";
pub const FIELD_OTHER: &str = "\u{e453}";
/// "Back" arrow on an inline page view.
pub const ARROW_LEFT: &str = "\u{e04c}";
/// Pencil on a row's "edit" icon button.
pub const EDIT: &str = "\u{e1f8}";
/// Dash drawn in a partially-checked (tri-state) checkbox.
pub const INDETERMINATE: &str = "\u{e11f}";
/// Tick drawn in a checked box.
pub const CHECK: &str = "\u{e070}";
/// Paperclip on an "Add Attachment" button.
pub const ATTACH: &str = "\u{e130}";
/// Document glyph leading an attached file's row.
pub const FILE: &str = "\u{e0d0}";
/// Trash can on a row's "remove" icon button.
pub const REMOVE: &str = "\u{e18d}";
/// Circled "i" on a row's "details" action.
pub const INFO: &str = "\u{e0fe}";
/// Pillared building -- a bank or other financial institution.
pub const LANDMARK: &str = "\u{e239}";
/// Money coming in (arrow down-left).
pub const INFLOW: &str = "\u{e047}";
/// Money going out (arrow up-right).
pub const OUTFLOW: &str = "\u{e051}";
/// Archive box on an "archive" icon button.
pub const ARCHIVE: &str = "\u{e045}";
/// A warehouse building.
pub const WAREHOUSE: &str = "\u{e3e9}";
/// A package -- stock held somewhere.
pub const PACKAGE: &str = "\u{e12c}";
/// A grid of squares -- a layout or bin map.
pub const LAYOUT_GRID: &str = "\u{e104}";

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: &[(&str, &str)] = &[
        ("CHEVRON_RIGHT", CHEVRON_RIGHT),
        ("CHEVRON_LEFT", CHEVRON_LEFT),
        ("CARET_DOWN", CARET_DOWN),
        ("SUN", SUN),
        ("MOON", MOON),
        ("SORT_ASCENDING", SORT_ASCENDING),
        ("SORT_DESCENDING", SORT_DESCENDING),
        ("KEBAB", KEBAB),
        ("MORE", MORE),
        ("STATUS_DOT", STATUS_DOT),
        ("TREE_EXPANDED", TREE_EXPANDED),
        ("TREE_COLLAPSED", TREE_COLLAPSED),
        ("SEARCH", SEARCH),
        ("RESET", RESET),
        ("COLUMNS", COLUMNS),
        ("EXPORT", EXPORT),
        ("CLOSE", CLOSE),
        ("ADD", ADD),
        ("FIELD_NAME", FIELD_NAME),
        ("FIELD_COMPANY", FIELD_COMPANY),
        ("FIELD_PHONE", FIELD_PHONE),
        ("FIELD_EMAIL", FIELD_EMAIL),
        ("FIELD_WEBSITE", FIELD_WEBSITE),
        ("FIELD_ADDRESS", FIELD_ADDRESS),
        ("FIELD_PAYMENT", FIELD_PAYMENT),
        ("FIELD_NOTE", FIELD_NOTE),
        ("FIELD_STATUS", FIELD_STATUS),
        ("FIELD_OTHER", FIELD_OTHER),
        ("ARROW_LEFT", ARROW_LEFT),
        ("EDIT", EDIT),
        ("INDETERMINATE", INDETERMINATE),
        ("CHECK", CHECK),
        ("ATTACH", ATTACH),
        ("FILE", FILE),
        ("REMOVE", REMOVE),
        ("INFO", INFO),
        ("LANDMARK", LANDMARK),
        ("INFLOW", INFLOW),
        ("OUTFLOW", OUTFLOW),
        ("ARCHIVE", ARCHIVE),
        ("WAREHOUSE", WAREHOUSE),
        ("PACKAGE", PACKAGE),
        ("LAYOUT_GRID", LAYOUT_GRID),
    ];

    #[test]
    fn every_icon_is_one_char_present_in_the_bundled_font() {
        let data = crate::frontend::components::fonts::ICONS_TTF;
        let face = ttf_parser::Face::parse(data, 0).expect("bundled icon font parses");
        for (name, glyph) in ALL {
            let mut chars = glyph.chars();
            let (Some(ch), None) = (chars.next(), chars.next()) else {
                panic!("{name} is not a single character");
            };
            assert!(
                face.glyph_index(ch).is_some(),
                "{name} (U+{:04X}) is missing from lucide.ttf",
                ch as u32
            );
        }
    }

    #[test]
    fn the_font_constant_names_the_bundled_family() {
        let data = crate::frontend::components::fonts::ICONS_TTF;
        let face = ttf_parser::Face::parse(data, 0).unwrap();
        let family = face
            .names()
            .into_iter()
            .find(|n| n.name_id == ttf_parser::name_id::FAMILY && n.is_unicode())
            .and_then(|n| n.to_string());
        assert_eq!(family.as_deref(), Some("lucide"));
    }
}
