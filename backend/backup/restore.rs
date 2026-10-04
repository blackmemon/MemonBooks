//! Restoring from a backup. Deliberately does *not* hot-swap the live,
//! already-open database connection -- it validates the backup file and
//! copies it over the target path, on the assumption the app restarts
//! afterward (the composition root, `src/main.rs`, opens a fresh
//! connection to whatever's at that path on the next launch).

use std::path::Path;

use rusqlite::Connection;

use crate::backend::core::errors::{AppError, AppResult};

/// Confirms `backup_path` looks like a MemonBooks database (opens, and
/// has the migrations tracking table this app's `Database::open` always
/// creates) before agreeing to overwrite anything with it.
fn looks_like_a_memonbooks_backup(backup_path: &Path) -> AppResult<bool> {
    let conn = Connection::open(backup_path)?;
    let exists: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'schema_migrations')",
        [],
        |row| row.get(0),
    )?;
    Ok(exists)
}

pub fn restore_from_backup(backup_path: &Path, target_db_path: &Path) -> AppResult<()> {
    if !backup_path.is_file() {
        return Err(AppError::Validation(format!(
            "backup file {} does not exist",
            backup_path.display()
        )));
    }
    if !looks_like_a_memonbooks_backup(backup_path)? {
        return Err(AppError::Validation(
            "this file does not look like a MemonBooks database backup".into(),
        ));
    }

    if let Some(parent) = target_db_path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|source| AppError::Io {
                path: parent.to_path_buf(),
                source,
            })?;
        }
    }

    std::fs::copy(backup_path, target_db_path).map_err(|source| AppError::Io {
        path: target_db_path.to_path_buf(),
        source,
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::database::{Database, Migration};

    fn temp_dir(label: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "memonbooks-restore-test-{label}-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn restoring_a_valid_backup_copies_it_to_the_target_path() {
        let dir = temp_dir("valid");
        let backup_path = dir.join("backup.sqlite3");
        let db = Database::open(&backup_path).unwrap();
        db.run_migrations(&[Migration {
            version: 1,
            name: "noop",
            sql: "CREATE TABLE t (id INTEGER PRIMARY KEY);",
        }])
        .unwrap();
        drop(db);

        let target_path = dir.join("restored.sqlite3");
        restore_from_backup(&backup_path, &target_path).unwrap();
        assert!(target_path.exists());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn restoring_a_file_without_our_schema_is_rejected() {
        let dir = temp_dir("invalid");
        let not_a_backup = dir.join("not-a-backup.sqlite3");
        Connection::open(&not_a_backup)
            .unwrap()
            .execute("CREATE TABLE unrelated (id INTEGER);", [])
            .unwrap();

        let target_path = dir.join("restored.sqlite3");
        let result = restore_from_backup(&not_a_backup, &target_path);
        assert!(result.is_err());
        assert!(!target_path.exists());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn restoring_a_missing_file_is_rejected() {
        let dir = temp_dir("missing");
        let result = restore_from_backup(
            &dir.join("does-not-exist.sqlite3"),
            &dir.join("restored.sqlite3"),
        );
        assert!(result.is_err());
        std::fs::remove_dir_all(&dir).ok();
    }
}
