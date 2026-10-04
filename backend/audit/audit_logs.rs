//! Write-only security/audit event log. This is infrastructure, like
//! `core::logging` -- not gated behind its own Development Order step --
//! because Architecture Rules (Section 3) require security events,
//! postings, reversals, and configuration changes to be audited starting
//! wherever they first happen. `user_id` intentionally has no foreign key:
//! the log must stay readable even if the referenced user is later
//! deleted or deactivated.

use rusqlite::params;

use crate::backend::core::errors::AppResult;
use crate::database::database::{Database, Migration};

pub const MIGRATIONS: &[Migration] = &[Migration {
    version: 401,
    name: "audit_logs_schema",
    sql: "CREATE TABLE audit_logs (
        id          INTEGER PRIMARY KEY AUTOINCREMENT,
        user_id     INTEGER,
        action      TEXT NOT NULL,
        entity_type TEXT,
        entity_id   TEXT,
        detail      TEXT,
        occurred_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
    );",
}];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditLog {
    pub id: Option<i64>,
    pub user_id: Option<i64>,
    pub action: String,
    pub entity_type: Option<String>,
    pub entity_id: Option<String>,
    pub detail: Option<String>,
}

impl AuditLog {
    pub fn new(action: impl Into<String>) -> Self {
        Self {
            id: None,
            user_id: None,
            action: action.into(),
            entity_type: None,
            entity_id: None,
            detail: None,
        }
    }
}

/// Takes a `&Connection` (not `&Database`) specifically so callers already
/// holding an open `rusqlite::Transaction` -- e.g. `AccountingRepository`
/// posting a ledger transaction -- can record the audit entry as part of
/// that same atomic write (a `Transaction` derefs to `Connection`).
pub fn record(conn: &rusqlite::Connection, entry: &AuditLog) -> AppResult<()> {
    conn.execute(
        "INSERT INTO audit_logs (user_id, action, entity_type, entity_id, detail) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![entry.user_id, entry.action, entry.entity_type, entry.entity_id, entry.detail],
    )?;
    Ok(())
}

pub fn list_recent(db: &Database, limit: i64) -> AppResult<Vec<AuditLog>> {
    db.with_connection(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, user_id, action, entity_type, entity_id, detail FROM audit_logs
             ORDER BY occurred_at DESC, id DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit], |row| {
            Ok(AuditLog {
                id: Some(row.get(0)?),
                user_id: row.get(1)?,
                action: row.get(2)?,
                entity_type: row.get(3)?,
                entity_id: row.get(4)?,
                detail: row.get(5)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(crate::backend::core::errors::AppError::from)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn db_with_schema() -> Database {
        let db = Database::open_in_memory().unwrap();
        db.run_migrations(MIGRATIONS).unwrap();
        db
    }

    #[test]
    fn records_and_lists_events_most_recent_first() {
        let db = db_with_schema();

        let mut login = AuditLog::new("auth.login_succeeded");
        login.user_id = Some(1);
        db.with_connection(|conn| record(conn, &login)).unwrap();

        let mut role_change = AuditLog::new("roles.updated");
        role_change.entity_type = Some("role".into());
        role_change.entity_id = Some("2".into());
        db.with_connection(|conn| record(conn, &role_change))
            .unwrap();

        let recent = list_recent(&db, 10).unwrap();
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].action, "roles.updated");
        assert_eq!(recent[1].action, "auth.login_succeeded");
    }

    #[test]
    fn list_recent_respects_the_limit() {
        let db = db_with_schema();
        for i in 0..5 {
            db.with_connection(|conn| record(conn, &AuditLog::new(format!("event.{i}"))))
                .unwrap();
        }
        assert_eq!(list_recent(&db, 2).unwrap().len(), 2);
    }
}
