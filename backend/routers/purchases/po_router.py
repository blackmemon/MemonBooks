"""po_router.py — Purchase Orders CRUD + status transitions."""

import uuid
from datetime import datetime, timezone
from typing import Optional

from fastapi import APIRouter, Depends, HTTPException, Query

from backend.security.auth import get_current_user
from backend.security.permissions import require_permission
from backend.database.connection import get_master_db
from backend.models.purchases.schemas import (
    POCreate, POUpdate, IDResponse, MessageResponse, PurchaseListResponse,
)
from backend.services.numbering import next_number

router = APIRouter()


def _now() -> str:
    return datetime.now(timezone.utc).isoformat()


def _calc_line_amounts(qty: float, unit_price: float, discount_pct: float,
                       tax_rate: float) -> dict:
    gross        = qty * unit_price
    discount_amt = round(gross * discount_pct / 100, 4)
    net          = gross - discount_amt
    tax_amt      = round(net * tax_rate / 100, 4)
    line_total   = round(net + tax_amt, 4)
    return dict(discount_amt=discount_amt, tax_amt=tax_amt, line_total=line_total, net=net)


def _get_tax_rate(tax_id: Optional[str], conn) -> float:
    if not tax_id:
        return 0.0
    row = conn.execute("SELECT rate FROM tax_config WHERE id=?", (tax_id,)).fetchone()
    return row["rate"] if row else 0.0


def _get_vendor(vendor_id: str, conn) -> dict:
    row = conn.execute("SELECT id, name FROM vendors WHERE id=? AND is_active=1", (vendor_id,)).fetchone()
    if not row:
        raise HTTPException(404, "Vendor not found or inactive.")
    return dict(row)


def _build_po_lines(conn, po_id: str) -> list[dict]:
    rows = conn.execute(
        """SELECT pol.*, i.code AS item_code, i.name AS item_name,
                  u.code AS uom_code
           FROM po_lines pol
           JOIN items i ON i.id = pol.item_id
           LEFT JOIN uom u ON u.id = pol.uom_id
           WHERE pol.po_id = ?
           ORDER BY pol.line_no""",
        (po_id,),
    ).fetchall()
    return [dict(r) for r in rows]


def _build_po_detail(conn, po_id: str) -> dict:
    row = conn.execute(
        """SELECT po.*, v.name AS vendor_name
           FROM purchase_orders po
           JOIN vendors v ON v.id = po.vendor_id
           WHERE po.id = ?""",
        (po_id,),
    ).fetchone()
    if not row:
        raise HTTPException(404, "Purchase order not found.")
    detail = dict(row)
    detail["lines"] = _build_po_lines(conn, po_id)
    return detail


# ─────────────────────────────────────────────────────────────────────────────
# LIST
# ─────────────────────────────────────────────────────────────────────────────

@router.get("/", dependencies=[Depends(require_permission("purchases", "view"))])
def list_po(
    vendor_id:  Optional[str] = Query(None),
    status:     Optional[str] = Query(None),
    date_from:  Optional[str] = Query(None),
    date_to:    Optional[str] = Query(None),
    search:     Optional[str] = Query(None),
    page:  int = Query(1, ge=1),
    limit: int = Query(50, ge=1, le=500),
    conn = Depends(get_master_db),
):
    sql = """SELECT po.id, po.po_number, po.po_date, po.expected_date,
                    po.status, po.grand_total, po.created_at,
                    v.name AS vendor_name
             FROM purchase_orders po
             JOIN vendors v ON v.id = po.vendor_id
             WHERE 1=1"""
    params: list = []
    if vendor_id:
        sql += " AND po.vendor_id = ?"; params.append(vendor_id)
    if status:
        sql += " AND po.status = ?"; params.append(status)
    if date_from:
        sql += " AND po.po_date >= ?"; params.append(date_from)
    if date_to:
        sql += " AND po.po_date <= ?"; params.append(date_to)
    if search:
        sql += " AND (po.po_number LIKE ? OR v.name LIKE ?)"; s = f"%{search}%"; params += [s, s]

    total = conn.execute(f"SELECT COUNT(*) FROM ({sql})", params).fetchone()[0]
    sql  += " ORDER BY po.created_at DESC LIMIT ? OFFSET ?"
    params += [limit, (page - 1) * limit]
    rows = [dict(r) for r in conn.execute(sql, params).fetchall()]
    return PurchaseListResponse(items=rows, total=total, page=page, limit=limit)


# ─────────────────────────────────────────────────────────────────────────────
# GET single
# ─────────────────────────────────────────────────────────────────────────────

@router.get("/{po_id}", dependencies=[Depends(require_permission("purchases", "view"))])
def get_po(po_id: str, conn=Depends(get_master_db)):
    return _build_po_detail(conn, po_id)


# ─────────────────────────────────────────────────────────────────────────────
# CREATE
# ─────────────────────────────────────────────────────────────────────────────

@router.post("/", dependencies=[Depends(require_permission("purchases", "create"))])
def create_po(
    body: POCreate,
    current_user: dict = Depends(get_current_user),
    conn = Depends(get_master_db),
) -> IDResponse:
    vendor = _get_vendor(body.vendor_id, conn)
    po_id     = str(uuid.uuid4())
    po_number = next_number("PO", conn)

    subtotal = discount_total = tax_total = 0.0

    for i, line in enumerate(body.lines, 1):
        # Validate item exists
        item = conn.execute("SELECT id, name FROM items WHERE id=? AND is_active=1", (line.item_id,)).fetchone()
        if not item:
            raise HTTPException(404, f"Item {line.item_id} not found.")

        tax_rate = _get_tax_rate(line.tax_id, conn)
        amounts  = _calc_line_amounts(line.qty, line.unit_price, line.discount_pct, tax_rate)

        conn.execute(
            """INSERT INTO po_lines
               (id, po_id, line_no, item_id, description, uom_id,
                qty, unit_price, discount_pct, discount_amt,
                tax_id, tax_rate, tax_amt, line_total,
                qty_received, qty_billed, location_id)
               VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,0,0,?)""",
            (
                str(uuid.uuid4()), po_id, i, line.item_id,
                line.description, line.uom_id,
                line.qty, line.unit_price, line.discount_pct, amounts["discount_amt"],
                line.tax_id, tax_rate, amounts["tax_amt"], amounts["line_total"],
                line.location_id,
            ),
        )
        subtotal      += line.qty * line.unit_price
        discount_total += amounts["discount_amt"]
        tax_total     += amounts["tax_amt"]

    grand_total = subtotal - discount_total + tax_total

    conn.execute(
        """INSERT INTO purchase_orders
           (id, po_number, vendor_id, po_date, expected_date, status,
            currency_id, payment_term_id, location_id,
            subtotal, discount_total, tax_total, grand_total,
            notes, created_by, created_at, updated_at)
           VALUES (?,?,?,?,?,'draft',?,?,?,?,?,?,?,?,?,?,?)""",
        (
            po_id, po_number, body.vendor_id,
            body.po_date, body.expected_date,
            body.currency_id, body.payment_term_id, body.location_id,
            round(subtotal, 2), round(discount_total, 2),
            round(tax_total, 2), round(grand_total, 2),
            body.notes, current_user.get("sub"), _now(), _now(),
        ),
    )
    return IDResponse(id=po_id, number=po_number)


# ─────────────────────────────────────────────────────────────────────────────
# UPDATE (draft only)
# ─────────────────────────────────────────────────────────────────────────────

@router.put("/{po_id}", dependencies=[Depends(require_permission("purchases", "edit"))])
def update_po(po_id: str, body: POUpdate, conn=Depends(get_master_db)) -> MessageResponse:
    po = conn.execute("SELECT status FROM purchase_orders WHERE id=?", (po_id,)).fetchone()
    if not po:
        raise HTTPException(404, "Purchase order not found.")
    if po["status"] != "draft":
        raise HTTPException(400, f"Cannot edit PO in '{po['status']}' status.")

    updates: dict = {}
    for f in ("vendor_id", "po_date", "expected_date", "currency_id",
              "payment_term_id", "location_id", "notes"):
        v = getattr(body, f, None)
        if v is not None:
            updates[f] = v

    if body.lines is not None:
        # Replace all lines
        conn.execute("DELETE FROM po_lines WHERE po_id=?", (po_id,))
        subtotal = discount_total = tax_total = 0.0
        for i, line in enumerate(body.lines, 1):
            tax_rate = _get_tax_rate(line.tax_id, conn)
            amounts  = _calc_line_amounts(line.qty, line.unit_price, line.discount_pct, tax_rate)
            conn.execute(
                """INSERT INTO po_lines
                   (id, po_id, line_no, item_id, description, uom_id,
                    qty, unit_price, discount_pct, discount_amt,
                    tax_id, tax_rate, tax_amt, line_total,
                    qty_received, qty_billed, location_id)
                   VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,0,0,?)""",
                (
                    str(uuid.uuid4()), po_id, i, line.item_id,
                    line.description, line.uom_id,
                    line.qty, line.unit_price, line.discount_pct, amounts["discount_amt"],
                    line.tax_id, tax_rate, amounts["tax_amt"], amounts["line_total"],
                    line.location_id,
                ),
            )
            subtotal      += line.qty * line.unit_price
            discount_total += amounts["discount_amt"]
            tax_total     += amounts["tax_amt"]
        grand_total = subtotal - discount_total + tax_total
        updates.update(dict(
            subtotal=round(subtotal, 2), discount_total=round(discount_total, 2),
            tax_total=round(tax_total, 2), grand_total=round(grand_total, 2),
        ))

    if updates:
        updates["updated_at"] = _now()
        set_clause = ", ".join(f"{k}=?" for k in updates)
        conn.execute(f"UPDATE purchase_orders SET {set_clause} WHERE id=?", (*updates.values(), po_id))
    return MessageResponse(detail="PO updated.")


# ─────────────────────────────────────────────────────────────────────────────
# STATUS TRANSITIONS
# ─────────────────────────────────────────────────────────────────────────────

def _transition(po_id: str, from_status: str, to_status: str,
                current_user: dict, conn, action_field: str = "") -> MessageResponse:
    po = conn.execute("SELECT id, status FROM purchase_orders WHERE id=?", (po_id,)).fetchone()
    if not po:
        raise HTTPException(404, "PO not found.")
    if po["status"] != from_status:
        raise HTTPException(400, f"PO must be in '{from_status}' status to perform this action.")
    updates: dict = {"status": to_status, "updated_at": _now()}
    if action_field:
        updates[action_field] = current_user.get("sub")
    set_clause = ", ".join(f"{k}=?" for k in updates)
    conn.execute(f"UPDATE purchase_orders SET {set_clause} WHERE id=?", (*updates.values(), po_id))
    return MessageResponse(detail=f"PO moved to '{to_status}'.")


@router.post("/{po_id}/submit", dependencies=[Depends(require_permission("purchases", "create"))])
def submit_po(po_id: str, current_user: dict = Depends(get_current_user), conn=Depends(get_master_db)):
    return _transition(po_id, "draft", "submitted", current_user, conn)


@router.post("/{po_id}/approve", dependencies=[Depends(require_permission("purchases", "approve"))])
def approve_po(po_id: str, current_user: dict = Depends(get_current_user), conn=Depends(get_master_db)):
    return _transition(po_id, "submitted", "approved", current_user, conn, "approved_by")


@router.post("/{po_id}/cancel", dependencies=[Depends(require_permission("purchases", "edit"))])
def cancel_po(po_id: str, current_user: dict = Depends(get_current_user), conn=Depends(get_master_db)):
    po = conn.execute("SELECT status FROM purchase_orders WHERE id=?", (po_id,)).fetchone()
    if not po:
        raise HTTPException(404, "PO not found.")
    if po["status"] in ("posted", "closed", "cancelled"):
        raise HTTPException(400, "Cannot cancel PO in current status.")
    conn.execute(
        "UPDATE purchase_orders SET status='cancelled', updated_at=? WHERE id=?",
        (_now(), po_id),
    )
    return MessageResponse(detail="PO cancelled.")


# ─────────────────────────────────────────────────────────────────────────────
# DELETE (draft only)
# ─────────────────────────────────────────────────────────────────────────────

@router.delete("/{po_id}", dependencies=[Depends(require_permission("purchases", "delete"))])
def delete_po(po_id: str, conn=Depends(get_master_db)) -> MessageResponse:
    po = conn.execute("SELECT status FROM purchase_orders WHERE id=?", (po_id,)).fetchone()
    if not po:
        raise HTTPException(404, "PO not found.")
    if po["status"] != "draft":
        raise HTTPException(400, "Only draft POs can be deleted.")
    conn.execute("DELETE FROM po_lines WHERE po_id=?", (po_id,))
    conn.execute("DELETE FROM purchase_orders WHERE id=?", (po_id,))
    return MessageResponse(detail="PO deleted.")
