"""jv_router.py — Manual Journal Vouchers CRUD + post."""

import uuid
from datetime import datetime, timezone
from typing import Optional

from fastapi import APIRouter, Depends, HTTPException, Query

from backend.security.auth import get_current_user
from backend.security.permissions import require_permission
from backend.database.connection import get_master_db
from backend.models.payments.schemas import JVCreate, IDResponse, MessageResponse, PayListResponse
from backend.services.numbering import next_number

router = APIRouter()


def _now() -> str:
    return datetime.now(timezone.utc).isoformat()


def _build_jv_lines(conn, jv_id: str) -> list[dict]:
    rows = conn.execute(
        """SELECT jvl.*, coa.code AS account_code, coa.name AS account_name
           FROM journal_voucher_lines jvl
           JOIN chart_of_accounts coa ON coa.id = jvl.account_id
           WHERE jvl.jv_id = ? ORDER BY jvl.line_no""",
        (jv_id,),
    ).fetchall()
    return [dict(r) for r in rows]


@router.get("/", dependencies=[Depends(require_permission("accounts", "view"))])
def list_jv(
    source_type: Optional[str] = Query(None),
    date_from:   Optional[str] = Query(None),
    date_to:     Optional[str] = Query(None),
    search:      Optional[str] = Query(None),
    page:  int = Query(1, ge=1),
    limit: int = Query(50, ge=1, le=500),
    conn=Depends(get_master_db),
):
    sql = "SELECT id, jv_number, jv_date, narration, status, source_type, created_at FROM journal_vouchers WHERE 1=1"
    params: list = []
    if source_type:
        sql += " AND source_type = ?"; params.append(source_type)
    if date_from:
        sql += " AND jv_date >= ?"; params.append(date_from)
    if date_to:
        sql += " AND jv_date <= ?"; params.append(date_to)
    if search:
        sql += " AND (jv_number LIKE ? OR narration LIKE ?)"; s = f"%{search}%"; params += [s, s]
    total = conn.execute(f"SELECT COUNT(*) FROM ({sql})", params).fetchone()[0]
    sql += " ORDER BY created_at DESC LIMIT ? OFFSET ?"
    params += [limit, (page - 1) * limit]
    return PayListResponse(items=[dict(r) for r in conn.execute(sql, params).fetchall()],
                           total=total, page=page, limit=limit)


@router.get("/{jv_id}", dependencies=[Depends(require_permission("accounts", "view"))])
def get_jv(jv_id: str, conn=Depends(get_master_db)):
    row = conn.execute("SELECT * FROM journal_vouchers WHERE id=?", (jv_id,)).fetchone()
    if not row:
        raise HTTPException(404, "JV not found.")
    detail = dict(row)
    detail["lines"] = _build_jv_lines(conn, jv_id)
    return detail


@router.post("/", dependencies=[Depends(require_permission("accounts", "create"))])
def create_jv(body: JVCreate, current_user: dict = Depends(get_current_user), conn=Depends(get_master_db)) -> IDResponse:
    # Validate balanced
    total_dr = sum(l.debit  for l in body.lines)
    total_cr = sum(l.credit for l in body.lines)
    if abs(total_dr - total_cr) > 0.01:
        raise HTTPException(400, f"JV must be balanced. Dr={total_dr:.2f} Cr={total_cr:.2f}")

    jv_id     = str(uuid.uuid4())
    jv_number = next_number("JV", conn)
    conn.execute(
        """INSERT INTO journal_vouchers
           (id, jv_number, jv_date, narration, status,
            source_type, source_id, created_by, created_at, updated_at)
           VALUES (?,?,?,?,'draft','manual','',?,?,?)""",
        (jv_id, jv_number, body.jv_date, body.narration,
         current_user.get("sub"), _now(), _now()),
    )
    for i, line in enumerate(body.lines, 1):
        conn.execute(
            "INSERT INTO journal_voucher_lines (id, jv_id, line_no, account_id, debit, credit, narration) VALUES (?,?,?,?,?,?,?)",
            (str(uuid.uuid4()), jv_id, i, line.account_id, line.debit, line.credit, line.narration or ""),
        )
    return IDResponse(id=jv_id, number=jv_number)


@router.post("/{jv_id}/post", dependencies=[Depends(require_permission("accounts", "post"))])
def post_jv(jv_id: str, current_user: dict = Depends(get_current_user), conn=Depends(get_master_db)):
    jv = conn.execute("SELECT status FROM journal_vouchers WHERE id=?", (jv_id,)).fetchone()
    if not jv or jv["status"] != "draft":
        raise HTTPException(400, "Only draft JVs can be posted.")
    conn.execute(
        "UPDATE journal_vouchers SET status='posted', posted_by=?, posted_at=?, updated_at=? WHERE id=?",
        (current_user.get("sub"), _now(), _now(), jv_id),
    )
    return MessageResponse(detail="JV posted.")


@router.post("/{jv_id}/reverse", dependencies=[Depends(require_permission("accounts", "post"))])
def reverse_jv(jv_id: str, current_user: dict = Depends(get_current_user), conn=Depends(get_master_db)):
    """Create a reversing JV with swapped Dr/Cr."""
    jv = conn.execute("SELECT * FROM journal_vouchers WHERE id=?", (jv_id,)).fetchone()
    if not jv or jv["status"] != "posted":
        raise HTTPException(400, "Only posted JVs can be reversed.")
    lines = _build_jv_lines(conn, jv_id)

    rev_id     = str(uuid.uuid4())
    rev_number = next_number("JV", conn)
    conn.execute(
        """INSERT INTO journal_vouchers
           (id, jv_number, jv_date, narration, status,
            source_type, source_id, created_by, created_at, updated_at)
           VALUES (?,?,?,?,'posted','reversal',?,?,?,?)""",
        (rev_id, rev_number, _now()[:10], f"REV: {jv['narration']}",
         jv_id, current_user.get("sub"), _now(), _now()),
    )
    for i, line in enumerate(lines, 1):
        conn.execute(
            "INSERT INTO journal_voucher_lines (id, jv_id, line_no, account_id, debit, credit, narration) VALUES (?,?,?,?,?,?,?)",
            (str(uuid.uuid4()), rev_id, i, line["account_id"],
             line["credit"], line["debit"], f"REV: {line.get('narration','')}"),
        )
    conn.execute("UPDATE journal_vouchers SET status='reversed', updated_at=? WHERE id=?", (_now(), jv_id))
    return IDResponse(id=rev_id, number=rev_number, detail="JV reversed.")
