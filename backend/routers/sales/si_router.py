"""si_router.py — Sale Invoices CRUD + post (creates JV) + sale return sub-router."""

import uuid
from datetime import datetime, timezone
from typing import Optional

from fastapi import APIRouter, Depends, HTTPException, Query

from backend.security.auth import get_current_user
from backend.security.permissions import require_permission
from backend.database.connection import get_master_db
from backend.models.sales.schemas import SICreate, SIUpdate, SRCreate, IDResponse, MessageResponse, SalesListResponse
from backend.services.numbering import next_number
from backend.services.auto_jv import create_sale_invoice_jv, create_sale_return_jv

router = APIRouter()


def _now() -> str:
    return datetime.now(timezone.utc).isoformat()


def _get_tax_rate_and_account(tax_id: Optional[str], conn) -> tuple[float, Optional[str]]:
    if not tax_id:
        return 0.0, None
    row = conn.execute("SELECT rate, account_id FROM tax_config WHERE id=?", (tax_id,)).fetchone()
    return (row["rate"], row["account_id"]) if row else (0.0, None)


def _calc_line(qty: float, unit_price: float, discount_pct: float, tax_rate: float) -> dict:
    gross        = qty * unit_price
    discount_amt = round(gross * discount_pct / 100, 4)
    net          = gross - discount_amt
    tax_amt      = round(net * tax_rate / 100, 4)
    return dict(discount_amt=discount_amt, tax_amt=tax_amt, line_total=round(net + tax_amt, 4), net=net)


def _build_si_lines(conn, si_id: str) -> list[dict]:
    rows = conn.execute(
        """SELECT sil.*, i.code AS item_code, i.name AS item_name, u.code AS uom_code
           FROM si_lines sil
           JOIN items i ON i.id = sil.item_id
           LEFT JOIN uom u ON u.id = sil.uom_id
           WHERE sil.si_id = ? ORDER BY sil.line_no""",
        (si_id,),
    ).fetchall()
    return [dict(r) for r in rows]


def _build_si_detail(conn, si_id: str) -> dict:
    row = conn.execute(
        """SELECT si.*, c.name AS customer_name, so.so_number, dc.dc_number
           FROM sale_invoices si
           JOIN customers c ON c.id = si.customer_id
           LEFT JOIN sale_orders so ON so.id = si.so_id
           LEFT JOIN delivery_challans dc ON dc.id = si.dc_id
           WHERE si.id = ?""",
        (si_id,),
    ).fetchone()
    if not row:
        raise HTTPException(404, "Sale invoice not found.")
    detail = dict(row)
    detail["lines"] = _build_si_lines(conn, si_id)
    return detail


# ─────────────────────────────────────────────────────────────────────────────
# LIST
# ─────────────────────────────────────────────────────────────────────────────

@router.get("/", dependencies=[Depends(require_permission("sales", "view"))])
def list_si(
    customer_id: Optional[str] = Query(None),
    status:      Optional[str] = Query(None),
    date_from:   Optional[str] = Query(None),
    date_to:     Optional[str] = Query(None),
    search:      Optional[str] = Query(None),
    page:  int = Query(1, ge=1),
    limit: int = Query(50, ge=1, le=500),
    conn = Depends(get_master_db),
):
    sql = """SELECT si.id, si.si_number, si.invoice_date, si.due_date,
                    si.status, si.grand_total, si.amount_due, si.created_at,
                    c.name AS customer_name, so.so_number
             FROM sale_invoices si
             JOIN customers c ON c.id = si.customer_id
             LEFT JOIN sale_orders so ON so.id = si.so_id
             WHERE 1=1"""
    params: list = []
    if customer_id:
        sql += " AND si.customer_id = ?"; params.append(customer_id)
    if status:
        sql += " AND si.status = ?"; params.append(status)
    if date_from:
        sql += " AND si.invoice_date >= ?"; params.append(date_from)
    if date_to:
        sql += " AND si.invoice_date <= ?"; params.append(date_to)
    if search:
        sql += " AND (si.si_number LIKE ? OR c.name LIKE ?)"; s = f"%{search}%"; params += [s, s]

    total = conn.execute(f"SELECT COUNT(*) FROM ({sql})", params).fetchone()[0]
    sql  += " ORDER BY si.created_at DESC LIMIT ? OFFSET ?"
    params += [limit, (page - 1) * limit]
    return SalesListResponse(items=[dict(r) for r in conn.execute(sql, params).fetchall()],
                             total=total, page=page, limit=limit)


@router.get("/{si_id}", dependencies=[Depends(require_permission("sales", "view"))])
def get_si(si_id: str, conn=Depends(get_master_db)):
    return _build_si_detail(conn, si_id)


# ─────────────────────────────────────────────────────────────────────────────
# CREATE
# ─────────────────────────────────────────────────────────────────────────────

@router.post("/", dependencies=[Depends(require_permission("sales", "create"))])
def create_si(
    body: SICreate,
    current_user: dict = Depends(get_current_user),
    conn = Depends(get_master_db),
) -> IDResponse:
    cust = conn.execute(
        "SELECT id, account_id FROM customers WHERE id=? AND is_active=1", (body.customer_id,)
    ).fetchone()
    if not cust:
        raise HTTPException(404, "Customer not found or inactive.")

    si_id     = str(uuid.uuid4())
    si_number = next_number("SI", conn)
    subtotal = discount_total = tax_total = 0.0

    for i, line in enumerate(body.lines, 1):
        item = conn.execute("SELECT id FROM items WHERE id=? AND is_active=1", (line.item_id,)).fetchone()
        if not item:
            raise HTTPException(404, f"Item {line.item_id} not found.")
        tax_rate, _ = _get_tax_rate_and_account(line.tax_id, conn)
        amounts     = _calc_line(line.qty, line.unit_price, line.discount_pct, tax_rate)
        conn.execute(
            """INSERT INTO si_lines
               (id, si_id, line_no, so_line_id, dc_line_id,
                item_id, description, uom_id,
                qty, unit_price, discount_pct, discount_amt,
                tax_id, tax_rate, tax_amt, line_total)
               VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)""",
            (str(uuid.uuid4()), si_id, i, line.so_line_id, line.dc_line_id,
             line.item_id, line.description, line.uom_id,
             line.qty, line.unit_price, line.discount_pct, amounts["discount_amt"],
             line.tax_id, tax_rate, amounts["tax_amt"], amounts["line_total"]),
        )
        subtotal      += line.qty * line.unit_price
        discount_total += amounts["discount_amt"]
        tax_total     += amounts["tax_amt"]

    grand_total   = subtotal - discount_total + tax_total
    receivable_acc = body.receivable_account_id or cust["account_id"]

    conn.execute(
        """INSERT INTO sale_invoices
           (id, si_number, customer_id, so_id, dc_id,
            invoice_date, due_date, status,
            currency_id, payment_term_id, receivable_account_id,
            subtotal, discount_total, tax_total, grand_total,
            amount_paid, amount_due,
            notes, created_by, created_at, updated_at)
           VALUES (?,?,?,?,?,?,?,'draft',?,?,?,?,?,?,?,0,?,?,?,?,?)""",
        (si_id, si_number, body.customer_id, body.so_id, body.dc_id,
         body.invoice_date, body.due_date,
         body.currency_id, body.payment_term_id, receivable_acc,
         round(subtotal,2), round(discount_total,2), round(tax_total,2), round(grand_total,2),
         round(grand_total,2),
         body.notes, current_user.get("sub"), _now(), _now()),
    )
    return IDResponse(id=si_id, number=si_number)


# ─────────────────────────────────────────────────────────────────────────────
# UPDATE (draft only)
# ─────────────────────────────────────────────────────────────────────────────

@router.put("/{si_id}", dependencies=[Depends(require_permission("sales", "edit"))])
def update_si(si_id: str, body: SIUpdate, conn=Depends(get_master_db)) -> MessageResponse:
    si = conn.execute("SELECT status FROM sale_invoices WHERE id=?", (si_id,)).fetchone()
    if not si or si["status"] != "draft":
        raise HTTPException(400, "Cannot edit SI in current status.")
    updates = {k: v for k, v in body.model_dump(exclude_none=True).items()}
    if updates:
        updates["updated_at"] = _now()
        set_clause = ", ".join(f"{k}=?" for k in updates)
        conn.execute(f"UPDATE sale_invoices SET {set_clause} WHERE id=?", (*updates.values(), si_id))
    return MessageResponse(detail="SI updated.")


# ─────────────────────────────────────────────────────────────────────────────
# POST → creates JV
# ─────────────────────────────────────────────────────────────────────────────

@router.post("/{si_id}/post", dependencies=[Depends(require_permission("sales", "post"))])
def post_si(si_id: str, current_user: dict = Depends(get_current_user), conn=Depends(get_master_db)):
    si = conn.execute("SELECT * FROM sale_invoices WHERE id=?", (si_id,)).fetchone()
    if not si:
        raise HTTPException(404, "SI not found.")
    if si["status"] != "draft":
        raise HTTPException(400, "Only draft invoices can be posted.")
    if not si["receivable_account_id"]:
        raise HTTPException(400, "Receivable account not set.")

    cust = conn.execute("SELECT name FROM customers WHERE id=?", (si["customer_id"],)).fetchone()
    customer_name = cust["name"] if cust else "Unknown"

    lines_raw = conn.execute(
        """SELECT sil.tax_amt, sil.line_total,
                  (sil.qty * sil.unit_price - sil.discount_amt) AS amount_net,
                  i.sale_account_id,
                  tc.account_id AS tax_account_id
           FROM si_lines sil
           JOIN items i ON i.id = sil.item_id
           LEFT JOIN tax_config tc ON tc.id = sil.tax_id
           WHERE sil.si_id = ?""",
        (si_id,),
    ).fetchall()

    jv_lines = [dict(r) for r in lines_raw]
    tax_account_id = next((r["tax_account_id"] for r in jv_lines if r["tax_account_id"]), None)

    jv_id = create_sale_invoice_jv(
        conn,
        si_id=si_id,
        si_number=si["si_number"],
        si_date=si["invoice_date"],
        customer_name=customer_name,
        receivable_account_id=si["receivable_account_id"],
        lines=jv_lines,
        tax_account_id=tax_account_id,
        grand_total=si["grand_total"],
        created_by=current_user.get("sub", "system"),
    )
    conn.execute(
        "UPDATE sale_invoices SET status='posted', jv_id=?, posted_by=?, posted_at=?, updated_at=? WHERE id=?",
        (jv_id, current_user.get("sub"), _now(), _now(), si_id),
    )
    return MessageResponse(detail=f"SI posted. JV created.")


@router.post("/{si_id}/cancel", dependencies=[Depends(require_permission("sales", "edit"))])
def cancel_si(si_id: str, conn=Depends(get_master_db)) -> MessageResponse:
    si = conn.execute("SELECT status FROM sale_invoices WHERE id=?", (si_id,)).fetchone()
    if not si or si["status"] in ("posted","paid","cancelled"):
        raise HTTPException(400, "Cannot cancel invoice in current status.")
    conn.execute("UPDATE sale_invoices SET status='cancelled', updated_at=? WHERE id=?", (_now(), si_id))
    return MessageResponse(detail="SI cancelled.")


# ─────────────────────────────────────────────────────────────────────────────
# Sale Return sub-router
# ─────────────────────────────────────────────────────────────────────────────

sr_router = APIRouter()


@sr_router.get("/", dependencies=[Depends(require_permission("sales", "view"))])
def list_sr(
    customer_id: Optional[str] = Query(None),
    status:      Optional[str] = Query(None),
    page:  int = Query(1, ge=1),
    limit: int = Query(50, ge=1, le=500),
    conn = Depends(get_master_db),
):
    sql = """SELECT sr.id, sr.sr_number, sr.return_date, sr.status, sr.grand_total, sr.created_at,
                    c.name AS customer_name
             FROM sale_returns sr
             JOIN customers c ON c.id = sr.customer_id
             WHERE 1=1"""
    params: list = []
    if customer_id:
        sql += " AND sr.customer_id = ?"; params.append(customer_id)
    if status:
        sql += " AND sr.status = ?"; params.append(status)
    total = conn.execute(f"SELECT COUNT(*) FROM ({sql})", params).fetchone()[0]
    sql  += " ORDER BY sr.created_at DESC LIMIT ? OFFSET ?"
    params += [limit, (page - 1) * limit]
    return SalesListResponse(items=[dict(r) for r in conn.execute(sql, params).fetchall()],
                             total=total, page=page, limit=limit)


@sr_router.post("/", dependencies=[Depends(require_permission("sales", "create"))])
def create_sr(
    body: SRCreate,
    current_user: dict = Depends(get_current_user),
    conn = Depends(get_master_db),
) -> IDResponse:
    cust = conn.execute(
        "SELECT id, name FROM customers WHERE id=? AND is_active=1", (body.customer_id,)
    ).fetchone()
    if not cust:
        raise HTTPException(404, "Customer not found.")

    sr_id     = str(uuid.uuid4())
    sr_number = next_number("SR", conn)
    grand_total = 0.0

    for i, line in enumerate(body.lines, 1):
        total_line = line.qty * line.unit_price
        conn.execute(
            """INSERT INTO sale_return_lines
               (id, sr_id, line_no, si_line_id, item_id, uom_id,
                qty, unit_price, total, reason)
               VALUES (?,?,?,?,?,?,?,?,?,?)""",
            (str(uuid.uuid4()), sr_id, i, line.si_line_id,
             line.item_id, line.uom_id, line.qty, line.unit_price,
             round(total_line,2), line.reason),
        )
        grand_total += total_line

    conn.execute(
        """INSERT INTO sale_returns
           (id, sr_number, customer_id, si_id, return_date,
            status, grand_total, notes, created_by, created_at, updated_at)
           VALUES (?,?,?,?,?,'draft',?,?,?,?,?)""",
        (sr_id, sr_number, body.customer_id, body.si_id, body.return_date,
         round(grand_total,2), body.notes, current_user.get("sub"), _now(), _now()),
    )
    return IDResponse(id=sr_id, number=sr_number)


@sr_router.post("/{sr_id}/post", dependencies=[Depends(require_permission("sales", "post"))])
def post_sr(sr_id: str, current_user: dict = Depends(get_current_user), conn=Depends(get_master_db)):
    sr = conn.execute("SELECT * FROM sale_returns WHERE id=?", (sr_id,)).fetchone()
    if not sr or sr["status"] != "draft":
        raise HTTPException(400, "SR must be draft to post.")
    cust = conn.execute("SELECT name, account_id FROM customers WHERE id=?", (sr["customer_id"],)).fetchone()
    sale_acc_row = conn.execute(
        """SELECT i.sale_account_id FROM sale_return_lines srl
           JOIN items i ON i.id = srl.item_id
           WHERE srl.sr_id = ? LIMIT 1""",
        (sr_id,),
    ).fetchone()
    sale_acc = sale_acc_row["sale_account_id"] if sale_acc_row else cust["account_id"]
    jv_id = create_sale_return_jv(
        conn,
        sr_id=sr_id, sr_number=sr["sr_number"], return_date=sr["return_date"],
        customer_name=cust["name"],
        receivable_account_id=cust["account_id"],
        sale_account_id=sale_acc,
        grand_total=sr["grand_total"],
        created_by=current_user.get("sub", "system"),
    )
    conn.execute("UPDATE sale_returns SET status='posted', jv_id=?, updated_at=? WHERE id=?",
                 (jv_id, _now(), sr_id))
    return MessageResponse(detail="Sale return posted.")
