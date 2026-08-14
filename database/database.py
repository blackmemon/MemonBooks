"""SQLAlchemy engine/session setup for the 2-database SQLCipher scheme.

master.db — device registration, license/activation, global config.
user.db   — everything else (masters, transactions, HR). Its engine can only
            be created after a user has authenticated, since the encryption
            key is derived from the user's identity (security.keys.derive_user_key).
"""
from __future__ import annotations

from contextlib import contextmanager
from typing import Iterator, Optional

import sqlcipher3.dbapi2 as sqlcipher
from sqlalchemy import create_engine
from sqlalchemy.engine import Engine
from sqlalchemy.orm import DeclarativeBase, Session, sessionmaker

from config import MASTER_DB_PATH, USER_DB_PATH
from security.keys import derive_master_key, derive_user_key


class MasterBase(DeclarativeBase):
    pass


class UserBase(DeclarativeBase):
    pass


def _make_engine(db_path, hex_key: str) -> Engine:
    def creator():
        con = sqlcipher.connect(str(db_path))
        con.execute(f"PRAGMA key=\"x'{hex_key}'\"")
        con.execute("PRAGMA foreign_keys=ON")
        return con

    return create_engine("sqlite://", creator=creator, future=True)


_master_engine: Optional[Engine] = None
_master_session_factory: Optional[sessionmaker] = None

_user_engine: Optional[Engine] = None
_user_session_factory: Optional[sessionmaker] = None


def get_master_engine() -> Engine:
    global _master_engine, _master_session_factory
    if _master_engine is None:
        _master_engine = _make_engine(MASTER_DB_PATH, derive_master_key())
        _master_session_factory = sessionmaker(bind=_master_engine, future=True, expire_on_commit=False)
    return _master_engine


@contextmanager
def get_master_session() -> Iterator[Session]:
    get_master_engine()
    assert _master_session_factory is not None
    session = _master_session_factory()
    try:
        yield session
        session.commit()
    except Exception:
        session.rollback()
        raise
    finally:
        session.close()


def open_user_database() -> None:
    """Open user.db using the fixed installation key. Safe to call at app
    startup, before any staff member has logged in — see security.keys
    module docstring for why the key does not depend on who logs in."""
    global _user_engine, _user_session_factory
    if _user_session_factory is not None:
        return
    _user_engine = _make_engine(USER_DB_PATH, derive_user_key())
    _user_session_factory = sessionmaker(bind=_user_engine, future=True, expire_on_commit=False)


def close_user_database() -> None:
    global _user_engine, _user_session_factory
    if _user_engine is not None:
        _user_engine.dispose()
    _user_engine = None
    _user_session_factory = None


def is_user_database_open() -> bool:
    return _user_session_factory is not None


def get_user_engine() -> Engine:
    if _user_engine is None:
        raise RuntimeError("user.db is not open yet — call open_user_database() first.")
    return _user_engine


@contextmanager
def get_user_session() -> Iterator[Session]:
    if _user_session_factory is None:
        raise RuntimeError("user.db is not open yet — call open_user_database() after login.")
    session = _user_session_factory()
    try:
        yield session
        session.commit()
    except Exception:
        session.rollback()
        raise
    finally:
        session.close()
