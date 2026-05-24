"""dc_router.py — Delivery Challans (from SO) + stock deduction."""

import uuid
from datetime import datetime, timezone
from typing import Optional

from fastapi import APIRouter, Depends, HTTPException, Query

from backend.security.auth import get_current_user
from backend.security.permissions import require_permission
from backend.database.connection import get_master_db
from backend.models.sales.schemas import DCCreate, IDResponse, MessageResponse, SalesListResponse
from backend.services.numbering import next_number
from backend.services.stock_ledger import process_dc_lines

router = APIRouter()


def _now() -> str:
    return datetime.now(timezone.utc).isoformat()


def _build_dc_lines(conn, dc_id: str) -> list[dict]:
    rows = conn.execute(
        """SELECT dl.*, i.code AS item_code, i.name AS item_name,
                  u.code AS uom_code, loc.name AS location_name,
                  sol.qty AS qty_ordered, sol.qty_delivered AS total_delivered
           FROM dc_lines dl
           JOIN items i ON i.id = dl.item_id
           LEFT JOIN uom u ON u.id = dl.uom_id
           LEFT JOIN inventory_locations loc ON loc.id = dl.location_id
           LEFT JOIN so_lines sol ON sol.id = dl.so_line_id
           WHERE dl.dc_id = ? ORDER BY dl.line_no""",
        (dc_id,),
    ).fetchall()
    return [dict(r) for r in rows]


def _build_dc_detail(conn, dc_id: str) -> dict:
    row = conn.execute(
        """SELECT dc.*, so.so_number, c.name AS customer_name
           FROM delivery_challans dc
           JOIN sale_orders so ON so.id = dc.so_id
           JOIN customers c ON c.id = so.customer_id
           WHERE dc.id = ?""",
        (dc_id,),
    ).fetchone()
    if not row:
        raise HTTPException(404, "DC not found.")
    detail = dict(row)
    detail["lines"] = _build_dc_lines(conn, dc_id)
    return detail


# ─────────────────────────────────────────────────────────────────────────────
# LIST
# ─────────────────────────────────────────────────────────────────────────────

@router.get("/", dependencies=[Depends(require_permission("sales", "view"))])
def list_dc(
    so_id:     Optional[str] = Query(None),
    status:    Optional[str] = Query(None),
    date_from: Optional[str] = Query(None),
    date_to:   Optional[str] = Query(None),
    search:    Optional[str] = Query(None),
    page:  int = Query(1, ge=1),
    limit: int = Query(50, ge=1, le=500),
    conn = Depends(get_master_db),
):
    sql = """SELECT dc.id, dc.dc_number, dc.dc_date, dc.status, dc.created_at,
                    so.so_number, c.name AS customer_name
             FROM delivery_challans dc
             JOIN sale_orders so ON so.id = dc.so_id
             JOIN customers c ON c.id = so.customer_id
             WHERE 1=1"""
    params: list = []
    if so_id:
        sql += " AND dc.so_id = ?"; params.append(so_id)
    if status:
        sql += " AND dc.status = ?"; params.append(status)
    if date_from:
        sql += " AND dc.dc_date >= ?"; params.append(date_from)
    if date_to:
        sql += " AND dc.dc_date <= ?"; params.append(date_to)
    if search:
        sql += " AND (dc.dc_number LIKE ? OR c.name LIKE ? OR so.so_number LIKE ?)"
        s = f"%{search}%"; params += [s, s, s]

    total = conn.execute(f"SELECT COUNT(*) FROM ({sql})", params).fetchone()[0]
    sql  += " ORDER BY dc.created_at DESC LIMIT ? OFFSET ?"
    params += [limit, (page - 1) * limit]
    return SalesListResponse(items=[dict(r) for r in conn.execute(sql, params).fetchall()],
                             total=total, page=page, limit=limit)


@router.get("/{dc_id}", dependencies=[Depends(require_permission("sales", "view"))])
def get_dc(dc_id: str, conn=Depends(get_master_db)):
    return _build_dc_detail(conn, dc_id)


# ─────────────────────────────────────────────────────────────────────────────
# CREATE
# ─────────────────────────────────────────────────────────────────────────────

@router.post("/", dependencies=[Depends(require_permission("sales", "create"))])
def create_dc(
    body: DCCreate,
    current_user: dict = Depends(get_current_user),
    conn = Depends(get_master_db),
) -> IDResponse:
    so = conn.execute(
        "SELECT id, status FROM sale_orders WHERE id=?", (body.so_id,)
    ).fetchone()
    if not so:
        raise HTTPException(404, "Sale order not found.")
    if so["status"] not in ("approved", "partial"):
        raise HTTPException(400, f"SO must be approved to create DC. Current: {so['status']}.")

    dc_id     = str(uuid.uuid4())
    dc_number = next_number("DC", conn)

    for i, line in enumerate(body.lines, 1):
        sol = conn.execute(
            "SELECT id, item_id, qty, qty_delivered, uom_id FROM so_lines WHERE id=? AND so_id=?",
            (line.so_line_id, body.so_id),
        ).fetchone()
        if not sol:
            raise HTTPException(404, f"SO line {line.so_line_id} not found in this SO.")

        remaining = sol["qty"] - sol["qty_delivered"]
        if line.qty_delivered > remaining + 0.0001:
            raise HTTPException(400, f"Delivery qty ({line.qty_delivered}) exceeds remaining ({remaining:.2f}).")

        conn.execute(
            """INSERT INTO dc_lines
               (id, dc_id, line_no, so_line_id, item_id, uom_id,
                qty_delivered, location_id)
               VALUES (?,?,?,?,?,?,?,?)""",
            (str(uuid.uuid4()), dc_id, i, line.so_line_id,
             line.item_id, line.uom_id or sol["uom_id"],
             line.qty_delivered, line.location_id or body.location_id),
        )
        conn.execute(
            "UPDATE so_lines SET qty_delivered = qty_delivered + ? WHERE id=?",
            (line.qty_delivered, line.so_line_id),
        )

    conn.execute(
        """INSERT INTO delivery_challans
           (id, dc_number, so_id, dc_date, status,
            location_id, notes, created_by, created_at, updated_at)
           VALUES (?,?,?,?,'draft',?,?,?,?,?)""",
        (dc_id, dc_number, body.so_id, body.dc_date,
         body.location_id, body.notes, current_user.get("sub"), _now(), _now()),
    )
    _refresh_so_status(conn, body.so_id)
    return IDResponse(id=dc_id, number=dc_number)


# ─────────────────────────────────────────────────────────────────────────────
# POST DC → deducts stock
# ─────────────────────────────────────────────────────────────────────────────

@router.post("/{dc_id}/post", dependencies=[Depends(require_permission("sales", "post"))])
def post_dc(
    dc_id: str,
    current_user: dict = Depends(get_current_user),
    conn = Depends(get_master_db),
) -> MessageResponse:
    dc = conn.execute("SELECT id, dc_number, dc_date, status FROM delivery_challans WHERE id=?", (dc_id,)).fetchone()
    if not dc:
        raise HTTPException(404, "DC not found.")
    if dc["status"] != "draft":
        raise HTTPException(400, "DC must be draft to post.")

    lines_raw = conn.execute(
        "SELECT id AS dc_line_id, item_id, qty_delivered, location_id FROM dc_lines WHERE dc_id=?",
        (dc_id,),
    ).fetchall()

    # Get unit cost (from item standard cost)
    stock_lines = []
    for r in lines_raw:
        cost_row = conn.execute("SELECT standard_cost FROM items WHERE id=?", (r["item_id"],)).fetchone()
        stock_lines.append({
            "dc_line_id":   r["dc_line_id"],
            "item_id":      r["item_id"],
            "qty_delivered": r["qty_delivered"],
            "unit_cost":    cost_row["standard_cost"] if cost_row else 0.0,
            "location_id":  r["location_id"],
        })

    process_dc_lines(
        conn,
        dc_id=dc_id,
        dc_number=dc["dc_number"],
        dc_date=dc["dc_date"],
        lines=stock_lines,
        created_by=current_user.get("sub", "system"),
    )

    conn.execute(
        "UPDATE delivery_challans SET status='posted', posted_by=?, posted_at=?, updated_at=? WHERE id=?",
        (current_user.get("sub"), _now(), _now(), dc_id),
    )
    return MessageResponse(detail="DC posted. Stock deducted.")


@router.post("/{dc_id}/cancel", dependencies=[Depends(require_permission("sales", "edit"))])
def cancel_dc(dc_id: str, conn=Depends(get_master_db)) -> MessageResponse:
    dc = conn.execute("SELECT status, so_id FROM delivery_challans WHERE id=?", (dc_id,)).fetchone()
    if not dc or dc["status"] != "draft":
        raise HTTPException(400, "Only draft DCs can be cancelled.")
    lines = conn.execute("SELECT so_line_id, qty_delivered FROM dc_lines WHERE dc_id=?", (dc_id,)).fetchall()
    for ln in lines:
        conn.execute(
            "UPDATE so_lines SET qty_delivered = MAX(0, qty_delivered - ?) WHERE id=?",
            (ln["qty_delivered"], ln["so_line_id"]),
        )
    conn.execute("UPDATE delivery_challans SET status='cancelled', updated_at=? WHERE id=?", (_now(), dc_id))
    _refresh_so_status(conn, dc["so_id"])
    return MessageResponse(detail="DC cancelled.")


def _refresh_so_status(conn, so_id: str) -> None:
    lines = conn.execute("SELECT qty, qty_delivered FROM so_lines WHERE so_id=?", (so_id,)).fetchall()
    if not lines:
        return
    total_ord = sum(r["qty"] for r in lines)
    total_del = sum(r["qty_delivered"] for r in lines)
    if total_del <= 0:
        new_status = "approved"
    elif total_del >= total_ord - 0.0001:
        new_status = "delivered"
    else:
        new_status = "partial"
    conn.execute(
        "UPDATE sale_orders SET status=?, updated_at=? WHERE id=? AND status NOT IN ('cancelled','closed')",
        (new_status, _now(), so_id),
    )
