use iced::Font;

use super::font_scale::scaled;

pub const DEFAULT: Font = Font::with_name("Nimbus Sans Narrow");
/// Bundled Lucide icon font (`assets/fonts/lucide.ttf`); loaded at app
/// start in `main.rs`. Icons are drawn with this, never the text font.
pub const ICONS: Font = Font::with_name("lucide");
pub const BOLD: Font = Font {
    weight: iced::font::Weight::Bold,
    ..DEFAULT
};

pub fn button_size() -> f32 {
    scaled(13.0)
}

pub fn label_size() -> f32 {
    scaled(13.0)
}
