"""Audit trail writer — spec section 13."""
from __future__ import annotations

from typing import Optional

from sqlalchemy.orm import Session

from backend.models.system_models import AuditLog


def record(
    db: Session,
    *,
    username: str,
    action: str,
    record_type: str,
    record_id: int,
    old_value: Optional[str] = None,
    new_value: Optional[str] = None,
) -> None:
    db.add(
        AuditLog(
            username=username,
            action=action,
            record_type=record_type,
            record_id=record_id,
            old_value=old_value,
            new_value=new_value,
        )
    )
