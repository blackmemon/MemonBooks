"""pi_router.py — Purchase Invoices CRUD + post (creates JV) + payment tracking."""

import uuid
from datetime import datetime, timezone
from typing import Optional

from fastapi import APIRouter, Depends, HTTPException, Query

from backend.security.auth import get_current_user
from backend.security.permissions import require_permission
from backend.database.connection import get_master_db
from backend.models.purchases.schemas import (
    PICreate, PIUpdate, IDResponse, MessageResponse, PurchaseListResponse,
)
from backend.services.numbering import next_number
from backend.services.auto_jv import create_purchase_invoice_jv

router = APIRouter()


def _now() -> str:
    return datetime.now(timezone.utc).isoformat()


def _calc_line(qty: float, unit_price: float, discount_pct: float,
               tax_rate: float, wht_rate: float) -> dict:
    gross        = qty * unit_price
    discount_amt = round(gross * discount_pct / 100, 4)
    net          = gross - discount_amt
    tax_amt      = round(net * tax_rate / 100, 4)
    wht_amt      = round(net * wht_rate / 100, 4)
    line_total   = round(net + tax_amt, 4)
    return dict(
        discount_amt=discount_amt, net=net,
        tax_amt=tax_amt, wht_amt=wht_amt, line_total=line_total,
    )


def _get_tax_info(tax_id: Optional[str], conn) -> tuple[float, Optional[str]]:
    """Returns (rate, account_id) or (0.0, None)."""
    if not tax_id:
        return 0.0, None
    row = conn.execute("SELECT rate, account_id FROM tax_config WHERE id=?", (tax_id,)).fetchone()
    return (row["rate"], row["account_id"]) if row else (0.0, None)


def _build_pi_lines(conn, pi_id: str) -> list[dict]:
    rows = conn.execute(
        """SELECT pl.*, i.code AS item_code, i.name AS item_name, u.code AS uom_code
           FROM pi_lines pl
           JOIN items i ON i.id = pl.item_id
           LEFT JOIN uom u ON u.id = pl.uom_id
           WHERE pl.pi_id = ?
           ORDER BY pl.line_no""",
        (pi_id,),
    ).fetchall()
    return [dict(r) for r in rows]


def _build_pi_detail(conn, pi_id: str) -> dict:
    row = conn.execute(
        """SELECT pi.*, v.name AS vendor_name,
                  po.po_number, g.grn_number
           FROM purchase_invoices pi
           JOIN vendors v ON v.id = pi.vendor_id
           LEFT JOIN purchase_orders po ON po.id = pi.po_id
           LEFT JOIN grns g ON g.id = pi.grn_id
           WHERE pi.id = ?""",
        (pi_id,),
    ).fetchone()
    if not row:
        raise HTTPException(404, "Purchase invoice not found.")
    detail = dict(row)
    detail["lines"] = _build_pi_lines(conn, pi_id)
    return detail


# ─────────────────────────────────────────────────────────────────────────────
# LIST
# ─────────────────────────────────────────────────────────────────────────────

@router.get("/", dependencies=[Depends(require_permission("purchases", "view"))])
def list_pi(
    vendor_id:  Optional[str] = Query(None),
    status:     Optional[str] = Query(None),
    date_from:  Optional[str] = Query(None),
    date_to:    Optional[str] = Query(None),
    search:     Optional[str] = Query(None),
    page:  int = Query(1, ge=1),
    limit: int = Query(50, ge=1, le=500),
    conn = Depends(get_master_db),
):
    sql = """SELECT pi.id, pi.pi_number, pi.invoice_date, pi.due_date,
                    pi.status, pi.grand_total, pi.amount_due,
                    pi.vendor_ref, pi.created_at,
                    v.name AS vendor_name, po.po_number
             FROM purchase_invoices pi
             JOIN vendors v ON v.id = pi.vendor_id
             LEFT JOIN purchase_orders po ON po.id = pi.po_id
             WHERE 1=1"""
    params: list = []
    if vendor_id:
        sql += " AND pi.vendor_id = ?"; params.append(vendor_id)
    if status:
        sql += " AND pi.status = ?"; params.append(status)
    if date_from:
        sql += " AND pi.invoice_date >= ?"; params.append(date_from)
    if date_to:
        sql += " AND pi.invoice_date <= ?"; params.append(date_to)
    if search:
        sql += " AND (pi.pi_number LIKE ? OR v.name LIKE ? OR pi.vendor_ref LIKE ?)"
        s = f"%{search}%"; params += [s, s, s]

    total = conn.execute(f"SELECT COUNT(*) FROM ({sql})", params).fetchone()[0]
    sql  += " ORDER BY pi.created_at DESC LIMIT ? OFFSET ?"
    params += [limit, (page - 1) * limit]
    rows = [dict(r) for r in conn.execute(sql, params).fetchall()]
    return PurchaseListResponse(items=rows, total=total, page=page, limit=limit)


# ─────────────────────────────────────────────────────────────────────────────
# GET
# ─────────────────────────────────────────────────────────────────────────────

@router.get("/{pi_id}", dependencies=[Depends(require_permission("purchases", "view"))])
def get_pi(pi_id: str, conn=Depends(get_master_db)):
    return _build_pi_detail(conn, pi_id)


# ─────────────────────────────────────────────────────────────────────────────
# CREATE
# ─────────────────────────────────────────────────────────────────────────────

@router.post("/", dependencies=[Depends(require_permission("purchases", "create"))])
def create_pi(
    body: PICreate,
    current_user: dict = Depends(get_current_user),
    conn = Depends(get_master_db),
) -> IDResponse:
    # Validate vendor
    vendor = conn.execute(
        "SELECT id, name, wht_rate, wht_account_id, account_id FROM vendors WHERE id=? AND is_active=1",
        (body.vendor_id,),
    ).fetchone()
    if not vendor:
        raise HTTPException(404, "Vendor not found or inactive.")

    pi_id     = str(uuid.uuid4())
    pi_number = next_number("PI", conn)

    subtotal = discount_total = tax_total = wht_total = 0.0

    for i, line in enumerate(body.lines, 1):
        item = conn.execute("SELECT id FROM items WHERE id=? AND is_active=1", (line.item_id,)).fetchone()
        if not item:
            raise HTTPException(404, f"Item {line.item_id} not found.")

        tax_rate, _ = _get_tax_info(line.tax_id, conn)
        wht_rate    = line.wht_rate if line.wht_rate else (vendor["wht_rate"] or 0.0)
        amounts     = _calc_line(line.qty, line.unit_price, line.discount_pct, tax_rate, wht_rate)

        conn.execute(
            """INSERT INTO pi_lines
               (id, pi_id, line_no, grn_line_id, po_line_id,
                item_id, description, uom_id,
                qty, unit_price, discount_pct, discount_amt,
                tax_id, tax_rate, tax_amt,
                wht_rate, wht_amt, line_total)
               VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)""",
            (
                str(uuid.uuid4()), pi_id, i,
                line.grn_line_id, line.po_line_id,
                line.item_id, line.description, line.uom_id,
                line.qty, line.unit_price, line.discount_pct, amounts["discount_amt"],
                line.tax_id, tax_rate, amounts["tax_amt"],
                wht_rate, amounts["wht_amt"], amounts["line_total"],
            ),
        )
        subtotal      += line.qty * line.unit_price
        discount_total += amounts["discount_amt"]
        tax_total     += amounts["tax_amt"]
        wht_total     += amounts["wht_amt"]

    grand_total  = subtotal - discount_total + tax_total
    amount_due   = grand_total - wht_total

    # Determine payable account
    payable_acc = body.payable_account_id or vendor["account_id"]

    conn.execute(
        """INSERT INTO purchase_invoices
           (id, pi_number, vendor_id, po_id, grn_id,
            invoice_date, due_date, vendor_ref, status,
            currency_id, payment_term_id, payable_account_id,
            subtotal, discount_total, tax_total, wht_total,
            grand_total, amount_paid, amount_due,
            notes, created_by, created_at, updated_at)
           VALUES (?,?,?,?,?,?,?,?,'draft',?,?,?,?,?,?,?,?,0,?,?,?,?,?)""",
        (
            pi_id, pi_number, body.vendor_id, body.po_id, body.grn_id,
            body.invoice_date, body.due_date, body.vendor_ref,
            body.currency_id, body.payment_term_id, payable_acc,
            round(subtotal, 2), round(discount_total, 2),
            round(tax_total, 2), round(wht_total, 2),
            round(grand_total, 2), round(amount_due, 2),
            body.notes, current_user.get("sub"), _now(), _now(),
        ),
    )
    return IDResponse(id=pi_id, number=pi_number)


# ─────────────────────────────────────────────────────────────────────────────
# UPDATE (draft only)
# ─────────────────────────────────────────────────────────────────────────────

@router.put("/{pi_id}", dependencies=[Depends(require_permission("purchases", "edit"))])
def update_pi(pi_id: str, body: PIUpdate, conn=Depends(get_master_db)) -> MessageResponse:
    pi = conn.execute("SELECT status FROM purchase_invoices WHERE id=?", (pi_id,)).fetchone()
    if not pi:
        raise HTTPException(404, "PI not found.")
    if pi["status"] != "draft":
        raise HTTPException(400, f"Cannot edit PI in '{pi['status']}' status.")
    updates = {k: v for k, v in body.model_dump(exclude_none=True).items()}
    if updates:
        updates["updated_at"] = _now()
        set_clause = ", ".join(f"{k}=?" for k in updates)
        conn.execute(f"UPDATE purchase_invoices SET {set_clause} WHERE id=?", (*updates.values(), pi_id))
    return MessageResponse(detail="PI updated.")


# ─────────────────────────────────────────────────────────────────────────────
# POST → creates JV (accounts payable entry)
# ─────────────────────────────────────────────────────────────────────────────

@router.post("/{pi_id}/post", dependencies=[Depends(require_permission("purchases", "post"))])
def post_pi(
    pi_id: str,
    current_user: dict = Depends(get_current_user),
    conn = Depends(get_master_db),
) -> MessageResponse:
    pi = conn.execute("SELECT * FROM purchase_invoices WHERE id=?", (pi_id,)).fetchone()
    if not pi:
        raise HTTPException(404, "PI not found.")
    if pi["status"] != "draft":
        raise HTTPException(400, "Only draft invoices can be posted.")
    if not pi["payable_account_id"]:
        raise HTTPException(400, "Payable account not set on invoice.")

    # Load vendor name
    vendor = conn.execute("SELECT name FROM vendors WHERE id=?", (pi["vendor_id"],)).fetchone()
    vendor_name = vendor["name"] if vendor else "Unknown"

    # Build line details for JV
    lines_raw = conn.execute(
        """SELECT pl.tax_amt, pl.wht_amt, pl.line_total,
                  (pl.qty * pl.unit_price - pl.discount_amt) AS amount_net,
                  i.purchase_account_id,
                  tc.account_id AS tax_account_id
           FROM pi_lines pl
           JOIN items i ON i.id = pl.item_id
           LEFT JOIN tax_config tc ON tc.id = pl.tax_id
           WHERE pl.pi_id = ?""",
        (pi_id,),
    ).fetchall()

    jv_lines = [dict(r) for r in lines_raw]
    tax_account_id = next((r["tax_account_id"] for r in jv_lines if r["tax_account_id"]), None)

    # Vendor WHT account
    vendor_full = conn.execute("SELECT wht_account_id FROM vendors WHERE id=?", (pi["vendor_id"],)).fetchone()
    wht_account_id = vendor_full["wht_account_id"] if vendor_full else None

    jv_id = create_purchase_invoice_jv(
        conn,
        pi_id=pi_id,
        pi_number=pi["pi_number"],
        pi_date=pi["invoice_date"],
        vendor_name=vendor_name,
        vendor_ref=pi["vendor_ref"],
        payable_account_id=pi["payable_account_id"],
        lines=jv_lines,
        tax_account_id=tax_account_id,
        wht_account_id=wht_account_id,
        grand_total=pi["grand_total"],
        wht_total=pi["wht_total"],
        created_by=current_user.get("sub", "system"),
    )

    conn.execute(
        "UPDATE purchase_invoices SET status='posted', jv_id=?, posted_by=?, posted_at=?, updated_at=? WHERE id=?",
        (jv_id, current_user.get("sub"), _now(), _now(), pi_id),
    )
    return MessageResponse(detail=f"PI posted. JV created: {jv_id[:8]}…")


# ─────────────────────────────────────────────────────────────────────────────
# CANCEL
# ─────────────────────────────────────────────────────────────────────────────

@router.post("/{pi_id}/cancel", dependencies=[Depends(require_permission("purchases", "edit"))])
def cancel_pi(pi_id: str, conn=Depends(get_master_db)) -> MessageResponse:
    pi = conn.execute("SELECT status FROM purchase_invoices WHERE id=?", (pi_id,)).fetchone()
    if not pi:
        raise HTTPException(404, "PI not found.")
    if pi["status"] in ("posted", "paid", "cancelled"):
        raise HTTPException(400, "Cannot cancel a posted or paid invoice.")
    conn.execute(
        "UPDATE purchase_invoices SET status='cancelled', updated_at=? WHERE id=?",
        (_now(), pi_id),
    )
    return MessageResponse(detail="PI cancelled.")


# ─────────────────────────────────────────────────────────────────────────────
# Purchase Return router (PR) — included here for simplicity
# ─────────────────────────────────────────────────────────────────────────────

pr_router = APIRouter()


@pr_router.get("/", dependencies=[Depends(require_permission("purchases", "view"))])
def list_pr(
    vendor_id:  Optional[str] = Query(None),
    status:     Optional[str] = Query(None),
    page:  int = Query(1, ge=1),
    limit: int = Query(50, ge=1, le=500),
    conn = Depends(get_master_db),
):
    sql = """SELECT pr.id, pr.pr_number, pr.return_date, pr.status, pr.grand_total,
                    v.name AS vendor_name
             FROM purchase_returns pr
             JOIN vendors v ON v.id = pr.vendor_id
             WHERE 1=1"""
    params: list = []
    if vendor_id:
        sql += " AND pr.vendor_id = ?"; params.append(vendor_id)
    if status:
        sql += " AND pr.status = ?"; params.append(status)
    total = conn.execute(f"SELECT COUNT(*) FROM ({sql})", params).fetchone()[0]
    sql  += " ORDER BY pr.created_at DESC LIMIT ? OFFSET ?"
    params += [limit, (page - 1) * limit]
    return PurchaseListResponse(items=[dict(r) for r in conn.execute(sql, params).fetchall()],
                                total=total, page=page, limit=limit)


@pr_router.post("/", dependencies=[Depends(require_permission("purchases", "create"))])
def create_pr(
    body: "PRCreate",
    current_user: dict = Depends(get_current_user),
    conn = Depends(get_master_db),
) -> IDResponse:
    from backend.models.purchases.schemas import PRCreate
    vendor = conn.execute(
        "SELECT id, name, account_id FROM vendors WHERE id=? AND is_active=1", (body.vendor_id,)
    ).fetchone()
    if not vendor:
        raise HTTPException(404, "Vendor not found.")

    pr_id     = str(uuid.uuid4())
    pr_number = next_number("PR", conn)
    grand_total = 0.0

    for i, line in enumerate(body.lines, 1):
        total_line = line.qty * line.unit_price
        conn.execute(
            """INSERT INTO purchase_return_lines
               (id, pr_id, line_no, grn_line_id, item_id, uom_id,
                qty, unit_price, total, reason)
               VALUES (?,?,?,?,?,?,?,?,?,?)""",
            (str(uuid.uuid4()), pr_id, i, line.grn_line_id,
             line.item_id, line.uom_id, line.qty, line.unit_price,
             round(total_line, 2), line.reason),
        )
        grand_total += total_line

    conn.execute(
        """INSERT INTO purchase_returns
           (id, pr_number, vendor_id, grn_id, po_id,
            return_date, status, grand_total, notes,
            created_by, created_at, updated_at)
           VALUES (?,?,?,?,?,?,'draft',?,?,?,?,?)""",
        (pr_id, pr_number, body.vendor_id, body.grn_id, body.po_id,
         body.return_date, round(grand_total, 2), body.notes,
         current_user.get("sub"), _now(), _now()),
    )
    return IDResponse(id=pr_id, number=pr_number)


@pr_router.post("/{pr_id}/post", dependencies=[Depends(require_permission("purchases", "post"))])
def post_pr(pr_id: str, current_user: dict = Depends(get_current_user), conn=Depends(get_master_db)):
    from backend.services.auto_jv import create_purchase_return_jv
    pr = conn.execute("SELECT * FROM purchase_returns WHERE id=?", (pr_id,)).fetchone()
    if not pr:
        raise HTTPException(404, "PR not found.")
    if pr["status"] != "draft":
        raise HTTPException(400, "PR must be draft to post.")

    vendor = conn.execute("SELECT name, account_id FROM vendors WHERE id=?", (pr["vendor_id"],)).fetchone()
    # Use a generic purchase account or first line's item purchase account
    pur_acc_row = conn.execute(
        """SELECT i.purchase_account_id FROM purchase_return_lines prl
           JOIN items i ON i.id = prl.item_id
           WHERE prl.pr_id = ? LIMIT 1""",
        (pr_id,),
    ).fetchone()
    purchase_acc = pur_acc_row["purchase_account_id"] if pur_acc_row else vendor["account_id"]

    jv_id = create_purchase_return_jv(
        conn,
        pr_id=pr_id, pr_number=pr["pr_number"], return_date=pr["return_date"],
        vendor_name=vendor["name"],
        payable_account_id=vendor["account_id"],
        purchase_account_id=purchase_acc,
        grand_total=pr["grand_total"],
        created_by=current_user.get("sub", "system"),
    )
    conn.execute(
        "UPDATE purchase_returns SET status='posted', jv_id=?, updated_at=? WHERE id=?",
        (jv_id, _now(), pr_id),
    )
    return MessageResponse(detail="Purchase return posted.")
