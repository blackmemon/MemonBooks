//! Text sizes for page/section headers (Section 6). Colors always come
//! from the active theme's `neutral_front` (description) or `text_primary`
//! (title) -- no new colors here.

use super::font_scale::scaled;

pub fn page_title_size() -> f32 {
    scaled(26.0)
}

pub fn page_description_size() -> f32 {
    scaled(13.0)
}

pub fn section_heading_size() -> f32 {
    scaled(16.0)
}

pub fn section_description_size() -> f32 {
    scaled(12.0)
}
