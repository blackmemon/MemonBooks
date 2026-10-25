use iced::Color;

use super::states;

const fn rgb8(r: u8, g: u8, b: u8) -> Color {
    Color {
        r: r as f32 / 255.0,
        g: g as f32 / 255.0,
        b: b as f32 / 255.0,
        a: 1.0,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Theme {
    Light,
    Dark,
}

impl Theme {
    pub fn accent_front(&self) -> Color {
        match self {
            Theme::Light | Theme::Dark => rgb8(0xFF, 0xFF, 0xFF),
        }
    }

    pub fn transparent(&self) -> Color {
        match self {
            Theme::Light | Theme::Dark => Color {
                a: 0.0,
                ..Color::BLACK
            },
        }
    }

    // ---- Sidebar tokens -------------------------------------------
    // The sidebar is black in both themes, so these ignore `self`: one
    // value per token, defined once here. They are methods (not free
    // constants) only so a caller holding a `Theme` reads them like every
    // other token.

    /// Sidebar panel: pure black.
    pub fn sidebar_bg(&self) -> Color {
        match self {
            Theme::Light | Theme::Dark => rgb8(0x00, 0x00, 0x00),
        }
    }

    /// Hovered sidebar row or button: a step lighter than the panel.
    pub fn sidebar_hover(&self) -> Color {
        match self {
            Theme::Light | Theme::Dark => rgb8(0x1C, 0x1C, 0x1C),
        }
    }

    /// Active sidebar row: lighter again, so it stays distinct from hover.
    pub fn sidebar_active(&self) -> Color {
        match self {
            Theme::Light | Theme::Dark => rgb8(0x2B, 0x2B, 0x2B),
        }
    }

    /// Sidebar labels and icons.
    pub fn sidebar_text(&self) -> Color {
        match self {
            Theme::Light | Theme::Dark => rgb8(0xFF, 0xFF, 0xFF),
        }
    }

    /// Sidebar secondary text (the "Signed in" line).
    pub fn sidebar_muted(&self) -> Color {
        match self {
            Theme::Light | Theme::Dark => rgb8(0xA8, 0xA8, 0xA8),
        }
    }

    /// Hairline between the sidebar's two nav groups.
    pub fn sidebar_divider(&self) -> Color {
        match self {
            Theme::Light | Theme::Dark => rgb8(0x33, 0x33, 0x33),
        }
    }

    pub fn page_bg(&self) -> Color {
        match self {
            Theme::Light => rgb8(255, 255, 255),
            Theme::Dark => rgb8(34, 34, 38),
        }
    }

    pub fn neutral_bg(&self) -> Color {
        match self {
            Theme::Light => rgb8(0xE5, 0xE7, 0xEB),
            Theme::Dark => rgb8(0x3A, 0x3D, 0x44),
        }
    }

    pub fn neutral_bg_hover(&self) -> Color {
        match self {
            Theme::Light => rgb8(248, 250, 252),
            Theme::Dark => rgb8(107, 114, 128),
        }
    }

    pub fn neutral_font(&self) -> Color {
        match self {
            Theme::Light => rgb8(0x11, 0x18, 0x27),
            Theme::Dark => rgb8(181, 181, 181),
        }
    }

    pub fn neutral_border(&self) -> Color {
        match self {
            Theme::Light => rgb8(235, 235, 235),
            Theme::Dark => rgb8(0x74, 0x74, 0x7C),
        }
    }

    pub fn neutral_border_hover(&self) -> Color {
        match self {
            Theme::Light => rgb8(0x2C, 0x8F, 0xBD),
            Theme::Dark => rgb8(0xFF, 0xFF, 0xFF),
        }
    }

    /// Primary accent -- buttons, active nav item, links, focus rings.
    pub fn primary(&self) -> Color {
        match self {
            Theme::Light => rgb8(0x51, 0xA2, 0xFF),
            Theme::Dark => rgb8(0x7B, 0x86, 0x99),
        }
    }

    /// `alpha` as drawn for a translucent tint: a dark surface needs twice
    /// the opacity for the same tint to read, light is used as given.
    pub fn tint_alpha(&self, alpha: f32) -> f32 {
        match self {
            Theme::Light => alpha,
            Theme::Dark => (alpha * 2.0).min(1.0),
        }
    }

    /// Opacity of a disabled control: dimmed, but still readable on dark.
    pub fn disabled_alpha(&self) -> f32 {
        match self {
            Theme::Light => states::DISABLED_ALPHA,
            Theme::Dark => states::DISABLED_ALPHA_DARK,
        }
    }

    pub fn primary_tint(&self, alpha: f32) -> Color {
        Color {
            a: self.tint_alpha(alpha),
            ..self.primary()
        }
    }

    /// Primary button hover/pressed color.
    pub fn primary_hover(&self) -> Color {
        match self {
            Theme::Light => rgb8(0x56, 0xAE, 0xD7),
            Theme::Dark => rgb8(0x5F, 0x69, 0x7A),
        }
    }

    /// Neutral button color.
    pub fn secondary(&self) -> Color {
        match self {
            Theme::Light => rgb8(0x6B, 0x72, 0x80),
            Theme::Dark => rgb8(0x4B, 0x55, 0x63),
        }
    }

    /// Neutral button hover/pressed color.
    pub fn secondary_hover(&self) -> Color {
        match self {
            Theme::Light => rgb8(0x4B, 0x55, 0x63),
            Theme::Dark => rgb8(0x6B, 0x72, 0x80),
        }
    }

    /// Add button color.
    pub fn add(&self) -> Color {
        match self {
            Theme::Light => rgb8(0xFF, 0x89, 0x04),
            Theme::Dark => rgb8(0xFF, 0x89, 0x04),
        }
    }

    /// Add button hover/pressed color.
    pub fn add_hover(&self) -> Color {
        match self {
            Theme::Light => rgb8(0xFF, 0x9D, 0x2E),
            Theme::Dark => rgb8(0xFF, 0x9D, 0x2E),
        }
    }

    /// Success / positive background.
    pub fn positive_bg(&self) -> Color {
        match self {
            Theme::Light => rgb8(0xC6, 0xF1, 0xCE),
            Theme::Dark => rgb8(0x16, 0x4A, 0x2B),
        }
    }

    /// Success / positive hover background.
    pub fn positive_bg_hover(&self) -> Color {
        match self {
            Theme::Light => rgb8(0x16, 0xA3, 0x4A),
            Theme::Dark => rgb8(0x16, 0xA3, 0x4A),
        }
    }

    /// Success / positive text & icon color.
    pub fn positive_front(&self) -> Color {
        match self {
            Theme::Light => rgb8(0x00, 0x64, 0x00),
            Theme::Dark => rgb8(0x7F, 0xE0, 0x8C), // PLACEHOLDER
        }
    }

    /// Error / negative background.
    pub fn negative_bg(&self) -> Color {
        match self {
            Theme::Light => rgb8(0xFF, 0xC8, 0xCE),
            Theme::Dark => rgb8(0x5A, 0x1E, 0x24),
        }
    }

    /// Error / negative hover background.
    pub fn negative_bg_hover(&self) -> Color {
        match self {
            Theme::Light => rgb8(0xDC, 0x26, 0x26),
            Theme::Dark => rgb8(0xDC, 0x26, 0x26),
        }
    }

    /// Error / negative text & icon color.
    pub fn negative_front(&self) -> Color {
        match self {
            Theme::Light => rgb8(0x9C, 0x00, 0x0A),
            Theme::Dark => rgb8(0xFF, 0x8A, 0x93), // PLACEHOLDER
        }
    }

    /// Header background.
    pub fn header_bg(&self) -> Color {
        match self {
            Theme::Light => rgb8(0xFF, 0xC8, 0xCE),
            Theme::Dark => rgb8(0xEF, 0x44, 0x44),
        }
    }

    /// Header hover background.
    pub fn header_bg_hover(&self) -> Color {
        match self {
            Theme::Light => rgb8(0xDC, 0x26, 0x26),
            Theme::Dark => rgb8(0xDC, 0x26, 0x26),
        }
    }

    /// Header text & icon color.
    pub fn header_front(&self) -> Color {
        match self {
            Theme::Light => rgb8(0x9C, 0x00, 0x0A),
            Theme::Dark => rgb8(0xFF, 0x8A, 0x93), // PLACEHOLDER
        }
    }

    pub fn surface_hover(&self) -> Color {
        match self {
            Theme::Light => rgb8(0xF8, 0xFA, 0xFC),
            Theme::Dark => rgb8(0x1F, 0x29, 0x37),
        }
    }

    /// Warning background (pending states, unsaved-changes notices).
    pub fn warning_bg(&self) -> Color {
        match self {
            Theme::Light => rgb8(0xFF, 0xEB, 0xA0),
            Theme::Dark => rgb8(0x3A, 0x2E, 0x0A), // PLACEHOLDER
        }
    }

    /// Warning text & icon color.
    pub fn warning_front(&self) -> Color {
        match self {
            Theme::Light => rgb8(0x9C, 0x5A, 0x00),
            Theme::Dark => rgb8(0xFF, 0xD1, 0x66), // PLACEHOLDER
        }
    }

    /// Neutral background -- status pills with no positive/negative/warning
    /// meaning (e.g. "Draft"), muted panels.
    /// Neutral text & icon color. Doubles as the app's one muted/secondary
    /// text tone (descriptions, secondary avatar lines, inactive tabs) and as
    /// the subtle border/divider color for cards and table rows, so the shell
    /// never needs a hex literal outside this file for those either.
    pub fn neutral_front(&self) -> Color {
        match self {
            Theme::Light => rgb8(0x4B, 0x55, 0x63),
            Theme::Dark => rgb8(0xB7, 0xBD, 0xC6), // PLACEHOLDER
        }
    }

    /// The base card/panel/sidebar surface -- what every White Theme
    /// component hardcoded as `Color::WHITE` before this file existed.
    pub fn surface(&self) -> Color {
        match self {
            Theme::Light => rgb8(0xFF, 0xFF, 0xFF),
            Theme::Dark => rgb8(0x1E, 0x1E, 0x22), // PLACEHOLDER
        }
    }

    /// The app's default body/heading text color -- what every White
    /// Theme component hardcoded as `Color::BLACK` before this file
    /// existed. Not for text sitting on a fixed-color accent chip
    /// (badge pills, the primary button's label) -- those stay literal
    /// `Color::WHITE`/`Color::BLACK` since they're contrast text on a
    /// surface that doesn't change with the theme.
    pub fn text_primary(&self) -> Color {
        match self {
            Theme::Light => rgb8(0x00, 0x00, 0x00),
            Theme::Dark => rgb8(0xF2, 0xF2, 0xF5), // PLACEHOLDER
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// WCAG 2.x relative luminance of an opaque colour.
    fn luminance(color: Color) -> f32 {
        let linear = |channel: f32| {
            if channel <= 0.03928 {
                channel / 12.92
            } else {
                ((channel + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * linear(color.r) + 0.7152 * linear(color.g) + 0.0722 * linear(color.b)
    }

    fn contrast_ratio(a: Color, b: Color) -> f32 {
        let (la, lb) = (luminance(a), luminance(b));
        (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
    }

    #[test]
    fn sidebar_tokens_are_identical_in_light_and_dark() {
        let (l, d) = (Theme::Light, Theme::Dark);
        assert_eq!(l.sidebar_bg(), d.sidebar_bg());
        assert_eq!(l.sidebar_hover(), d.sidebar_hover());
        assert_eq!(l.sidebar_active(), d.sidebar_active());
        assert_eq!(l.sidebar_text(), d.sidebar_text());
        assert_eq!(l.sidebar_muted(), d.sidebar_muted());
        assert_eq!(l.sidebar_divider(), d.sidebar_divider());
        assert_eq!(l.sidebar_bg(), Color::BLACK);
    }

    #[test]
    fn sidebar_text_and_icons_have_at_least_4_5_to_1_contrast_on_every_sidebar_surface() {
        for theme in [Theme::Light, Theme::Dark] {
            for surface in [
                theme.sidebar_bg(),
                theme.sidebar_hover(),
                theme.sidebar_active(),
            ] {
                for (name, front) in [
                    ("text", theme.sidebar_text()),
                    ("muted", theme.sidebar_muted()),
                ] {
                    let ratio = contrast_ratio(front, surface);
                    assert!(
                        ratio >= 4.5,
                        "sidebar {name} on {surface:?} is only {ratio:.2}:1"
                    );
                }
            }
        }
    }

    #[test]
    fn sidebar_hover_and_active_are_distinct_steps_above_the_panel() {
        let t = Theme::Light;
        assert!(luminance(t.sidebar_hover()) > luminance(t.sidebar_bg()));
        assert!(luminance(t.sidebar_active()) > luminance(t.sidebar_hover()));
    }

    // The light palette stays aligned with the White Theme in PROJECT_SPEC.md.
    #[test]
    fn light_theme_matches_the_original_white_theme_values() {
        assert_eq!(Theme::Light.primary(), rgb8(0x51, 0xA2, 0xFF));
        assert_eq!(Theme::Light.positive_bg(), rgb8(0xC6, 0xF1, 0xCE));
        assert_eq!(Theme::Light.positive_front(), rgb8(0x00, 0x64, 0x00));
        assert_eq!(Theme::Light.negative_bg(), rgb8(0xFF, 0xC8, 0xCE));
        assert_eq!(Theme::Light.negative_front(), rgb8(0x9C, 0x00, 0x0A));
        assert_eq!(Theme::Light.warning_bg(), rgb8(0xFF, 0xEB, 0xA0));
        assert_eq!(Theme::Light.warning_front(), rgb8(0x9C, 0x5A, 0x00));
        assert_eq!(Theme::Light.neutral_bg(), rgb8(0xE5, 0xE7, 0xEB));
        assert_eq!(Theme::Light.neutral_front(), rgb8(0x4B, 0x55, 0x63));
        assert_eq!(Theme::Light.surface(), Color::WHITE);
        assert_eq!(Theme::Light.text_primary(), Color::BLACK);
    }

    // Not asserting specific dark values (they're explicitly placeholder
    // and will be replaced) -- just that every method actually branches,
    // so a future edit can't accidentally leave a `Dark` arm returning
    // the light value by copy-paste.
    #[test]
    fn dark_theme_differs_from_light_for_every_token() {
        assert_ne!(Theme::Dark.primary(), Theme::Light.primary());
        assert_ne!(Theme::Dark.positive_bg(), Theme::Light.positive_bg());
        assert_ne!(Theme::Dark.positive_front(), Theme::Light.positive_front());
        assert_ne!(Theme::Dark.negative_bg(), Theme::Light.negative_bg());
        assert_ne!(Theme::Dark.negative_front(), Theme::Light.negative_front());
        assert_ne!(Theme::Dark.warning_bg(), Theme::Light.warning_bg());
        assert_ne!(Theme::Dark.warning_front(), Theme::Light.warning_front());
        assert_ne!(Theme::Dark.neutral_bg(), Theme::Light.neutral_bg());
        assert_ne!(Theme::Dark.neutral_front(), Theme::Light.neutral_front());
        assert_ne!(Theme::Dark.surface(), Theme::Light.surface());
        assert_ne!(Theme::Dark.text_primary(), Theme::Light.text_primary());
    }
}
