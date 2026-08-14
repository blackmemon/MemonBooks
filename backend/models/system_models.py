"""Infrastructure models not owned by a specific business module: login
accounts, audit trail, generic attachments/custom-fields, and master.db's
device/license/config tables. Not part of spec section 14's four named
groups, but required by sections 2.2 (audit columns), 3.x (Attachments,
Custom Fields on every master), and 13 (Audit and History).
"""
from __future__ import annotations

import datetime as dt
from typing import Optional

from sqlalchemy import String, Text, UniqueConstraint
from sqlalchemy.orm import Mapped, mapped_column

from database.database import MasterBase, UserBase
from backend.models.mixins import TimestampMixin

# ---- master.db ----------------------------------------------------------


class DeviceRegistration(TimestampMixin, MasterBase):
    __tablename__ = "device_registration"

    id: Mapped[int] = mapped_column(primary_key=True)
    device_hash: Mapped[str] = mapped_column(String(64), unique=True)
    registered_at: Mapped[dt.datetime] = mapped_column(default=dt.datetime.utcnow)


class GlobalConfig(MasterBase):
    __tablename__ = "global_config"

    key: Mapped[str] = mapped_column(String(100), primary_key=True)
    value: Mapped[Optional[str]] = mapped_column(Text, nullable=True)


# ---- user.db --------------------------------------------------------------

USER_ROLES = ("Admin", "Accountant", "Sales", "Purchase", "HR", "Viewer")


class UserAccount(TimestampMixin, UserBase):
    __tablename__ = "user_accounts"

    id: Mapped[int] = mapped_column(primary_key=True)
    username: Mapped[str] = mapped_column(String(80), unique=True, index=True)
    full_name: Mapped[str] = mapped_column(String(150))
    password_hash: Mapped[str] = mapped_column(String(200))
    role: Mapped[str] = mapped_column(String(20), default="Admin")
    is_active: Mapped[bool] = mapped_column(default=True)


class AuditLog(UserBase):
    __tablename__ = "audit_logs"

    id: Mapped[int] = mapped_column(primary_key=True)
    username: Mapped[str] = mapped_column(String(80))
    action: Mapped[str] = mapped_column(String(20))  # Create/Update/Approve/Post/Cancel/Reverse
    record_type: Mapped[str] = mapped_column(String(60))
    record_id: Mapped[int] = mapped_column()
    old_value: Mapped[Optional[str]] = mapped_column(Text, nullable=True)
    new_value: Mapped[Optional[str]] = mapped_column(Text, nullable=True)
    timestamp: Mapped[dt.datetime] = mapped_column(default=dt.datetime.utcnow)


class CustomFieldValue(UserBase):
    """Dynamic name/value pairs, reused across Vendor/Customer/Item/Employee/... (spec 3.x, 4.10)."""

    __tablename__ = "custom_field_values"
    __table_args__ = (
        UniqueConstraint("entity_type", "entity_id", "field_name", name="uq_custom_field"),
    )

    id: Mapped[int] = mapped_column(primary_key=True)
    entity_type: Mapped[str] = mapped_column(String(40))
    entity_id: Mapped[int] = mapped_column()
    field_name: Mapped[str] = mapped_column(String(100))
    field_value: Mapped[Optional[str]] = mapped_column(Text, nullable=True)


class Attachment(TimestampMixin, UserBase):
    """Backs utils/file_storage.py: data/attachments/<entity_type>/<entity_id>/<filename>."""

    __tablename__ = "attachments"

    id: Mapped[int] = mapped_column(primary_key=True)
    entity_type: Mapped[str] = mapped_column(String(40))
    entity_id: Mapped[int] = mapped_column()
    file_name: Mapped[str] = mapped_column(String(255))
    relative_path: Mapped[str] = mapped_column(String(500))
    uploaded_by: Mapped[Optional[str]] = mapped_column(String(100), nullable=True)
