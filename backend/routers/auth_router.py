"""auth_router.py — Authentication endpoints."""

from fastapi import APIRouter, Depends, Body
from pydantic import BaseModel, Field

from backend.security.auth import (
    login,
    get_current_user,
    change_password,
    decode_token,
)
from backend.security.permissions import get_all_permissions_for_role
from backend.database.connection import get_master_db

router = APIRouter()


class LoginRequest(BaseModel):
    username: str = Field(..., min_length=1, max_length=100)
    password: str = Field(..., min_length=1, max_length=200)
    device_hash: str = Field(..., min_length=8, max_length=128)


class ChangePasswordRequest(BaseModel):
    old_password: str = Field(..., min_length=1)
    new_password: str = Field(..., min_length=8, max_length=200)


@router.post("/login")
def login_endpoint(
    body: LoginRequest,
    conn=Depends(get_master_db),
):
    """Authenticate user and return JWT access token."""
    result = login(body.username, body.password, body.device_hash, conn)
    # Attach permissions for the frontend
    result["permissions"] = get_all_permissions_for_role(result["role"])
    return result


@router.post("/logout")
def logout_endpoint(current_user: dict = Depends(get_current_user)):
    """
    Logout — client must discard the token.
    Server-side token revocation requires a token blacklist (Redis/DB).
    For offline ERP use case, client-side discard is sufficient.
    """
    return {"detail": "Logged out successfully."}


@router.get("/me")
def me_endpoint(current_user: dict = Depends(get_current_user)):
    """Return current user info and permissions."""
    return {
        "user_id": current_user.get("sub"),
        "username": current_user.get("username"),
        "role": current_user.get("role"),
        "db_context": current_user.get("db"),
        "permissions": get_all_permissions_for_role(current_user.get("role", "")),
    }


@router.post("/change-password")
def change_password_endpoint(
    body: ChangePasswordRequest,
    current_user: dict = Depends(get_current_user),
    conn=Depends(get_master_db),
):
    """Change current user's password and return new password hash for user.db re-key."""
    from backend.database.connection import get_user_conn, rekey_user_db
    from backend.security.encryption import derive_user_hash
    from backend.config import settings

    user_id = current_user.get("sub")
    username = current_user.get("username")

    old_row = conn.execute(
        "SELECT password_hash FROM users WHERE id = ?", (user_id,)
    ).fetchone()
    old_hash = old_row["password_hash"] if old_row else ""
    old_user_hash = derive_user_hash(username, old_hash)

    new_password_hash = change_password(user_id, body.old_password, body.new_password, conn)
    new_user_hash = derive_user_hash(username, new_password_hash)

    # Re-key user.db with new derived key
    try:
        rekey_user_db(
            old_secret_token=settings.SECRET_TOKEN,
            old_user_hash=old_user_hash,
            new_secret_token=settings.SECRET_TOKEN,
            new_user_hash=new_user_hash,
            db_salt=settings.DB_SALT,
        )
    except FileNotFoundError:
        pass  # user.db may not exist for admin-only setup

    return {"detail": "Password changed successfully. Please log in again."}
