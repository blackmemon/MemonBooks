//! Creating a backup: `VACUUM INTO` gives a consistent, defragmented
//! snapshot of the live database in one statement, safe to run while the
//! app has the database open (unlike a plain file copy, which could
//! catch a write mid-flight).

use std::path::Path;

use crate::backend::core::errors::{AppError, AppResult};
use crate::database::database::Database;

pub fn create_backup(db: &Database, destination: &Path) -> AppResult<()> {
    if let Some(parent) = destination.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|source| AppError::Io {
                path: parent.to_path_buf(),
                source,
            })?;
        }
    }

    let destination_str = destination.to_str().ok_or_else(|| AppError::Validation("backup destination path is not valid UTF-8".into()))?;

    db.with_connection(|conn| {
        conn.execute("VACUUM INTO ?1", [destination_str])?;
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_a_backup_file_with_the_current_schema() {
        let db = Database::open_in_memory().unwrap();
        db.run_migrations(&[crate::database::database::Migration {
            version: 1,
            name: "widgets",
            sql: "CREATE TABLE widgets (id INTEGER PRIMARY KEY, name TEXT NOT NULL);",
        }])
        .unwrap();
        db.with_connection(|conn| {
            conn.execute("INSERT INTO widgets (name) VALUES ('gizmo')", [])?;
            Ok(())
        })
        .unwrap();

        let dir = std::env::temp_dir().join(format!("memonbooks-backup-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let backup_path = dir.join("backup.sqlite3");

        create_backup(&db, &backup_path).unwrap();
        assert!(backup_path.exists());

        let restored = rusqlite::Connection::open(&backup_path).unwrap();
        let count: i64 = restored.query_row("SELECT COUNT(*) FROM widgets", [], |row| row.get(0)).unwrap();
        assert_eq!(count, 1);

        std::fs::remove_dir_all(&dir).ok();
    }
}
