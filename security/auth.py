"""Staff login: bcrypt password check against UserAccount rows in user.db.
Does not affect database encryption keys — see security/keys.py docstring."""
from __future__ import annotations

from dataclasses import dataclass
from typing import Optional

import bcrypt
from sqlalchemy import select

from database.database import get_user_session
from backend.models.system_models import UserAccount


@dataclass(frozen=True)
class Session:
    username: str
    full_name: str
    role: str


_current_session: Optional[Session] = None


def hash_password(plain: str) -> str:
    return bcrypt.hashpw(plain.encode("utf-8"), bcrypt.gensalt()).decode("utf-8")


def login(username: str, password: str) -> Optional[Session]:
    global _current_session
    with get_user_session() as db:
        user = db.execute(
            select(UserAccount).where(UserAccount.username == username.strip())
        ).scalar_one_or_none()
        if user is None or not user.is_active:
            return None
        if not bcrypt.checkpw(password.encode("utf-8"), user.password_hash.encode("utf-8")):
            return None
        _current_session = Session(username=user.username, full_name=user.full_name, role=user.role)
        return _current_session


def logout() -> None:
    global _current_session
    _current_session = None


def current_session() -> Optional[Session]:
    return _current_session


def current_username() -> str:
    return _current_session.username if _current_session else "system"
