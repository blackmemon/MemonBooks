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

use rusqlite::{Connection, OptionalExtension};

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

    /// An ephemeral, in-memory database -- used by this crate's own unit
    /// tests, and by the integration tests under `tests/`, which run as
    /// a separate crate and so need this to not be `#[cfg(test)]`-gated.
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
    ///
    /// Migration versions are one global namespace (`schema_migrations`'
    /// primary key), not scoped per file or per call -- two distinct
    /// migrations claiming the same version is a bug, not something to
    /// resolve by silently keeping only one of them. This rejects that
    /// outright, whether the collision is within one `migrations` slice
    /// or across separate `run_migrations` calls on the same `Database`
    /// (each call only sees its own slice, so the second check happens
    /// against what's already recorded).
    pub fn run_migrations(&self, migrations: &[Migration]) -> AppResult<()> {
        let mut sorted: Vec<&Migration> = migrations.iter().collect();
        sorted.sort_by_key(|m| m.version);

        for pair in sorted.windows(2) {
            if pair[0].version == pair[1].version {
                return Err(AppError::Migration(format!(
                    "migration version {} is claimed by both \"{}\" and \"{}\" -- migration versions must be globally unique",
                    pair[0].version, pair[0].name, pair[1].name
                )));
            }
        }

        let mut conn = self.conn.lock().map_err(|_| {
            AppError::Internal("database connection mutex was poisoned".into())
        })?;

        for migration in sorted {
            let existing_name: Option<String> = conn
                .query_row(
                    "SELECT name FROM schema_migrations WHERE version = ?1",
                    [migration.version],
                    |row| row.get(0),
                )
                .optional()?;

            match existing_name {
                Some(name) if name == migration.name => continue,
                Some(name) => {
                    return Err(AppError::Migration(format!(
                        "migration version {} is already recorded as \"{name}\", but this run claims it as \"{}\" -- migration versions must be globally unique",
                        migration.version, migration.name
                    )));
                }
                None => {}
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
    pub fn with_connection<T>(&self, f: impl FnOnce(&mut Connection) -> AppResult<T>) -> AppResult<T> {
        let mut conn = self.conn.lock().map_err(|_| {
            AppError::Internal("database connection mutex was poisoned".into())
        })?;
        f(&mut conn)
    }

    /// Runs `f` inside a single SQLite transaction: commits if `f` returns
    /// `Ok`, rolls back if it returns `Err` (or panics). Repositories use
    /// this for any write that touches more than one table and must be
    /// all-or-nothing -- per Architecture Rules, Section 3: "multi-table
    /// financial posting must be atomic."
    pub fn with_transaction<T>(&self, f: impl FnOnce(&rusqlite::Transaction) -> AppResult<T>) -> AppResult<T> {
        self.with_connection(|conn| {
            let tx = conn.transaction()?;
            let result = f(&tx)?;
            tx.commit()?;
            Ok(result)
        })
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

    /// Two distinct migrations claiming the same version in one
    /// `migrations` slice must be rejected outright, not silently
    /// resolved by running whichever one sorts first.
    #[test]
    fn duplicate_version_within_one_call_is_rejected() {
        let db = Database::open_in_memory().unwrap();
        let first = Migration { version: 5, name: "first_claim", sql: "CREATE TABLE a (id INTEGER PRIMARY KEY);" };
        let second = Migration { version: 5, name: "second_claim", sql: "CREATE TABLE b (id INTEGER PRIMARY KEY);" };

        let result = db.run_migrations(&[first, second]);

        assert!(matches!(result, Err(AppError::Migration(_))));
        // Neither table should exist -- rejected before either applied.
        let table_count: i64 = db
            .with_connection(|conn| {
                Ok(conn.query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name IN ('a', 'b')",
                    [],
                    |row| row.get(0),
                )?)
            })
            .unwrap();
        assert_eq!(table_count, 0);
    }

    /// Reproduces the real incident this hardening is for: two
    /// separate modules' `MIGRATIONS` constants happened to both use
    /// version 401 (`backend::audit::audit_logs` and a new
    /// `user_repository` migration). Because each module calls
    /// `run_migrations` with only its own slice, the collision only
    /// shows up against what's already recorded in `schema_migrations`
    /// -- exactly this scenario, not the single-call one above.
    /// Previously this silently skipped the second migration's SQL
    /// entirely (its CREATE TABLE never ran) while still leaving the
    /// database looking like nothing had gone wrong.
    #[test]
    fn duplicate_version_across_separate_calls_is_rejected() {
        let db = Database::open_in_memory().unwrap();
        let audit_logs_migration = Migration { version: 401, name: "audit_log_schema", sql: "CREATE TABLE audit_logs (id INTEGER PRIMARY KEY);" };
        let unrelated_migration = Migration { version: 401, name: "users_must_change_password", sql: "CREATE TABLE should_never_run (id INTEGER PRIMARY KEY);" };

        db.run_migrations(&[audit_logs_migration]).unwrap();
        let result = db.run_migrations(&[unrelated_migration]);

        assert!(matches!(result, Err(AppError::Migration(_))));
        let ran: bool = db
            .with_connection(|conn| {
                Ok(conn.query_row(
                    "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'should_never_run')",
                    [],
                    |row| row.get(0),
                )?)
            })
            .unwrap();
        assert!(!ran, "the colliding migration's SQL must not have run");
    }
}
