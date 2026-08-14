"""Attachment storage — data/attachments/<entity_type>/<entity_id>/<filename> (spec 3.1)."""
from __future__ import annotations

import shutil
from pathlib import Path

from sqlalchemy.orm import Session

from config import ATTACHMENTS_DIR
from backend.models.system_models import Attachment


def entity_dir(entity_type: str, entity_id: int) -> Path:
    path = ATTACHMENTS_DIR / entity_type.lower() / str(entity_id)
    path.mkdir(parents=True, exist_ok=True)
    return path


def save_attachment(
    db: Session, *, entity_type: str, entity_id: int, source_path: str, uploaded_by: str | None = None
) -> Attachment:
    src = Path(source_path)
    dest_dir = entity_dir(entity_type, entity_id)
    dest = dest_dir / src.name
    shutil.copy2(src, dest)
    relative_path = str(dest.relative_to(ATTACHMENTS_DIR.parent))
    record = Attachment(
        entity_type=entity_type,
        entity_id=entity_id,
        file_name=src.name,
        relative_path=relative_path,
        uploaded_by=uploaded_by,
    )
    db.add(record)
    db.flush()
    return record


def list_attachments(db: Session, entity_type: str, entity_id: int) -> list[Attachment]:
    from sqlalchemy import select

    stmt = select(Attachment).where(
        Attachment.entity_type == entity_type, Attachment.entity_id == entity_id
    )
    return list(db.execute(stmt).scalars())
