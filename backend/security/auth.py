"""
auth.py — Authentication: login validation, JWT generation/verification,
two-database token embedding, device check, and session management.

Token payload carries:
  sub          — user_id (UUID)
  username     — login username
  role         — user role (admin/manager/user/viewer)
  device_hash  — hardware fingerprint at login time
  db           — "master" or "user" (which DB was used for this session)
  exp          — expiry timestamp
  iat          — issued-at timestamp
  jti          — unique token ID (for revocation)
"""

import uuid
from datetime import datetime, timedelta, timezone
from typing import Optional

import bcrypt
from jose import JWTError, jwt
from fastapi import Depends, HTTPException, status
from fastapi.security import OAuth2PasswordBearer

from backend.security.device_fingerprint import get_device_hash, verify_device_hash
from backend.security.encryption import derive_user_hash
from backend.database.connection import get_master_db, get_user_db

# ──────────────────────────────────────────────────────────────────────────────
# Constants
# ──────────────────────────────────────────────────────────────────────────────

_ALGORITHM = "HS256"
_ACCESS_TOKEN_EXPIRE_MINUTES = 480  # 8 hours
_REFRESH_TOKEN_EXPIRE_DAYS = 7

oauth2_scheme = OAuth2PasswordBearer(tokenUrl="/api/auth/login")


def _get_jwt_secret() -> str:
    from backend.config import settings
    return settings.JWT_SECRET


# ──────────────────────────────────────────────────────────────────────────────
# Password hashing
# ──────────────────────────────────────────────────────────────────────────────

def hash_password(plain_password: str) -> str:
    """Hash a password with bcrypt (work factor 12)."""
    salt = bcrypt.gensalt(rounds=12)
    return bcrypt.hashpw(plain_password.encode("utf-8"), salt).decode("utf-8")


def verify_password(plain_password: str, hashed_password: str) -> bool:
    """Verify a plain password against a bcrypt hash."""
    return bcrypt.checkpw(
        plain_password.encode("utf-8"),
        hashed_password.encode("utf-8"),
    )


# ──────────────────────────────────────────────────────────────────────────────
# JWT creation
# ──────────────────────────────────────────────────────────────────────────────

def create_access_token(
    user_id: str,
    username: str,
    role: str,
    device_hash: str,
    db_context: str,
    extra_claims: Optional[dict] = None,
) -> str:
    """Create a signed JWT access token."""
    now = datetime.now(timezone.utc)
    payload = {
        "sub": user_id,
        "username": username,
        "role": role,
        "device_hash": device_hash,
        "db": db_context,
        "jti": str(uuid.uuid4()),
        "iat": now,
        "exp": now + timedelta(minutes=_ACCESS_TOKEN_EXPIRE_MINUTES),
    }
    if extra_claims:
        payload.update(extra_claims)
    return jwt.encode(payload, _get_jwt_secret(), algorithm=_ALGORITHM)


def create_refresh_token(user_id: str, jti_access: str) -> str:
    """Create a signed JWT refresh token tied to the access token's jti."""
    now = datetime.now(timezone.utc)
    payload = {
        "sub": user_id,
        "type": "refresh",
        "access_jti": jti_access,
        "jti": str(uuid.uuid4()),
        "iat": now,
        "exp": now + timedelta(days=_REFRESH_TOKEN_EXPIRE_DAYS),
    }
    return jwt.encode(payload, _get_jwt_secret(), algorithm=_ALGORITHM)


# ──────────────────────────────────────────────────────────────────────────────
# JWT verification
# ──────────────────────────────────────────────────────────────────────────────

def decode_token(token: str) -> dict:
    """
    Decode and validate a JWT token.
    Raises HTTPException 401 on any failure.
    """
    try:
        payload = jwt.decode(token, _get_jwt_secret(), algorithms=[_ALGORITHM])
        return payload
    except JWTError as exc:
        raise HTTPException(
            status_code=status.HTTP_401_UNAUTHORIZED,
            detail=f"Invalid or expired token: {exc}",
            headers={"WWW-Authenticate": "Bearer"},
        )


# ──────────────────────────────────────────────────────────────────────────────
# Login logic
# ──────────────────────────────────────────────────────────────────────────────

def authenticate_user(username: str, password: str, conn) -> Optional[dict]:
    """
    Verify credentials against master.db.
    Returns user row as dict or None if authentication fails.
    """
    row = conn.execute(
        """
        SELECT id, username, password_hash, role, is_active, failed_attempts,
               locked_until
        FROM users
        WHERE username = ? AND is_deleted = 0
        """,
        (username.lower().strip(),),
    ).fetchone()

    if not row:
        return None

    user = dict(row)

    if not user["is_active"]:
        raise HTTPException(
            status_code=status.HTTP_403_FORBIDDEN,
            detail="Account is deactivated. Contact administrator.",
        )

    locked_until = user.get("locked_until")
    if locked_until:
        lock_dt = datetime.fromisoformat(locked_until)
        if datetime.now(timezone.utc) < lock_dt:
            raise HTTPException(
                status_code=status.HTTP_429_TOO_MANY_REQUESTS,
                detail=f"Account locked until {locked_until}. Too many failed attempts.",
            )

    if not verify_password(password, user["password_hash"]):
        # Increment failed attempts; lock after 5
        attempts = (user["failed_attempts"] or 0) + 1
        lock_sql = ""
        lock_params: tuple = ()
        if attempts >= 5:
            locked_until_dt = datetime.now(timezone.utc) + timedelta(minutes=15)
            lock_sql = ", locked_until = ?"
            lock_params = (locked_until_dt.isoformat(),)

        conn.execute(
            f"UPDATE users SET failed_attempts = ? {lock_sql} WHERE id = ?",
            (attempts, *lock_params, user["id"]),
        )
        conn.commit()
        return None

    # Successful login — reset counters
    conn.execute(
        "UPDATE users SET failed_attempts = 0, locked_until = NULL, last_login = ? WHERE id = ?",
        (datetime.now(timezone.utc).isoformat(), user["id"]),
    )
    conn.commit()
    return user


def login(
    username: str,
    password: str,
    client_device_hash: str,
    conn,
) -> dict:
    """
    Full login flow:
      1. Authenticate credentials
      2. Verify device hash matches enrolled device
      3. Issue access + refresh tokens

    Returns dict with access_token, refresh_token, token_type, user info.
    """
    user = authenticate_user(username, password, conn)

    if not user:
        raise HTTPException(
            status_code=status.HTTP_401_UNAUTHORIZED,
            detail="Invalid username or password.",
            headers={"WWW-Authenticate": "Bearer"},
        )

    # Device verification
    stored_device_hash = conn.execute(
        "SELECT device_hash FROM users WHERE id = ?", (user["id"],)
    ).fetchone()

    if stored_device_hash and stored_device_hash["device_hash"]:
        if stored_device_hash["device_hash"] != client_device_hash:
            raise HTTPException(
                status_code=status.HTTP_403_FORBIDDEN,
                detail="Unauthorized device. This account is bound to a different device.",
            )
    else:
        # First login — bind device hash to this user
        conn.execute(
            "UPDATE users SET device_hash = ? WHERE id = ?",
            (client_device_hash, user["id"]),
        )
        conn.commit()

    # Determine DB context
    db_context = "master" if user["role"] == "admin" else "user"

    access_token = create_access_token(
        user_id=user["id"],
        username=user["username"],
        role=user["role"],
        device_hash=client_device_hash,
        db_context=db_context,
    )

    return {
        "access_token": access_token,
        "token_type": "bearer",
        "user_id": user["id"],
        "username": user["username"],
        "role": user["role"],
        "db_context": db_context,
    }


# ──────────────────────────────────────────────────────────────────────────────
# FastAPI dependencies
# ──────────────────────────────────────────────────────────────────────────────

async def get_current_user(token: str = Depends(oauth2_scheme)) -> dict:
    """
    FastAPI dependency: decode JWT and return current user payload.
    Use on every protected endpoint.
    """
    payload = decode_token(token)

    # Device re-verification on every request
    current_device_hash = get_device_hash()
    token_device_hash = payload.get("device_hash", "")

    if token_device_hash and current_device_hash != token_device_hash:
        raise HTTPException(
            status_code=status.HTTP_403_FORBIDDEN,
            detail="Device mismatch detected. Session invalidated.",
        )

    return payload


async def require_admin(current_user: dict = Depends(get_current_user)) -> dict:
    """FastAPI dependency: require admin role."""
    if current_user.get("role") != "admin":
        raise HTTPException(
            status_code=status.HTTP_403_FORBIDDEN,
            detail="Admin role required.",
        )
    return current_user


async def require_manager_or_above(current_user: dict = Depends(get_current_user)) -> dict:
    """FastAPI dependency: require manager or admin role."""
    if current_user.get("role") not in ("admin", "manager"):
        raise HTTPException(
            status_code=status.HTTP_403_FORBIDDEN,
            detail="Manager or Admin role required.",
        )
    return current_user


# ──────────────────────────────────────────────────────────────────────────────
# Password change flow (triggers user.db re-key)
# ──────────────────────────────────────────────────────────────────────────────

def change_password(
    user_id: str,
    old_password: str,
    new_password: str,
    conn,
) -> str:
    """
    Change a user's password in master.db.
    Returns the new password_hash (caller must trigger user.db re-key with it).
    """
    row = conn.execute(
        "SELECT password_hash FROM users WHERE id = ? AND is_deleted = 0",
        (user_id,),
    ).fetchone()

    if not row:
        raise HTTPException(status_code=404, detail="User not found.")

    if not verify_password(old_password, row["password_hash"]):
        raise HTTPException(status_code=400, detail="Old password is incorrect.")

    new_hash = hash_password(new_password)
    conn.execute(
        "UPDATE users SET password_hash = ?, password_changed_at = ? WHERE id = ?",
        (new_hash, datetime.now(timezone.utc).isoformat(), user_id),
    )
    conn.commit()
    return new_hash
