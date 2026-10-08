//! Persists the user's manual light/dark theme choice across restarts.
//! A device-level UI preference, not company data, so it lives in its
//! own plain-text file under the app's data directory rather than the
//! database -- a single word ("light"/"dark") doesn't need a schema.
//! `load` returns `None` when nothing has been saved yet (no file, or
//! unrecognized contents), so the caller knows to fall back to OS
//! theme detection instead of defaulting silently to `Light`.

use std::path::Path;

use crate::frontend::components::colors::Theme;

pub fn load(path: &Path) -> Option<Theme> {
    let contents = std::fs::read_to_string(path).ok()?;
    match contents.trim() {
        "light" => Some(Theme::Light),
        "dark" => Some(Theme::Dark),
        _ => None,
    }
}

pub fn save(path: &Path, theme: Theme) -> std::io::Result<()> {
    let value = match theme {
        Theme::Light => "light",
        Theme::Dark => "dark",
    };
    std::fs::write(path, value)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path(label: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "memonbooks_theme_preference_test_{label}_{:?}",
            std::thread::current().id()
        ))
    }

    #[test]
    fn load_returns_none_when_no_file_exists() {
        let path = temp_path("missing");

        assert_eq!(load(&path), None);
    }

    #[test]
    fn save_then_load_round_trips_light() {
        let path = temp_path("light");

        save(&path, Theme::Light).unwrap();

        assert_eq!(load(&path), Some(Theme::Light));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn save_then_load_round_trips_dark() {
        let path = temp_path("dark");

        save(&path, Theme::Dark).unwrap();

        assert_eq!(load(&path), Some(Theme::Dark));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn saving_a_new_choice_overwrites_the_previous_one() {
        let path = temp_path("overwrite");

        save(&path, Theme::Light).unwrap();
        save(&path, Theme::Dark).unwrap();

        assert_eq!(load(&path), Some(Theme::Dark));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn load_returns_none_for_unrecognized_contents() {
        let path = temp_path("garbage");
        std::fs::write(&path, "not-a-theme").unwrap();

        assert_eq!(load(&path), None);
        let _ = std::fs::remove_file(&path);
    }
}
