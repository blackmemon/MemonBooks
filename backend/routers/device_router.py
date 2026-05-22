"""device_router.py — Device registration and approval endpoints."""

import uuid
from datetime import datetime, timezone

from fastapi import APIRouter, Depends, HTTPException
from pydantic import BaseModel, Field

from backend.security.auth import get_current_user, require_admin
from backend.database.connection import get_master_db
from backend.security.encryption import encrypt_record_fields
from backend.config import settings

router = APIRouter()


def _now() -> str:
    return datetime.now(timezone.utc).isoformat()


class RegisterDeviceRequest(BaseModel):
    device_name: str = Field(..., max_length=100)
    platform: str = Field(..., pattern="^(android|ios|windows|linux|macos)$")
    device_hash: str = Field(..., min_length=8, max_length=128)
    imei_1: str | None = Field(None, max_length=20)
    imei_2: str | None = Field(None, max_length=20)
    mac_address: str | None = Field(None, max_length=20)
    user_id: str | None = None


@router.post("/register")
def register_device(
    body: RegisterDeviceRequest,
    current_user: dict = Depends(require_admin),
    conn=Depends(get_master_db),
):
    """Register a new approved device (admin only)."""
    existing = conn.execute(
        "SELECT id FROM approved_devices WHERE device_hash = ?", (body.device_hash,)
    ).fetchone()

    if existing:
        raise HTTPException(status_code=409, detail="Device already registered.")

    record = {
        "imei_1": body.imei_1,
        "imei_2": body.imei_2,
        "mac_address": body.mac_address,
    }
    encrypted = encrypt_record_fields("approved_devices", record, settings.FIELD_ENCRYPT_KEY)

    device_id = str(uuid.uuid4())
    conn.execute(
        """INSERT INTO approved_devices
           (id, user_id, device_name, platform, device_hash, imei_1, imei_2, mac_address,
            is_active, approved_by, approved_at, created_at)
           VALUES (?,?,?,?,?,?,?,?,1,?,?,?)""",
        (
            device_id,
            body.user_id,
            body.device_name,
            body.platform,
            body.device_hash,
            encrypted.get("imei_1"),
            encrypted.get("imei_2"),
            encrypted.get("mac_address"),
            current_user.get("sub"),
            _now(),
            _now(),
        ),
    )
    return {"device_id": device_id, "detail": "Device registered successfully."}


@router.get("/list")
def list_devices(
    current_user: dict = Depends(require_admin),
    conn=Depends(get_master_db),
):
    """List all registered devices (admin only)."""
    rows = conn.execute(
        """SELECT id, device_name, platform, device_hash, is_active, approved_at, last_seen_at
           FROM approved_devices ORDER BY approved_at DESC"""
    ).fetchall()
    return [dict(r) for r in rows]


@router.delete("/{device_id}")
def revoke_device(
    device_id: str,
    current_user: dict = Depends(require_admin),
    conn=Depends(get_master_db),
):
    """Revoke (deactivate) a registered device (admin only)."""
    affected = conn.execute(
        "UPDATE approved_devices SET is_active = 0 WHERE id = ?", (device_id,)
    ).rowcount
    if affected == 0:
        raise HTTPException(status_code=404, detail="Device not found.")
    return {"detail": "Device revoked."}


@router.get("/hash")
def get_device_hash_endpoint():
    """Return the current machine's device hash (used by Tauri on desktop)."""
    from backend.security.device_fingerprint import get_device_hash
    return {"device_hash": get_device_hash()}
