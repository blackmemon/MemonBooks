//! Centralized font-size scaling: one global multiplier, chosen from
//! Settings, that every size getter in [`super::typography`],
//! [`super::headers`], and [`super::fonts`] applies before returning a
//! size.
//!
//! Kept as process-global state rather than threaded through every
//! `view(state, theme)` call the way [`super::colors::Theme`] is --
//! doing that would mean changing the signature of every view and
//! component function in the app for one cross-cutting concern.
//! `MemonBooks.font_scale` (in `src/main.rs`) is still the source of
//! truth for persistence and for what the Settings picker displays;
//! [`set`] just mirrors it here so size getters can read it without a
//! `Ctx`. [`set`] is called once at startup (after loading the
//! persisted preference) and again every time the Settings picker
//! changes it, so a change takes effect on the very next `view()` with
//! no restart required.

use std::sync::atomic::{AtomicU32, AtomicU8, Ordering};

use super::spacing;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FontScale {
    Small,
    #[default]
    Medium,
    Large,
    ExtraLarge,
}

/// In picker order, smallest to largest.
pub const ALL: [FontScale; 4] = [
    FontScale::Small,
    FontScale::Medium,
    FontScale::Large,
    FontScale::ExtraLarge,
];

impl FontScale {
    fn multiplier(self) -> f32 {
        match self {
            FontScale::Small => 0.875,
            FontScale::Medium => 1.0,
            FontScale::Large => 1.125,
            FontScale::ExtraLarge => 1.25,
        }
    }

    fn from_index(index: u8) -> Self {
        match index {
            0 => FontScale::Small,
            2 => FontScale::Large,
            3 => FontScale::ExtraLarge,
            _ => FontScale::Medium,
        }
    }

    fn index(self) -> u8 {
        match self {
            FontScale::Small => 0,
            FontScale::Medium => 1,
            FontScale::Large => 2,
            FontScale::ExtraLarge => 3,
        }
    }
}

impl std::fmt::Display for FontScale {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            FontScale::Small => "Small",
            FontScale::Medium => "Medium",
            FontScale::Large => "Large",
            FontScale::ExtraLarge => "Extra Large",
        })
    }
}

static CURRENT: AtomicU8 = AtomicU8::new(1); // FontScale::Medium

pub fn set(scale: FontScale) {
    CURRENT.store(scale.index(), Ordering::Relaxed);
}

pub fn current() -> FontScale {
    FontScale::from_index(CURRENT.load(Ordering::Relaxed))
}

/// Window width (logical pixels) the type scale is currently sized for;
/// a medium-class width until the first layout reports the real one.
static WINDOW_WIDTH: AtomicU32 = AtomicU32::new(1000.0_f32.to_bits());

/// Mirrors the window width here (like [`set`] does for the preference)
/// so size getters need no context. Called by the app shell on layout.
pub fn set_window_width(width: f32) {
    if width.is_finite() && width > 0.0 {
        WINDOW_WIDTH.store(width.to_bits(), Ordering::Relaxed);
    }
}

pub fn window_width() -> f32 {
    f32::from_bits(WINDOW_WIDTH.load(Ordering::Relaxed))
}

/// How much the type grows with the window, from the shared breakpoints:
/// slightly smaller when narrow, the base size when medium, larger from
/// 1200 and larger again from 1700 (about a 1920 screen). iced already
/// works in logical pixels, so the display's own scale factor is applied
/// on top of this by the window system.
pub fn width_factor(width: f32) -> f32 {
    if width < spacing::BREAKPOINT_NARROW {
        0.9
    } else if width < spacing::BREAKPOINT_WIDE {
        1.0
    } else if width < spacing::BREAKPOINT_XWIDE {
        1.1
    } else {
        1.2
    }
}

/// Applies the user's Font Size choice and the window-width factor to a
/// base (medium-window) size. Every getter in `typography` routes its
/// base literal through this instead of returning it directly.
pub fn scaled(base: f32) -> f32 {
    base * current().multiplier() * width_factor(window_width())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // `CURRENT` is process-global, so tests that mutate it must not run
    // concurrently with each other.
    static TEST_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn medium_is_the_default_and_leaves_sizes_unscaled() {
        let _guard = TEST_LOCK.lock().unwrap();
        set(FontScale::Medium);

        assert_eq!(current(), FontScale::Medium);
        assert_eq!(scaled(12.0), 12.0);
    }

    #[test]
    fn set_changes_what_scaled_returns_until_changed_again() {
        let _guard = TEST_LOCK.lock().unwrap();
        set(FontScale::ExtraLarge);
        assert_eq!(scaled(12.0), 15.0);

        set(FontScale::Small);
        assert_eq!(scaled(12.0), 10.5);

        set(FontScale::Medium);
    }
}
