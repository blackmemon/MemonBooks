"""grn_router.py — Goods Receipt Note CRUD + accept/reject flow + stock update."""

import uuid
from datetime import datetime, timezone
from typing import Optional

from fastapi import APIRouter, Depends, HTTPException, Query

from backend.security.auth import get_current_user
from backend.security.permissions import require_permission
from backend.database.connection import get_master_db
from backend.models.purchases.schemas import (
    GRNCreate, IDResponse, MessageResponse, PurchaseListResponse,
)
from backend.services.numbering import next_number
from backend.services.stock_ledger import process_grn_lines

router = APIRouter()


def _now() -> str:
    return datetime.now(timezone.utc).isoformat()


def _build_grn_lines(conn, grn_id: str) -> list[dict]:
    rows = conn.execute(
        """SELECT gl.*, i.code AS item_code, i.name AS item_name,
                  u.code AS uom_code, loc.name AS location_name,
                  pol.qty AS qty_ordered
           FROM grn_lines gl
           JOIN items i ON i.id = gl.item_id
           LEFT JOIN uom u ON u.id = gl.uom_id
           LEFT JOIN inventory_locations loc ON loc.id = gl.location_id
           LEFT JOIN po_lines pol ON pol.id = gl.po_line_id
           WHERE gl.grn_id = ?
           ORDER BY gl.line_no""",
        (grn_id,),
    ).fetchall()
    return [dict(r) for r in rows]


def _build_grn_detail(conn, grn_id: str) -> dict:
    row = conn.execute(
        """SELECT g.*, po.po_number, v.name AS vendor_name
           FROM grns g
           JOIN purchase_orders po ON po.id = g.po_id
           JOIN vendors v ON v.id = po.vendor_id
           WHERE g.id = ?""",
        (grn_id,),
    ).fetchone()
    if not row:
        raise HTTPException(404, "GRN not found.")
    detail = dict(row)
    detail["lines"] = _build_grn_lines(conn, grn_id)
    return detail


# ─────────────────────────────────────────────────────────────────────────────
# LIST
# ─────────────────────────────────────────────────────────────────────────────

@router.get("/", dependencies=[Depends(require_permission("purchases", "view"))])
def list_grn(
    po_id:     Optional[str] = Query(None),
    status:    Optional[str] = Query(None),
    date_from: Optional[str] = Query(None),
    date_to:   Optional[str] = Query(None),
    search:    Optional[str] = Query(None),
    page:  int = Query(1, ge=1),
    limit: int = Query(50, ge=1, le=500),
    conn = Depends(get_master_db),
):
    sql = """SELECT g.id, g.grn_number, g.grn_date, g.status, g.created_at,
                    po.po_number, v.name AS vendor_name
             FROM grns g
             JOIN purchase_orders po ON po.id = g.po_id
             JOIN vendors v ON v.id = po.vendor_id
             WHERE 1=1"""
    params: list = []
    if po_id:
        sql += " AND g.po_id = ?"; params.append(po_id)
    if status:
        sql += " AND g.status = ?"; params.append(status)
    if date_from:
        sql += " AND g.grn_date >= ?"; params.append(date_from)
    if date_to:
        sql += " AND g.grn_date <= ?"; params.append(date_to)
    if search:
        sql += " AND (g.grn_number LIKE ? OR v.name LIKE ? OR po.po_number LIKE ?)"
        s = f"%{search}%"; params += [s, s, s]

    total = conn.execute(f"SELECT COUNT(*) FROM ({sql})", params).fetchone()[0]
    sql  += " ORDER BY g.created_at DESC LIMIT ? OFFSET ?"
    params += [limit, (page - 1) * limit]
    rows = [dict(r) for r in conn.execute(sql, params).fetchall()]
    return PurchaseListResponse(items=rows, total=total, page=page, limit=limit)


# ─────────────────────────────────────────────────────────────────────────────
# GET single
# ─────────────────────────────────────────────────────────────────────────────

@router.get("/{grn_id}", dependencies=[Depends(require_permission("purchases", "view"))])
def get_grn(grn_id: str, conn=Depends(get_master_db)):
    return _build_grn_detail(conn, grn_id)


# ─────────────────────────────────────────────────────────────────────────────
# CREATE GRN (from approved PO)
# ─────────────────────────────────────────────────────────────────────────────

@router.post("/", dependencies=[Depends(require_permission("purchases", "create"))])
def create_grn(
    body: GRNCreate,
    current_user: dict = Depends(get_current_user),
    conn = Depends(get_master_db),
) -> IDResponse:
    # Validate PO exists and is approved
    po = conn.execute(
        "SELECT id, status, vendor_id FROM purchase_orders WHERE id=?", (body.po_id,)
    ).fetchone()
    if not po:
        raise HTTPException(404, "Purchase order not found.")
    if po["status"] not in ("approved", "partial"):
        raise HTTPException(400, f"PO must be approved to create GRN. Current status: {po['status']}.")

    grn_id     = str(uuid.uuid4())
    grn_number = next_number("GRN", conn)

    for i, line in enumerate(body.lines, 1):
        # Validate PO line exists under this PO
        pol = conn.execute(
            "SELECT id, item_id, qty, qty_received, uom_id FROM po_lines WHERE id=? AND po_id=?",
            (line.po_line_id, body.po_id),
        ).fetchone()
        if not pol:
            raise HTTPException(404, f"PO line {line.po_line_id} not found in this PO.")

        remaining = pol["qty"] - pol["qty_received"]
        if line.qty_received > remaining + 0.0001:
            raise HTTPException(
                400,
                f"Received qty ({line.qty_received}) exceeds remaining qty ({remaining:.2f}) for line {i}.",
            )

        # Resolve unit cost from PO line
        pol_full = conn.execute("SELECT unit_price FROM po_lines WHERE id=?", (line.po_line_id,)).fetchone()
        unit_cost = pol_full["unit_price"] if pol_full else 0.0

        conn.execute(
            """INSERT INTO grn_lines
               (id, grn_id, line_no, po_line_id, item_id, uom_id,
                qty_received, qty_accepted, qty_rejected,
                rejection_reason, unit_cost, location_id)
               VALUES (?,?,?,?,?,?,?,?,?,?,?,?)""",
            (
                str(uuid.uuid4()), grn_id, i, line.po_line_id,
                line.item_id, pol["uom_id"],
                line.qty_received, line.qty_accepted, line.qty_rejected,
                line.rejection_reason, unit_cost,
                line.location_id or body.location_id,
            ),
        )

        # Update qty_received on PO line
        conn.execute(
            "UPDATE po_lines SET qty_received = qty_received + ? WHERE id=?",
            (line.qty_accepted, line.po_line_id),
        )

    conn.execute(
        """INSERT INTO grns
           (id, grn_number, po_id, grn_date, status,
            location_id, notes, created_by, created_at, updated_at)
           VALUES (?,?,?,?,'draft',?,?,?,?,?)""",
        (grn_id, grn_number, body.po_id,
         body.grn_date, body.location_id, body.notes,
         current_user.get("sub"), _now(), _now()),
    )

    # Update PO status to partial if any lines received
    _refresh_po_status(conn, body.po_id)
    return IDResponse(id=grn_id, number=grn_number)


# ─────────────────────────────────────────────────────────────────────────────
# ACCEPT GRN → updates stock
# ─────────────────────────────────────────────────────────────────────────────

@router.post("/{grn_id}/accept", dependencies=[Depends(require_permission("purchases", "approve"))])
def accept_grn(
    grn_id: str,
    current_user: dict = Depends(get_current_user),
    conn = Depends(get_master_db),
) -> MessageResponse:
    grn = conn.execute("SELECT id, grn_number, grn_date, status FROM grns WHERE id=?", (grn_id,)).fetchone()
    if not grn:
        raise HTTPException(404, "GRN not found.")
    if grn["status"] != "draft":
        raise HTTPException(400, "GRN must be in draft to accept.")

    # Build lines for stock ledger
    lines_raw = conn.execute(
        """SELECT gl.id AS grn_line_id, gl.item_id, gl.qty_accepted,
                  gl.unit_cost, gl.location_id
           FROM grn_lines gl WHERE gl.grn_id = ?""",
        (grn_id,),
    ).fetchall()

    stock_lines = [
        {
            "grn_line_id": r["grn_line_id"],
            "item_id":     r["item_id"],
            "qty_accepted": r["qty_accepted"],
            "unit_cost":   r["unit_cost"],
            "location_id": r["location_id"],
        }
        for r in lines_raw
    ]

    process_grn_lines(
        conn,
        grn_id=grn_id,
        grn_number=grn["grn_number"],
        grn_date=grn["grn_date"],
        lines=stock_lines,
        created_by=current_user.get("sub", "system"),
    )

    conn.execute(
        "UPDATE grns SET status='accepted', accepted_by=?, accepted_at=?, updated_at=? WHERE id=?",
        (current_user.get("sub"), _now(), _now(), grn_id),
    )
    return MessageResponse(detail="GRN accepted and stock updated.")


# ─────────────────────────────────────────────────────────────────────────────
# CANCEL GRN (draft only)
# ─────────────────────────────────────────────────────────────────────────────

@router.post("/{grn_id}/cancel", dependencies=[Depends(require_permission("purchases", "edit"))])
def cancel_grn(grn_id: str, conn=Depends(get_master_db)) -> MessageResponse:
    grn = conn.execute("SELECT id, status, po_id FROM grns WHERE id=?", (grn_id,)).fetchone()
    if not grn:
        raise HTTPException(404, "GRN not found.")
    if grn["status"] != "draft":
        raise HTTPException(400, "Only draft GRNs can be cancelled.")

    # Reverse the qty_received increments on PO lines
    lines = conn.execute(
        "SELECT po_line_id, qty_accepted FROM grn_lines WHERE grn_id=?", (grn_id,)
    ).fetchall()
    for ln in lines:
        conn.execute(
            "UPDATE po_lines SET qty_received = MAX(0, qty_received - ?) WHERE id=?",
            (ln["qty_accepted"], ln["po_line_id"]),
        )

    conn.execute("UPDATE grns SET status='cancelled', updated_at=? WHERE id=?", (_now(), grn_id))
    _refresh_po_status(conn, grn["po_id"])
    return MessageResponse(detail="GRN cancelled.")


# ─────────────────────────────────────────────────────────────────────────────
# Helper: refresh PO status after GRN changes
# ─────────────────────────────────────────────────────────────────────────────

def _refresh_po_status(conn, po_id: str) -> None:
    lines = conn.execute(
        "SELECT qty, qty_received FROM po_lines WHERE po_id=?", (po_id,)
    ).fetchall()
    if not lines:
        return
    total_ord = sum(r["qty"] for r in lines)
    total_rec = sum(r["qty_received"] for r in lines)
    if total_rec <= 0:
        new_status = "approved"
    elif total_rec >= total_ord - 0.0001:
        new_status = "received"
    else:
        new_status = "partial"
    conn.execute(
        "UPDATE purchase_orders SET status=?, updated_at=? WHERE id=? AND status NOT IN ('cancelled','closed')",
        (new_status, _now(), po_id),
    )
