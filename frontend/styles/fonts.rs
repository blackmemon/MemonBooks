//! White Theme font family (Section 6 of PROJECT_SPEC.md).
//!
//! Decision: system default font stack -- no bundled/custom font is
//! shipped, so `DEFAULT` and `BOLD` below are just named handles onto
//! iced's own default font (regular and bold weight), not a real
//! typeface choice. They exist so call sites share one constant
//! instead of each writing out `Font { weight: iced::font::Weight::
//! Bold, ..Font::DEFAULT }` -- found, character-for-character
//! identical, at 7 separate call sites across `components/common`
//! before this file existed. If a future non-default font decision
//! gets made, this is the one place that changes.

use iced::Font;

pub const DEFAULT: Font = Font::DEFAULT;

pub const BOLD: Font = Font { weight: iced::font::Weight::Bold, ..Font::DEFAULT };
