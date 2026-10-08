//! Persists the user's chosen font size across restarts. A device-level
//! UI preference, not company data, so it lives in its own plain-text
//! file under the app's data directory rather than the database --
//! same reasoning as `theme_preference`. `load` returns `None` when
//! nothing has been saved yet (no file, or unrecognized contents), so
//! the caller knows to fall back to the default (`FontScale::Medium`)
//! instead of erroring.

use std::path::Path;

use crate::frontend::components::font_scale::FontScale;

pub fn load(path: &Path) -> Option<FontScale> {
    let contents = std::fs::read_to_string(path).ok()?;
    match contents.trim() {
        "small" => Some(FontScale::Small),
        "medium" => Some(FontScale::Medium),
        "large" => Some(FontScale::Large),
        "extra_large" => Some(FontScale::ExtraLarge),
        _ => None,
    }
}

pub fn save(path: &Path, scale: FontScale) -> std::io::Result<()> {
    let value = match scale {
        FontScale::Small => "small",
        FontScale::Medium => "medium",
        FontScale::Large => "large",
        FontScale::ExtraLarge => "extra_large",
    };
    std::fs::write(path, value)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path(label: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "memonbooks_font_scale_preference_test_{label}_{:?}",
            std::thread::current().id()
        ))
    }

    #[test]
    fn load_returns_none_when_no_file_exists() {
        let path = temp_path("missing");

        assert_eq!(load(&path), None);
    }

    #[test]
    fn save_then_load_round_trips_every_scale() {
        for scale in [
            FontScale::Small,
            FontScale::Medium,
            FontScale::Large,
            FontScale::ExtraLarge,
        ] {
            let path = temp_path(&format!("{scale:?}"));

            save(&path, scale).unwrap();

            assert_eq!(load(&path), Some(scale));
            let _ = std::fs::remove_file(&path);
        }
    }

    #[test]
    fn saving_a_new_choice_overwrites_the_previous_one() {
        let path = temp_path("overwrite");

        save(&path, FontScale::Small).unwrap();
        save(&path, FontScale::ExtraLarge).unwrap();

        assert_eq!(load(&path), Some(FontScale::ExtraLarge));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn load_returns_none_for_unrecognized_contents() {
        let path = temp_path("garbage");
        std::fs::write(&path, "not-a-size").unwrap();

        assert_eq!(load(&path), None);
        let _ = std::fs::remove_file(&path);
    }
}
