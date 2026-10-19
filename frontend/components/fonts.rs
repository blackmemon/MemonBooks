use iced::Font;

use super::font_scale::scaled;

/// Pragati Narrow (SIL OFL 1.1, `assets/fonts/pragati-narrow/LICENSE`),
/// the app's text font. The name is the family stored in the TTF's name
/// table, not the file name. Set as the application default font in
/// `main.rs`, so every widget without an explicit `.font()` uses it.
pub const DEFAULT: Font = Font::with_name("Pragati Narrow");
/// Bundled Lucide icon font (`assets/fonts/lucide.ttf`); loaded at app
/// start in `main.rs`. Icons are drawn with this, never the text font.
pub const ICONS: Font = Font::with_name("lucide");
/// Resolves to the real Pragati Narrow Bold face ([`BOLD_TTF`]), not a
/// synthesized bold.
pub const BOLD: Font = Font {
    weight: iced::font::Weight::Bold,
    ..DEFAULT
};

/// Latin subset (Basic Latin + Latin-1 + common punctuation/currency) of
/// Pragati Narrow Regular, the face behind [`DEFAULT`].
pub const REGULAR_TTF: &[u8] =
    include_bytes!("../../assets/fonts/pragati-narrow/static/pragati-narrow-latin-400-normal.ttf");
/// Latin subset of Pragati Narrow Bold, the face behind [`BOLD`].
pub const BOLD_TTF: &[u8] =
    include_bytes!("../../assets/fonts/pragati-narrow/static/pragati-narrow-latin-700-normal.ttf");
/// The Lucide icon font, the face behind [`ICONS`].
pub const ICONS_TTF: &[u8] = include_bytes!("../../assets/fonts/lucide.ttf");

/// Every font the app uses, embedded in the binary at compile time so
/// nothing is read from disk or the system at runtime. `main.rs` loads
/// each one once at startup.
pub const EMBEDDED: [&[u8]; 3] = [REGULAR_TTF, BOLD_TTF, ICONS_TTF];

pub fn button_size() -> f32 {
    scaled(13.0)
}

pub fn label_size() -> f32 {
    scaled(13.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn family_name(font: Font) -> &'static str {
        match font.family {
            iced::font::Family::Name(name) => name,
            other => panic!("expected a named family, got {other:?}"),
        }
    }

    fn parsed_family(data: &[u8]) -> String {
        let face = ttf_parser::Face::parse(data, 0).expect("embedded font parses");
        face.names()
            .into_iter()
            .find(|n| n.name_id == ttf_parser::name_id::FAMILY && n.is_unicode())
            .and_then(|n| n.to_string())
            .expect("embedded font has a family name")
    }

    #[test]
    fn every_embedded_font_is_non_empty_and_parses() {
        for data in EMBEDDED {
            assert!(!data.is_empty());
            ttf_parser::Face::parse(data, 0).expect("embedded font parses");
        }
    }

    #[test]
    fn font_constants_name_the_family_inside_each_embedded_ttf() {
        assert_eq!(parsed_family(REGULAR_TTF), family_name(DEFAULT));
        assert_eq!(parsed_family(BOLD_TTF), family_name(BOLD));
        assert_eq!(parsed_family(ICONS_TTF), family_name(ICONS));
    }

    #[test]
    fn regular_and_bold_are_real_weight_files() {
        let regular = ttf_parser::Face::parse(REGULAR_TTF, 0).unwrap();
        let bold = ttf_parser::Face::parse(BOLD_TTF, 0).unwrap();
        assert_eq!(regular.weight(), ttf_parser::Weight::Normal);
        assert_eq!(bold.weight(), ttf_parser::Weight::Bold);
        assert_eq!(BOLD.weight, iced::font::Weight::Bold);
    }
}
