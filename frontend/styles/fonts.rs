use iced::Font;

use super::font_scale::scaled;

pub const DEFAULT: Font = Font::DEFAULT;
pub const BOLD: Font = Font {
    weight: iced::font::Weight::Bold,
    ..Font::DEFAULT
};

pub fn button_size() -> f32 {
    scaled(13.0)
}

pub fn label_size() -> f32 {
    scaled(13.0)
}
