//! Text sizes for page/section headers (Section 6). Colors always come
//! from the active theme's `neutral_front` (description) or `text_primary`
//! (title) -- no new colors here.

pub fn page_title_size() -> f32 {
    super::typography::title()
}

pub fn page_description_size() -> f32 {
    super::typography::label()
}

pub fn section_heading_size() -> f32 {
    super::typography::panel_title()
}

pub fn section_description_size() -> f32 {
    super::typography::body()
}
