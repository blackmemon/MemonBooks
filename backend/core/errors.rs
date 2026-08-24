//! Central application error type. Every layer (database, repositories,
//! services, views) converges on `AppError` so callers deal with one
//! error type instead of threading crate-specific errors upward.

use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("migration error: {0}")]
    Migration(String),

    #[error("I/O error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("configuration error: {0}")]
    Config(String),

    #[error("validation error: {0}")]
    Validation(String),

    #[error("not found: {0}")]
    NotFound(String),

    #[error("internal error: {0}")]
    Internal(String),
}

impl AppError {
    /// True when this is a SQLite constraint violation -- most commonly
    /// a foreign-key violation from deleting a record something else
    /// still references. Repositories match on this to turn a raw
    /// SQLite failure into a clear, domain-specific message instead of
    /// leaking the SQLite error text (e.g. "FOREIGN KEY constraint
    /// failed") up to a view.
    pub fn is_constraint_violation(&self) -> bool {
        matches!(self, AppError::Database(rusqlite::Error::SqliteFailure(err, _)) if err.code == rusqlite::ErrorCode::ConstraintViolation)
    }
}

pub type AppResult<T> = Result<T, AppError>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    fn a_real_rusqlite_error() -> rusqlite::Error {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute("SELECT * FROM no_such_table", []).unwrap_err()
    }

    #[test]
    fn database_error_from_rusqlite_converts_via_the_question_mark_operator() {
        fn fails() -> AppResult<()> {
            let conn = rusqlite::Connection::open_in_memory().unwrap();
            conn.execute("SELECT * FROM no_such_table", [])?;
            Ok(())
        }

        let err = fails().unwrap_err();
        assert!(matches!(err, AppError::Database(_)));
        assert!(err.to_string().starts_with("database error: "));
    }

    #[test]
    fn database_error_display_includes_the_underlying_rusqlite_message() {
        let rusqlite_err = a_real_rusqlite_error();
        let inner_message = rusqlite_err.to_string();
        let err = AppError::Database(rusqlite_err);

        assert_eq!(err.to_string(), format!("database error: {inner_message}"));
    }

    #[test]
    fn migration_error_display_wraps_the_message() {
        let err = AppError::Migration("column already exists".into());
        assert_eq!(err.to_string(), "migration error: column already exists");
    }

    #[test]
    fn io_error_display_includes_the_path_and_preserves_the_source_for_the_error_chain() {
        let source = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err = AppError::Io { path: PathBuf::from("/tmp/backup.db"), source };

        assert_eq!(err.to_string(), "I/O error at /tmp/backup.db: file not found");
        let chained_source = err.source().expect("Io variant must preserve its source for the error chain");
        assert_eq!(chained_source.to_string(), "file not found");
    }

    #[test]
    fn config_error_display_wraps_the_message() {
        let err = AppError::Config("missing DATABASE_PATH".into());
        assert_eq!(err.to_string(), "configuration error: missing DATABASE_PATH");
    }

    #[test]
    fn validation_error_display_wraps_the_message() {
        let err = AppError::Validation("amount must be greater than zero".into());
        assert_eq!(err.to_string(), "validation error: amount must be greater than zero");
    }

    #[test]
    fn not_found_error_display_wraps_the_message() {
        let err = AppError::NotFound("account 42 not found".into());
        assert_eq!(err.to_string(), "not found: account 42 not found");
    }

    #[test]
    fn internal_error_display_wraps_the_message() {
        let err = AppError::Internal("unreachable branch reached".into());
        assert_eq!(err.to_string(), "internal error: unreachable branch reached");
    }

    #[test]
    fn database_variant_preserves_the_rusqlite_error_as_its_source() {
        let err = AppError::Database(a_real_rusqlite_error());
        assert!(err.source().is_some());
    }

    #[test]
    fn is_constraint_violation_true_for_a_real_foreign_key_failure() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", true).unwrap();
        conn.execute_batch(
            "CREATE TABLE parents (id INTEGER PRIMARY KEY);
             CREATE TABLE children (id INTEGER PRIMARY KEY, parent_id INTEGER NOT NULL REFERENCES parents(id));
             INSERT INTO parents (id) VALUES (1);
             INSERT INTO children (id, parent_id) VALUES (1, 1);",
        )
        .unwrap();

        let rusqlite_err = conn.execute("DELETE FROM parents WHERE id = 1", []).unwrap_err();
        let err = AppError::Database(rusqlite_err);

        assert!(err.is_constraint_violation());
    }

    #[test]
    fn is_constraint_violation_false_for_an_unrelated_database_error() {
        let err = AppError::Database(a_real_rusqlite_error());
        assert!(!err.is_constraint_violation());
    }

    #[test]
    fn is_constraint_violation_false_for_non_database_variants() {
        assert!(!AppError::NotFound("x".into()).is_constraint_violation());
        assert!(!AppError::Validation("x".into()).is_constraint_violation());
    }
}
