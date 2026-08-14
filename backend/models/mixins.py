"""Shared columns per spec section 2 (Common System Rules)."""
from __future__ import annotations

import datetime as dt

from sqlalchemy import ForeignKey, String
from sqlalchemy.orm import Mapped, mapped_column

MASTER_STATUSES = ("Active", "Inactive")
DOCUMENT_STATUSES = ("Draft", "Pending Approval", "Approved", "Posted", "Cancelled")


class TimestampMixin:
    created_at: Mapped[dt.datetime] = mapped_column(default=dt.datetime.utcnow)
    updated_at: Mapped[dt.datetime] = mapped_column(
        default=dt.datetime.utcnow, onupdate=dt.datetime.utcnow
    )


class MasterStatusMixin(TimestampMixin):
    """For master records: Active / Inactive (spec 2.1)."""

    status: Mapped[str] = mapped_column(String(20), default="Active")
    created_by: Mapped[str | None] = mapped_column(String(100), nullable=True)
    updated_by: Mapped[str | None] = mapped_column(String(100), nullable=True)


class DocumentStatusMixin(TimestampMixin):
    """For transactional documents: Draft/Pending Approval/Approved/Posted/Cancelled (spec 2.1/2.2)."""

    status: Mapped[str] = mapped_column(String(20), default="Draft")
    created_by: Mapped[str | None] = mapped_column(String(100), nullable=True)
    updated_by: Mapped[str | None] = mapped_column(String(100), nullable=True)
    approved_by: Mapped[str | None] = mapped_column(String(100), nullable=True)
    approved_at: Mapped[dt.datetime | None] = mapped_column(nullable=True)
    posted_by: Mapped[str | None] = mapped_column(String(100), nullable=True)
    posted_at: Mapped[dt.datetime | None] = mapped_column(nullable=True)
    cancelled_by: Mapped[str | None] = mapped_column(String(100), nullable=True)
    cancelled_at: Mapped[dt.datetime | None] = mapped_column(nullable=True)
