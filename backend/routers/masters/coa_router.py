"""coa_router.py — Chart of Accounts CRUD + hierarchy tree endpoints."""

import uuid
from datetime import datetime, timezone
from typing import Optional

from fastapi import APIRouter, Depends, HTTPException, Query

from backend.security.auth import get_current_user
from backend.security.permissions import require_permission
from backend.database.connection import get_master_db
from backend.models.masters.schemas import (
    COACreate, COAUpdate, IDResponse, MessageResponse
)

router = APIRouter()


def _now() -> str:
    return datetime.now(timezone.utc).isoformat()


def _build_tree(all_rows: list[dict], parent_id: Optional[str] = None) -> list[dict]:
    result = []
    for row in all_rows:
        if row["parent_id"] == parent_id:
            node = dict(row)
            node["children"] = _build_tree(all_rows, row["id"])
            result.append(node)
    return result


def _flatten_tree(nodes: list[dict], level: int = 0) -> list[dict]:
    result = []
    for node in nodes:
        flat = {k: v for k, v in node.items() if k != "children"}
        prefix = "  " * level + ("└─ " if level else "")
        flat["display_name"] = prefix + flat["name"]
        result.append(flat)
        result.extend(_flatten_tree(node.get("children", []), level + 1))
    return result


@router.get("/", dependencies=[Depends(require_permission("masters", "view"))])
def list_coa(
    tree: bool = Query(False),
    account_type: Optional[str] = Query(None),
    active_only: bool = Query(True),
    flat_dropdown: bool = Query(False),
    conn=Depends(get_master_db),
):
    """List COA — as tree, flat dropdown, or plain list."""
    sql = "SELECT id, code, name, account_type, parent_id, level, is_control, is_active FROM chart_of_accounts WHERE 1=1"
    params: list = []
    if active_only:
        sql += " AND is_active = 1"
    if account_type:
        sql += " AND account_type = ?"
        params.append(account_type)
    sql += " ORDER BY code"
    rows = [dict(r) for r in conn.execute(sql, params).fetchall()]

    if flat_dropdown:
        return _flatten_tree(_build_tree(rows))
    if tree:
        return _build_tree(rows)
    return rows


@router.get("/{account_id}", dependencies=[Depends(require_permission("masters", "view"))])
def get_coa(account_id: str, conn=Depends(get_master_db)):
    row = conn.execute("SELECT * FROM chart_of_accounts WHERE id = ?", (account_id,)).fetchone()
    if not row:
        raise HTTPException(404, "Account not found.")
    return dict(row)


@router.post("/", dependencies=[Depends(require_permission("masters", "create"))])
def create_coa(
    body: COACreate,
    current_user: dict = Depends(get_current_user),
    conn=Depends(get_master_db),
) -> IDResponse:
    if conn.execute("SELECT id FROM chart_of_accounts WHERE code = ?", (body.code,)).fetchone():
        raise HTTPException(409, f"Account code '{body.code}' already exists.")

    level = 1
    if body.parent_id:
        parent = conn.execute(
            "SELECT level, account_type FROM chart_of_accounts WHERE id = ?", (body.parent_id,)
        ).fetchone()
        if not parent:
            raise HTTPException(404, "Parent account not found.")
        if parent["account_type"] != body.account_type:
            raise HTTPException(400, "Child must match parent account_type.")
        level = parent["level"] + 1

    account_id = str(uuid.uuid4())
    conn.execute(
        """INSERT INTO chart_of_accounts
           (id,code,name,account_type,parent_id,level,is_control,is_active,
            currency_id,description,created_by,created_at,updated_at)
           VALUES (?,?,?,?,?,?,?,1,?,?,?,?,?)""",
        (account_id, body.code, body.name, body.account_type, body.parent_id,
         level, int(body.is_control), body.currency_id, body.description,
         current_user.get("sub"), _now(), _now()),
    )
    return IDResponse(id=account_id)


@router.put("/{account_id}", dependencies=[Depends(require_permission("masters", "edit"))])
def update_coa(
    account_id: str,
    body: COAUpdate,
    conn=Depends(get_master_db),
) -> MessageResponse:
    if not conn.execute("SELECT id FROM chart_of_accounts WHERE id = ?", (account_id,)).fetchone():
        raise HTTPException(404, "Account not found.")
    updates = {k: v for k, v in body.model_dump(exclude_none=True).items()}
    if not updates:
        return MessageResponse(detail="No changes.")
    updates["updated_at"] = _now()
    set_clause = ", ".join(f"{k} = ?" for k in updates)
    conn.execute(f"UPDATE chart_of_accounts SET {set_clause} WHERE id = ?", (*updates.values(), account_id))
    return MessageResponse(detail="Account updated.")


@router.delete("/{account_id}", dependencies=[Depends(require_permission("masters", "delete"))])
def delete_coa(account_id: str, conn=Depends(get_master_db)) -> MessageResponse:
    if conn.execute("SELECT id FROM chart_of_accounts WHERE parent_id = ? AND is_active=1", (account_id,)).fetchone():
        raise HTTPException(400, "Cannot deactivate: has active child accounts.")
    if conn.execute("SELECT id FROM journal_voucher_lines WHERE account_id = ? LIMIT 1", (account_id,)).fetchone():
        raise HTTPException(400, "Account has posted transactions.")
    conn.execute("UPDATE chart_of_accounts SET is_active=0, updated_at=? WHERE id=?", (_now(), account_id))
    return MessageResponse(detail="Account deactivated.")
