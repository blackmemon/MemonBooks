//! Persistence for roles (+ their permissions), users, and sessions.

use chrono::{DateTime, Utc};
use rusqlite::{params, OptionalExtension, Row};

use crate::backend::core::auth::sessions::Session;
use crate::backend::core::errors::{AppError, AppResult};
use crate::backend::core::roles::Role;
use crate::backend::core::users::User;
use crate::backend::core::validation::Validate;
use crate::database::database::{Database, Migration};

pub const MIGRATIONS: &[Migration] = &[Migration {
    version: 400,
    name: "users_roles_sessions_schema",
    sql: "
        CREATE TABLE roles (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            name        TEXT NOT NULL UNIQUE,
            description TEXT
        );

        CREATE TABLE role_permissions (
            role_id         INTEGER NOT NULL REFERENCES roles(id),
            permission_key  TEXT NOT NULL,
            PRIMARY KEY (role_id, permission_key)
        );

        CREATE TABLE users (
            id                  INTEGER PRIMARY KEY AUTOINCREMENT,
            username            TEXT NOT NULL UNIQUE,
            display_name        TEXT NOT NULL,
            password_hash       TEXT NOT NULL,
            role_id             INTEGER NOT NULL REFERENCES roles(id),
            is_active           INTEGER NOT NULL DEFAULT 1,
            two_factor_enabled  INTEGER NOT NULL DEFAULT 0,
            two_factor_secret   TEXT,
            created_at          TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
        );

        CREATE TABLE sessions (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id     INTEGER NOT NULL REFERENCES users(id),
            token       TEXT NOT NULL UNIQUE,
            created_at  TEXT NOT NULL,
            expires_at  TEXT NOT NULL,
            revoked_at  TEXT
        );
    ",
}, Migration {
    // 401 is already taken by backend::audit::audit_logs -- migration
    // versions are a single global namespace (schema_migrations' primary
    // key), not scoped per file, so every new migration anywhere in the
    // codebase has to be checked against all the others, not just this
    // file's own.
    version: 402,
    name: "users_must_change_password",
    sql: "ALTER TABLE users ADD COLUMN must_change_password INTEGER NOT NULL DEFAULT 0;",
}];

#[derive(Clone)]
pub struct UserRepository {
    db: Database,
}

impl UserRepository {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    // -- roles ----------------------------------------------------------

    /// Inserts the role and its permission grants as one atomic write.
    pub fn create_role(&self, role: &Role) -> AppResult<Role> {
        role.validate()?;
        self.db.with_transaction(|tx| {
            tx.execute(
                "INSERT INTO roles (name, description) VALUES (?1, ?2)",
                params![role.name, role.description],
            )?;
            let role_id = tx.last_insert_rowid();
            for permission in &role.permissions {
                tx.execute(
                    "INSERT INTO role_permissions (role_id, permission_key) VALUES (?1, ?2)",
                    params![role_id, permission],
                )?;
            }
            Ok(Role {
                id: Some(role_id),
                ..role.clone()
            })
        })
    }

    pub fn get_role(&self, id: i64) -> AppResult<Role> {
        self.db.with_connection(|conn| {
            let mut role = conn
                .query_row(
                    "SELECT id, name, description FROM roles WHERE id = ?1",
                    params![id],
                    map_role,
                )
                .optional()?
                .ok_or_else(|| AppError::NotFound(format!("role {id} not found")))?;

            let mut stmt = conn.prepare("SELECT permission_key FROM role_permissions WHERE role_id = ?1")?;
            let permissions = stmt.query_map(params![id], |row| row.get::<_, String>(0))?;
            role.permissions = permissions.collect::<Result<Vec<_>, _>>()?;
            Ok(role)
        })
    }

    pub fn list_roles(&self) -> AppResult<Vec<Role>> {
        let ids: Vec<i64> = self.db.with_connection(|conn| {
            let mut stmt = conn.prepare("SELECT id FROM roles ORDER BY name")?;
            let rows = stmt.query_map([], |row| row.get(0))?;
            rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
        })?;
        ids.into_iter().map(|id| self.get_role(id)).collect()
    }

    /// Replaces the role's name/description and its full permission set.
    pub fn update_role(&self, role: &Role) -> AppResult<()> {
        role.validate()?;
        let id = role
            .id
            .ok_or_else(|| AppError::Validation("cannot update a role without an id".into()))?;
        self.db.with_transaction(|tx| {
            let affected = tx.execute(
                "UPDATE roles SET name = ?1, description = ?2 WHERE id = ?3",
                params![role.name, role.description, id],
            )?;
            if affected == 0 {
                return Err(AppError::NotFound(format!("role {id} not found")));
            }
            tx.execute("DELETE FROM role_permissions WHERE role_id = ?1", params![id])?;
            for permission in &role.permissions {
                tx.execute(
                    "INSERT INTO role_permissions (role_id, permission_key) VALUES (?1, ?2)",
                    params![id, permission],
                )?;
            }
            Ok(())
        })
    }

    // -- users ------------------------------------------------------------

    pub fn create_user(&self, user: &User) -> AppResult<User> {
        user.validate()?;
        self.db.with_connection(|conn| {
            conn.execute(
                "INSERT INTO users (username, display_name, password_hash, role_id, is_active, two_factor_enabled, two_factor_secret, must_change_password)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    user.username,
                    user.display_name,
                    user.password_hash,
                    user.role_id,
                    user.is_active,
                    user.two_factor_enabled,
                    user.two_factor_secret,
                    user.must_change_password,
                ],
            )?;
            Ok(User {
                id: Some(conn.last_insert_rowid()),
                ..user.clone()
            })
        })
    }

    pub fn get_user(&self, id: i64) -> AppResult<User> {
        self.db.with_connection(|conn| {
            conn.query_row(
                "SELECT id, username, display_name, password_hash, role_id, is_active, two_factor_enabled, two_factor_secret, must_change_password
                 FROM users WHERE id = ?1",
                params![id],
                map_user,
            )
            .optional()?
            .ok_or_else(|| AppError::NotFound(format!("user {id} not found")))
        })
    }

    pub fn find_user_by_username(&self, username: &str) -> AppResult<Option<User>> {
        let username = username.to_ascii_lowercase();
        self.db.with_connection(|conn| {
            conn.query_row(
                "SELECT id, username, display_name, password_hash, role_id, is_active, two_factor_enabled, two_factor_secret, must_change_password
                 FROM users WHERE username = ?1",
                params![username],
                map_user,
            )
            .optional()
            .map_err(AppError::from)
        })
    }

    pub fn list_users(&self) -> AppResult<Vec<User>> {
        self.db.with_connection(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, username, display_name, password_hash, role_id, is_active, two_factor_enabled, two_factor_secret, must_change_password
                 FROM users ORDER BY username",
            )?;
            let rows = stmt.query_map([], map_user)?;
            rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
        })
    }

    pub fn update_user(&self, user: &User) -> AppResult<()> {
        user.validate()?;
        let id = user
            .id
            .ok_or_else(|| AppError::Validation("cannot update a user without an id".into()))?;
        self.db.with_connection(|conn| {
            let affected = conn.execute(
                "UPDATE users SET username = ?1, display_name = ?2, password_hash = ?3, role_id = ?4,
                 is_active = ?5, two_factor_enabled = ?6, two_factor_secret = ?7, must_change_password = ?8 WHERE id = ?9",
                params![
                    user.username,
                    user.display_name,
                    user.password_hash,
                    user.role_id,
                    user.is_active,
                    user.two_factor_enabled,
                    user.two_factor_secret,
                    user.must_change_password,
                    id,
                ],
            )?;
            if affected == 0 {
                return Err(AppError::NotFound(format!("user {id} not found")));
            }
            Ok(())
        })
    }

    /// Sets a new password hash and clears `must_change_password` in
    /// one update -- the operation `AuthService::change_password`
    /// drives, rather than requiring callers to round-trip a full
    /// `update_user` themselves.
    pub fn set_password(&self, user_id: i64, new_password_hash: &str) -> AppResult<()> {
        self.db.with_connection(|conn| {
            let affected = conn.execute(
                "UPDATE users SET password_hash = ?1, must_change_password = 0 WHERE id = ?2",
                params![new_password_hash, user_id],
            )?;
            if affected == 0 {
                return Err(AppError::NotFound(format!("user {user_id} not found")));
            }
            Ok(())
        })
    }

    // -- sessions -----------------------------------------------------

    pub fn create_session(&self, session: &Session) -> AppResult<Session> {
        self.db.with_connection(|conn| {
            conn.execute(
                "INSERT INTO sessions (user_id, token, created_at, expires_at, revoked_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![session.user_id, session.token, session.created_at, session.expires_at, session.revoked_at],
            )?;
            Ok(Session {
                id: Some(conn.last_insert_rowid()),
                ..session.clone()
            })
        })
    }

    pub fn find_session_by_token(&self, token: &str) -> AppResult<Option<Session>> {
        self.db.with_connection(|conn| {
            conn.query_row(
                "SELECT id, user_id, token, created_at, expires_at, revoked_at FROM sessions WHERE token = ?1",
                params![token],
                map_session,
            )
            .optional()
            .map_err(AppError::from)
        })
    }

    pub fn revoke_session(&self, id: i64, revoked_at: DateTime<Utc>) -> AppResult<()> {
        self.db.with_connection(|conn| {
            let affected = conn.execute(
                "UPDATE sessions SET revoked_at = ?1 WHERE id = ?2",
                params![revoked_at, id],
            )?;
            if affected == 0 {
                return Err(AppError::NotFound(format!("session {id} not found")));
            }
            Ok(())
        })
    }
}

fn map_role(row: &Row) -> rusqlite::Result<Role> {
    Ok(Role {
        id: Some(row.get(0)?),
        name: row.get(1)?,
        description: row.get(2)?,
        permissions: Vec::new(),
    })
}

fn map_user(row: &Row) -> rusqlite::Result<User> {
    Ok(User {
        id: Some(row.get(0)?),
        username: row.get(1)?,
        display_name: row.get(2)?,
        password_hash: row.get(3)?,
        role_id: row.get(4)?,
        is_active: row.get(5)?,
        two_factor_enabled: row.get(6)?,
        two_factor_secret: row.get(7)?,
        must_change_password: row.get(8)?,
    })
}

fn map_session(row: &Row) -> rusqlite::Result<Session> {
    Ok(Session {
        id: Some(row.get(0)?),
        user_id: row.get(1)?,
        token: row.get(2)?,
        created_at: row.get(3)?,
        expires_at: row.get(4)?,
        revoked_at: row.get(5)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo_with_schema() -> UserRepository {
        let db = Database::open_in_memory().unwrap();
        db.run_migrations(MIGRATIONS).unwrap();
        UserRepository::new(db)
    }

    fn admin_role() -> Role {
        let mut role = Role::new("Administrator");
        role.permissions.push("users.manage".into());
        role.permissions.push("roles.manage".into());
        role
    }

    #[test]
    fn creating_a_role_persists_its_permissions_atomically() {
        let repo = repo_with_schema();
        let created = repo.create_role(&admin_role()).unwrap();

        let fetched = repo.get_role(created.id.unwrap()).unwrap();
        assert_eq!(fetched.name, "Administrator");
        assert_eq!(fetched.permissions.len(), 2);
        assert!(fetched.has_permission("users.manage"));
    }

    #[test]
    fn updating_a_role_replaces_its_permission_set() {
        let repo = repo_with_schema();
        let mut role = repo.create_role(&admin_role()).unwrap();
        role.permissions = vec!["reports.view".into()];
        repo.update_role(&role).unwrap();

        let fetched = repo.get_role(role.id.unwrap()).unwrap();
        assert_eq!(fetched.permissions, vec!["reports.view".to_string()]);
    }

    #[test]
    fn creates_and_finds_a_user_by_username() {
        let repo = repo_with_schema();
        let role = repo.create_role(&admin_role()).unwrap();
        let user = repo
            .create_user(&User::new("alice", "Alice Smith", "hash", role.id.unwrap()))
            .unwrap();
        assert!(user.id.is_some());

        let found = repo.find_user_by_username("ALICE").unwrap().expect("user should exist");
        assert_eq!(found.id, user.id);
    }

    #[test]
    fn find_user_by_username_returns_none_for_unknown_username() {
        let repo = repo_with_schema();
        assert!(repo.find_user_by_username("nobody").unwrap().is_none());
    }

    #[test]
    fn username_uniqueness_is_enforced() {
        let repo = repo_with_schema();
        let role = repo.create_role(&admin_role()).unwrap();
        repo.create_user(&User::new("alice", "Alice Smith", "hash", role.id.unwrap()))
            .unwrap();

        let result = repo.create_user(&User::new("alice", "Someone Else", "hash2", role.id.unwrap()));
        assert!(matches!(result, Err(AppError::Database(_))));
    }

    #[test]
    fn session_round_trips_by_token_and_can_be_revoked() {
        let repo = repo_with_schema();
        let role = repo.create_role(&admin_role()).unwrap();
        let user = repo
            .create_user(&User::new("alice", "Alice Smith", "hash", role.id.unwrap()))
            .unwrap();

        let session = repo.create_session(&Session::new(user.id.unwrap())).unwrap();
        let found = repo
            .find_session_by_token(&session.token)
            .unwrap()
            .expect("session should exist");
        assert!(found.is_valid());

        repo.revoke_session(found.id.unwrap(), Utc::now()).unwrap();
        let revoked = repo.find_session_by_token(&session.token).unwrap().unwrap();
        assert!(!revoked.is_valid());
    }
}
