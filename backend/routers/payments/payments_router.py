"""payments_router.py — Cash/Bank Payments to vendors + WHT."""

import uuid
from datetime import datetime, timezone
from typing import Optional

from fastapi import APIRouter, Depends, HTTPException, Query

from backend.security.auth import get_current_user
from backend.security.permissions import require_permission
from backend.database.connection import get_master_db
from backend.models.payments.schemas import PaymentCreate, IDResponse, MessageResponse, PayListResponse
from backend.services.numbering import next_number
from backend.services.auto_jv import create_payment_jv

router = APIRouter()


def _now() -> str:
    return datetime.now(timezone.utc).isoformat()


@router.get("/", dependencies=[Depends(require_permission("payments", "view"))])
def list_payments(
    vendor_id: Optional[str] = Query(None),
    status: Optional[str] = Query(None),
    date_from: Optional[str] = Query(None),
    date_to: Optional[str] = Query(None),
    page: int = Query(1, ge=1),
    limit: int = Query(50, ge=1, le=500),
    conn=Depends(get_master_db),
):
    sql = """SELECT p.id, p.payment_number, p.payment_date, p.amount, p.wht_amount,
                    p.status, p.reference, p.created_at, v.name AS vendor_name
             FROM payments p JOIN vendors v ON v.id = p.vendor_id WHERE 1=1"""
    params: list = []
    if vendor_id:
        sql += " AND p.vendor_id = ?"; params.append(vendor_id)
    if status:
        sql += " AND p.status = ?"; params.append(status)
    if date_from:
        sql += " AND p.payment_date >= ?"; params.append(date_from)
    if date_to:
        sql += " AND p.payment_date <= ?"; params.append(date_to)
    total = conn.execute(f"SELECT COUNT(*) FROM ({sql})", params).fetchone()[0]
    sql += " ORDER BY p.created_at DESC LIMIT ? OFFSET ?"
    params += [limit, (page - 1) * limit]
    return PayListResponse(items=[dict(r) for r in conn.execute(sql, params).fetchall()],
                           total=total, page=page, limit=limit)


@router.get("/{payment_id}", dependencies=[Depends(require_permission("payments", "view"))])
def get_payment(payment_id: str, conn=Depends(get_master_db)):
    row = conn.execute(
        "SELECT p.*, v.name AS vendor_name FROM payments p JOIN vendors v ON v.id=p.vendor_id WHERE p.id=?",
        (payment_id,),
    ).fetchone()
    if not row:
        raise HTTPException(404, "Payment not found.")
    return dict(row)


@router.post("/", dependencies=[Depends(require_permission("payments", "create"))])
def create_payment(body: PaymentCreate, current_user: dict = Depends(get_current_user), conn=Depends(get_master_db)) -> IDResponse:
    vendor = conn.execute("SELECT id, name, account_id, wht_account_id FROM vendors WHERE id=? AND is_active=1", (body.vendor_id,)).fetchone()
    if not vendor:
        raise HTTPException(404, "Vendor not found.")
    payment_id = str(uuid.uuid4())
    payment_number = next_number("CP", conn)
    payable_acc = body.payable_account_id or vendor["account_id"]
    wht_acc = body.wht_account_id or vendor["wht_account_id"]
    conn.execute(
        """INSERT INTO payments (id, payment_number, vendor_id, payment_date, amount, wht_amount,
           bank_account_id, payable_account_id, wht_account_id, pi_id, reference, status, notes, created_by, created_at, updated_at)
           VALUES (?,?,?,?,?,?,?,?,?,?,?,'draft',?,?,?,?)""",
        (payment_id, payment_number, body.vendor_id, body.payment_date, body.amount,
         body.wht_amount, body.bank_account_id, payable_acc, wht_acc, body.pi_id,
         body.reference, body.notes, current_user.get("sub"), _now(), _now()),
    )
    return IDResponse(id=payment_id, number=payment_number)


@router.post("/{payment_id}/post", dependencies=[Depends(require_permission("payments", "post"))])
def post_payment(payment_id: str, current_user: dict = Depends(get_current_user), conn=Depends(get_master_db)):
    pay = conn.execute("SELECT * FROM payments WHERE id=?", (payment_id,)).fetchone()
    if not pay or pay["status"] != "draft":
        raise HTTPException(400, "Only draft payments can be posted.")
    vendor = conn.execute("SELECT name FROM vendors WHERE id=?", (pay["vendor_id"],)).fetchone()
    jv_id = create_payment_jv(conn, payment_id=payment_id, payment_number=pay["payment_number"],
        payment_date=pay["payment_date"], payee_name=vendor["name"] if vendor else "Unknown",
        bank_account_id=pay["bank_account_id"], payable_account_id=pay["payable_account_id"],
        amount=pay["amount"], wht_amount=pay["wht_amount"] or 0.0, wht_account_id=pay["wht_account_id"],
        created_by=current_user.get("sub", "system"), payment_type="payment")
    if pay["pi_id"]:
        conn.execute("UPDATE purchase_invoices SET amount_paid=amount_paid+?, amount_due=MAX(0,amount_due-?) WHERE id=?",
                     (pay["amount"], pay["amount"], pay["pi_id"]))
    conn.execute("UPDATE payments SET status='posted', jv_id=?, posted_by=?, posted_at=?, updated_at=? WHERE id=?",
                 (jv_id, current_user.get("sub"), _now(), _now(), payment_id))
    return MessageResponse(detail="Payment posted.")


@router.post("/{payment_id}/cancel", dependencies=[Depends(require_permission("payments", "edit"))])
def cancel_payment(payment_id: str, conn=Depends(get_master_db)) -> MessageResponse:
    pay = conn.execute("SELECT status FROM payments WHERE id=?", (payment_id,)).fetchone()
    if not pay or pay["status"] in ("posted", "cancelled"):
        raise HTTPException(400, "Cannot cancel payment in current status.")
    conn.execute("UPDATE payments SET status='cancelled', updated_at=? WHERE id=?", (_now(), payment_id))
    return MessageResponse(detail="Payment cancelled.")
