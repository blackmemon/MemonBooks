//! SQLite connection handling and the schema migration runner.
//!
//! Migrations are plain `.sql` files under `database/migrations/`, embedded
//! into the binary with `include_str!` (not read from disk at runtime) so
//! a packaged/sandboxed build -- Mac App Store, MSIX, Flatpak -- still has
//! them available. Each module that owns schema contributes its own
//! `Migration` entries; `Database::run_migrations` applies whichever of
//! them haven't been recorded yet, in `version` order, each in its own
//! transaction.

use std::path::Path;
use std::sync::{Arc, Mutex};

use rusqlite::Connection;

use crate::backend::core::errors::{AppError, AppResult};

#[derive(Debug, Clone, Copy)]
pub struct Migration {
    pub version: i64,
    pub name: &'static str,
    pub sql: &'static str,
}

#[derive(Clone)]
pub struct Database {
    conn: Arc<Mutex<Connection>>,
}

impl Database {
    pub fn open(path: &Path) -> AppResult<Self> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(|source| AppError::Io {
                    path: parent.to_path_buf(),
                    source,
                })?;
            }
        }

        let conn = Connection::open(path)?;
        Self::from_connection(conn)
    }

    #[cfg(test)]
    pub fn open_in_memory() -> AppResult<Self> {
        Self::from_connection(Connection::open_in_memory()?)
    }

    fn from_connection(conn: Connection) -> AppResult<Self> {
        conn.pragma_update(None, "foreign_keys", true)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_migrations (
                version     INTEGER PRIMARY KEY,
                name        TEXT NOT NULL,
                applied_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
            );",
        )?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Runs every migration in `migrations` whose version isn't already
    /// recorded in `schema_migrations`, in ascending version order. Each
    /// migration runs in its own transaction; the version is recorded only
    /// once that migration's SQL has applied cleanly.
    pub fn run_migrations(&self, migrations: &[Migration]) -> AppResult<()> {
        let mut sorted: Vec<&Migration> = migrations.iter().collect();
        sorted.sort_by_key(|m| m.version);

        let mut conn = self.conn.lock().map_err(|_| {
            AppError::Internal("database connection mutex was poisoned".into())
        })?;

        for migration in sorted {
            let already_applied: bool = conn.query_row(
                "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version = ?1)",
                [migration.version],
                |row| row.get(0),
            )?;
            if already_applied {
                continue;
            }

            let tx = conn.transaction()?;
            tx.execute_batch(migration.sql).map_err(|err| {
                AppError::Migration(format!(
                    "migration {} ({}) failed: {err}",
                    migration.version, migration.name
                ))
            })?;
            tx.execute(
                "INSERT INTO schema_migrations (version, name) VALUES (?1, ?2)",
                (migration.version, migration.name),
            )?;
            tx.commit()?;
        }

        Ok(())
    }

    /// Runs `f` with exclusive access to the underlying connection.
    /// Repositories use this instead of holding the lock themselves.
    pub fn with_connection<T>(&self, f: impl FnOnce(&Connection) -> AppResult<T>) -> AppResult<T> {
        let conn = self.conn.lock().map_err(|_| {
            AppError::Internal("database connection mutex was poisoned".into())
        })?;
        f(&conn)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CREATE_WIDGETS: Migration = Migration {
        version: 1,
        name: "create_widgets",
        sql: "CREATE TABLE widgets (id INTEGER PRIMARY KEY, name TEXT NOT NULL);",
    };

    const ADD_WIDGET_COLOR: Migration = Migration {
        version: 2,
        name: "add_widget_color",
        sql: "ALTER TABLE widgets ADD COLUMN color TEXT;",
    };

    #[test]
    fn fresh_database_applies_all_migrations_in_order() {
        let db = Database::open_in_memory().unwrap();
        db.run_migrations(&[ADD_WIDGET_COLOR, CREATE_WIDGETS]).unwrap();

        db.with_connection(|conn| {
            conn.execute(
                "INSERT INTO widgets (name, color) VALUES ('gizmo', 'red')",
                [],
            )
            .unwrap();
            Ok(())
        })
        .unwrap();

        let count: i64 = db
            .with_connection(|conn| Ok(conn.query_row("SELECT COUNT(*) FROM widgets", [], |r| r.get(0))?))
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn already_applied_migrations_are_not_reapplied() {
        let db = Database::open_in_memory().unwrap();
        db.run_migrations(&[CREATE_WIDGETS]).unwrap();
        // Re-running the same migration set must be a no-op, not a
        // "table already exists" error.
        db.run_migrations(&[CREATE_WIDGETS]).unwrap();
        db.run_migrations(&[CREATE_WIDGETS, ADD_WIDGET_COLOR]).unwrap();
    }

    #[test]
    fn invalid_migration_sql_surfaces_as_migration_error() {
        let db = Database::open_in_memory().unwrap();
        let broken = Migration {
            version: 1,
            name: "broken",
            sql: "THIS IS NOT VALID SQL;",
        };

        let result = db.run_migrations(&[broken]);
        assert!(matches!(result, Err(AppError::Migration(_))));
    }
}
