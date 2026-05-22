"""
connection.py — SQLCipher database connection manager for master.db and user.db.

Security rules:
  - master.db absent  → both DBs refuse to open
  - wrong DEVICE_HASH → key derivation produces wrong key → SQLCipher rejects open
  - user.db key includes USER_HASH → password change = user.db re-key required
  - All connections are thread-local to prevent cross-request sharing
"""

import sqlite3
import threading
from contextlib import contextmanager
from pathlib import Path
from typing import Generator, Optional

from backend.security.device_fingerprint import get_device_hash
from backend.security.encryption import (
    derive_master_db_key,
    derive_user_db_key,
    sqlcipher_pragma_key,
)

try:
    from sqlcipher3 import dbapi2 as sqlcipher
except ImportError:
    # Fallback for dev environments without SQLCipher — plain SQLite (no encryption)
    import sqlite3 as sqlcipher  # type: ignore
    _SQLCIPHER_AVAILABLE = False
else:
    _SQLCIPHER_AVAILABLE = True

# ──────────────────────────────────────────────────────────────────────────────
# Thread-local storage for connections
# ──────────────────────────────────────────────────────────────────────────────

_thread_local = threading.local()

_MASTER_DB_KEY = "master_db_conn"
_USER_DB_KEY = "user_db_conn"


# ──────────────────────────────────────────────────────────────────────────────
# Internal helpers
# ──────────────────────────────────────────────────────────────────────────────

def _get_db_paths() -> tuple[Path, Path]:
    """Return absolute paths to master.db and user.db."""
    from backend.config import settings
    db_dir = Path(settings.DATA_DIR) / "db"
    return db_dir / "master.db", db_dir / "user.db"


def _open_sqlcipher(db_path: Path, hex_key: str) -> sqlcipher.Connection:
    """
    Open a SQLCipher-encrypted database file.
    Raises RuntimeError if the key is wrong or file is corrupt.
    """
    conn = sqlcipher.connect(str(db_path), check_same_thread=False)

    if _SQLCIPHER_AVAILABLE:
        # Apply key before any SQL
        conn.execute(f"PRAGMA key = \"x'{hex_key}'\";")
        # Verify the key is correct by attempting a read
        try:
            conn.execute("SELECT count(*) FROM sqlite_master;").fetchone()
        except Exception:
            conn.close()
            raise RuntimeError(
                f"Failed to open {db_path.name}: wrong key or corrupted database."
            )

    conn.execute("PRAGMA journal_mode=WAL;")
    conn.execute("PRAGMA foreign_keys=ON;")
    conn.execute("PRAGMA busy_timeout=5000;")
    conn.row_factory = sqlite3.Row
    return conn


# ──────────────────────────────────────────────────────────────────────────────
# Master DB
# ──────────────────────────────────────────────────────────────────────────────

def open_master_db(app_secret: str, db_salt: str) -> sqlcipher.Connection:
    """
    Open master.db with the derived key. Called once at application startup.
    Stores connection in thread-local storage.
    Raises FileNotFoundError if master.db does not exist.
    """
    master_path, _ = _get_db_paths()

    if not master_path.exists():
        raise FileNotFoundError(
            "master.db not found. Run scripts/setup_db.py to initialize the database."
        )

    device_hash = get_device_hash()
    hex_key = derive_master_db_key(device_hash, app_secret, db_salt)
    conn = _open_sqlcipher(master_path, hex_key)
    _thread_local.__dict__[_MASTER_DB_KEY] = conn
    return conn


def get_master_conn() -> sqlcipher.Connection:
    """Return the current thread's master.db connection. Raises if not opened."""
    conn = _thread_local.__dict__.get(_MASTER_DB_KEY)
    if conn is None:
        raise RuntimeError("master.db connection not initialized. Call open_master_db() first.")
    return conn


def close_master_db() -> None:
    """Close master.db connection for the current thread."""
    conn = _thread_local.__dict__.pop(_MASTER_DB_KEY, None)
    if conn:
        conn.close()


# ──────────────────────────────────────────────────────────────────────────────
# User DB
# ──────────────────────────────────────────────────────────────────────────────

def open_user_db(
    secret_token: str,
    user_hash: str,
    db_salt: str,
) -> Optional[sqlcipher.Connection]:
    """
    Open user.db with the derived key.
    user.db is optional — admin-only mode works without it.
    Returns None if user.db does not exist.
    Raises RuntimeError if user.db exists but key is wrong.
    """
    master_path, user_path = _get_db_paths()

    if not master_path.exists():
        raise RuntimeError("master.db is absent — cannot open user.db.")

    if not user_path.exists():
        return None

    device_hash = get_device_hash()
    hex_key = derive_user_db_key(device_hash, secret_token, user_hash, db_salt)
    conn = _open_sqlcipher(user_path, hex_key)
    _thread_local.__dict__[_USER_DB_KEY] = conn
    return conn


def get_user_conn() -> Optional[sqlcipher.Connection]:
    """Return the current thread's user.db connection. None if not opened."""
    return _thread_local.__dict__.get(_USER_DB_KEY)


def close_user_db() -> None:
    """Close user.db connection for the current thread."""
    conn = _thread_local.__dict__.pop(_USER_DB_KEY, None)
    if conn:
        conn.close()


# ──────────────────────────────────────────────────────────────────────────────
# Context managers for FastAPI dependency injection
# ──────────────────────────────────────────────────────────────────────────────

@contextmanager
def master_db_session() -> Generator[sqlcipher.Connection, None, None]:
    """
    Context manager for master.db access.
    Usage: with master_db_session() as conn: ...
    """
    conn = get_master_conn()
    try:
        yield conn
        conn.commit()
    except Exception:
        conn.rollback()
        raise


@contextmanager
def user_db_session() -> Generator[Optional[sqlcipher.Connection], None, None]:
    """
    Context manager for user.db access.
    Yields None if user.db is not available.
    """
    conn = get_user_conn()
    try:
        yield conn
        if conn:
            conn.commit()
    except Exception:
        if conn:
            conn.rollback()
        raise


# ──────────────────────────────────────────────────────────────────────────────
# FastAPI dependency functions
# ──────────────────────────────────────────────────────────────────────────────

def get_master_db() -> Generator[sqlcipher.Connection, None, None]:
    """FastAPI dependency: yields master.db connection."""
    with master_db_session() as conn:
        yield conn


def get_user_db() -> Generator[Optional[sqlcipher.Connection], None, None]:
    """FastAPI dependency: yields user.db connection (None if unavailable)."""
    with user_db_session() as conn:
        yield conn


# ──────────────────────────────────────────────────────────────────────────────
# Rekey utilities (called after password change or security rotation)
# ──────────────────────────────────────────────────────────────────────────────

def rekey_user_db(
    old_secret_token: str,
    old_user_hash: str,
    new_secret_token: str,
    new_user_hash: str,
    db_salt: str,
) -> None:
    """
    Re-encrypt user.db with a new key after a password change.
    Both old and new keys are required for safe rotation.
    """
    _, user_path = _get_db_paths()
    if not user_path.exists():
        raise FileNotFoundError("user.db not found.")

    device_hash = get_device_hash()
    old_key = derive_user_db_key(device_hash, old_secret_token, old_user_hash, db_salt)
    new_key = derive_user_db_key(device_hash, new_secret_token, new_user_hash, db_salt)

    conn = _open_sqlcipher(user_path, old_key)
    try:
        if _SQLCIPHER_AVAILABLE:
            conn.execute(f"PRAGMA rekey = \"x'{new_key}'\";")
        conn.commit()
    finally:
        conn.close()
