"""so_router.py — Sale Orders CRUD + approval workflow."""

import uuid
from datetime import datetime, timezone
from typing import Optional

from fastapi import APIRouter, Depends, HTTPException, Query

from backend.security.auth import get_current_user
from backend.security.permissions import require_permission
from backend.database.connection import get_master_db
from backend.models.sales.schemas import SOCreate, SOUpdate, IDResponse, MessageResponse, SalesListResponse
from backend.services.numbering import next_number

router = APIRouter()


def _now() -> str:
    return datetime.now(timezone.utc).isoformat()


def _get_tax_rate(tax_id: Optional[str], conn) -> float:
    if not tax_id:
        return 0.0
    row = conn.execute("SELECT rate FROM tax_config WHERE id=?", (tax_id,)).fetchone()
    return row["rate"] if row else 0.0


def _calc_line(qty: float, unit_price: float, discount_pct: float, tax_rate: float) -> dict:
    gross        = qty * unit_price
    discount_amt = round(gross * discount_pct / 100, 4)
    net          = gross - discount_amt
    tax_amt      = round(net * tax_rate / 100, 4)
    return dict(discount_amt=discount_amt, tax_amt=tax_amt, line_total=round(net + tax_amt, 4))


def _build_so_lines(conn, so_id: str) -> list[dict]:
    rows = conn.execute(
        """SELECT sol.*, i.code AS item_code, i.name AS item_name, u.code AS uom_code
           FROM so_lines sol
           JOIN items i ON i.id = sol.item_id
           LEFT JOIN uom u ON u.id = sol.uom_id
           WHERE sol.so_id = ? ORDER BY sol.line_no""",
        (so_id,),
    ).fetchall()
    return [dict(r) for r in rows]


def _build_so_detail(conn, so_id: str) -> dict:
    row = conn.execute(
        """SELECT so.*, c.name AS customer_name
           FROM sale_orders so
           JOIN customers c ON c.id = so.customer_id
           WHERE so.id = ?""",
        (so_id,),
    ).fetchone()
    if not row:
        raise HTTPException(404, "Sale order not found.")
    detail = dict(row)
    detail["lines"] = _build_so_lines(conn, so_id)
    return detail


# ─────────────────────────────────────────────────────────────────────────────
# LIST
# ─────────────────────────────────────────────────────────────────────────────

@router.get("/", dependencies=[Depends(require_permission("sales", "view"))])
def list_so(
    customer_id: Optional[str] = Query(None),
    status:      Optional[str] = Query(None),
    date_from:   Optional[str] = Query(None),
    date_to:     Optional[str] = Query(None),
    search:      Optional[str] = Query(None),
    page:  int = Query(1, ge=1),
    limit: int = Query(50, ge=1, le=500),
    conn = Depends(get_master_db),
):
    sql = """SELECT so.id, so.so_number, so.so_date, so.delivery_date,
                    so.status, so.grand_total, so.created_at,
                    c.name AS customer_name
             FROM sale_orders so
             JOIN customers c ON c.id = so.customer_id
             WHERE 1=1"""
    params: list = []
    if customer_id:
        sql += " AND so.customer_id = ?"; params.append(customer_id)
    if status:
        sql += " AND so.status = ?"; params.append(status)
    if date_from:
        sql += " AND so.so_date >= ?"; params.append(date_from)
    if date_to:
        sql += " AND so.so_date <= ?"; params.append(date_to)
    if search:
        sql += " AND (so.so_number LIKE ? OR c.name LIKE ?)"; s = f"%{search}%"; params += [s, s]

    total = conn.execute(f"SELECT COUNT(*) FROM ({sql})", params).fetchone()[0]
    sql  += " ORDER BY so.created_at DESC LIMIT ? OFFSET ?"
    params += [limit, (page - 1) * limit]
    return SalesListResponse(items=[dict(r) for r in conn.execute(sql, params).fetchall()],
                             total=total, page=page, limit=limit)


@router.get("/{so_id}", dependencies=[Depends(require_permission("sales", "view"))])
def get_so(so_id: str, conn=Depends(get_master_db)):
    return _build_so_detail(conn, so_id)


# ─────────────────────────────────────────────────────────────────────────────
# CREATE
# ─────────────────────────────────────────────────────────────────────────────

@router.post("/", dependencies=[Depends(require_permission("sales", "create"))])
def create_so(
    body: SOCreate,
    current_user: dict = Depends(get_current_user),
    conn = Depends(get_master_db),
) -> IDResponse:
    cust = conn.execute("SELECT id FROM customers WHERE id=? AND is_active=1", (body.customer_id,)).fetchone()
    if not cust:
        raise HTTPException(404, "Customer not found or inactive.")

    so_id     = str(uuid.uuid4())
    so_number = next_number("SO", conn)
    subtotal = discount_total = tax_total = 0.0

    for i, line in enumerate(body.lines, 1):
        item = conn.execute("SELECT id FROM items WHERE id=? AND is_active=1", (line.item_id,)).fetchone()
        if not item:
            raise HTTPException(404, f"Item {line.item_id} not found.")
        tax_rate = _get_tax_rate(line.tax_id, conn)
        amounts  = _calc_line(line.qty, line.unit_price, line.discount_pct, tax_rate)
        conn.execute(
            """INSERT INTO so_lines
               (id, so_id, line_no, item_id, description, uom_id,
                qty, unit_price, discount_pct, discount_amt,
                tax_id, tax_rate, tax_amt, line_total,
                qty_delivered, qty_billed, location_id)
               VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,0,0,?)""",
            (str(uuid.uuid4()), so_id, i, line.item_id, line.description, line.uom_id,
             line.qty, line.unit_price, line.discount_pct, amounts["discount_amt"],
             line.tax_id, tax_rate, amounts["tax_amt"], amounts["line_total"],
             line.location_id),
        )
        subtotal      += line.qty * line.unit_price
        discount_total += amounts["discount_amt"]
        tax_total     += amounts["tax_amt"]

    grand_total = subtotal - discount_total + tax_total
    conn.execute(
        """INSERT INTO sale_orders
           (id, so_number, customer_id, so_date, delivery_date, status,
            currency_id, payment_term_id, location_id,
            subtotal, discount_total, tax_total, grand_total,
            notes, created_by, created_at, updated_at)
           VALUES (?,?,?,?,?,'draft',?,?,?,?,?,?,?,?,?,?,?)""",
        (so_id, so_number, body.customer_id, body.so_date, body.delivery_date,
         body.currency_id, body.payment_term_id, body.location_id,
         round(subtotal,2), round(discount_total,2), round(tax_total,2), round(grand_total,2),
         body.notes, current_user.get("sub"), _now(), _now()),
    )
    return IDResponse(id=so_id, number=so_number)


# ─────────────────────────────────────────────────────────────────────────────
# UPDATE / TRANSITIONS / DELETE
# ─────────────────────────────────────────────────────────────────────────────

@router.put("/{so_id}", dependencies=[Depends(require_permission("sales", "edit"))])
def update_so(so_id: str, body: SOUpdate, conn=Depends(get_master_db)) -> MessageResponse:
    so = conn.execute("SELECT status FROM sale_orders WHERE id=?", (so_id,)).fetchone()
    if not so:
        raise HTTPException(404, "SO not found.")
    if so["status"] != "draft":
        raise HTTPException(400, "Cannot edit SO in current status.")

    updates: dict = {}
    for f in ("customer_id","so_date","delivery_date","currency_id","payment_term_id","location_id","notes"):
        v = getattr(body, f, None)
        if v is not None:
            updates[f] = v

    if body.lines is not None:
        conn.execute("DELETE FROM so_lines WHERE so_id=?", (so_id,))
        subtotal = discount_total = tax_total = 0.0
        for i, line in enumerate(body.lines, 1):
            tax_rate = _get_tax_rate(line.tax_id, conn)
            amounts  = _calc_line(line.qty, line.unit_price, line.discount_pct, tax_rate)
            conn.execute(
                """INSERT INTO so_lines
                   (id, so_id, line_no, item_id, description, uom_id,
                    qty, unit_price, discount_pct, discount_amt,
                    tax_id, tax_rate, tax_amt, line_total,
                    qty_delivered, qty_billed, location_id)
                   VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,0,0,?)""",
                (str(uuid.uuid4()), so_id, i, line.item_id, line.description, line.uom_id,
                 line.qty, line.unit_price, line.discount_pct, amounts["discount_amt"],
                 line.tax_id, tax_rate, amounts["tax_amt"], amounts["line_total"],
                 line.location_id),
            )
            subtotal      += line.qty * line.unit_price
            discount_total += amounts["discount_amt"]
            tax_total     += amounts["tax_amt"]
        grand_total = subtotal - discount_total + tax_total
        updates.update(dict(subtotal=round(subtotal,2), discount_total=round(discount_total,2),
                            tax_total=round(tax_total,2), grand_total=round(grand_total,2)))

    if updates:
        updates["updated_at"] = _now()
        set_clause = ", ".join(f"{k}=?" for k in updates)
        conn.execute(f"UPDATE sale_orders SET {set_clause} WHERE id=?", (*updates.values(), so_id))
    return MessageResponse(detail="SO updated.")


@router.post("/{so_id}/submit", dependencies=[Depends(require_permission("sales", "create"))])
def submit_so(so_id: str, conn=Depends(get_master_db)):
    so = conn.execute("SELECT status FROM sale_orders WHERE id=?", (so_id,)).fetchone()
    if not so or so["status"] != "draft":
        raise HTTPException(400, "SO must be draft to submit.")
    conn.execute("UPDATE sale_orders SET status='submitted', updated_at=? WHERE id=?", (_now(), so_id))
    return MessageResponse(detail="SO submitted.")


@router.post("/{so_id}/approve", dependencies=[Depends(require_permission("sales", "approve"))])
def approve_so(so_id: str, current_user: dict = Depends(get_current_user), conn=Depends(get_master_db)):
    so = conn.execute("SELECT status FROM sale_orders WHERE id=?", (so_id,)).fetchone()
    if not so or so["status"] != "submitted":
        raise HTTPException(400, "SO must be submitted to approve.")
    conn.execute(
        "UPDATE sale_orders SET status='approved', approved_by=?, updated_at=? WHERE id=?",
        (current_user.get("sub"), _now(), so_id),
    )
    return MessageResponse(detail="SO approved.")


@router.post("/{so_id}/cancel", dependencies=[Depends(require_permission("sales", "edit"))])
def cancel_so(so_id: str, conn=Depends(get_master_db)) -> MessageResponse:
    so = conn.execute("SELECT status FROM sale_orders WHERE id=?", (so_id,)).fetchone()
    if not so:
        raise HTTPException(404, "SO not found.")
    if so["status"] in ("posted", "closed", "cancelled"):
        raise HTTPException(400, "Cannot cancel SO in current status.")
    conn.execute("UPDATE sale_orders SET status='cancelled', updated_at=? WHERE id=?", (_now(), so_id))
    return MessageResponse(detail="SO cancelled.")


@router.delete("/{so_id}", dependencies=[Depends(require_permission("sales", "delete"))])
def delete_so(so_id: str, conn=Depends(get_master_db)) -> MessageResponse:
    so = conn.execute("SELECT status FROM sale_orders WHERE id=?", (so_id,)).fetchone()
    if not so or so["status"] != "draft":
        raise HTTPException(400, "Only draft SOs can be deleted.")
    conn.execute("DELETE FROM so_lines WHERE so_id=?", (so_id,))
    conn.execute("DELETE FROM sale_orders WHERE id=?", (so_id,))
    return MessageResponse(detail="SO deleted.")
