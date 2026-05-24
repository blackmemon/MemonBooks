"""receipts_router.py — Cash/Bank Receipts from customers."""

import uuid
from datetime import datetime, timezone
from typing import Optional

from fastapi import APIRouter, Depends, HTTPException, Query

from backend.security.auth import get_current_user
from backend.security.permissions import require_permission
from backend.database.connection import get_master_db
from backend.models.payments.schemas import ReceiptCreate, IDResponse, MessageResponse, PayListResponse
from backend.services.numbering import next_number
from backend.services.auto_jv import create_payment_jv

router = APIRouter()


def _now() -> str:
    return datetime.now(timezone.utc).isoformat()


@router.get("/", dependencies=[Depends(require_permission("payments", "view"))])
def list_receipts(
    customer_id: Optional[str] = Query(None),
    status: Optional[str] = Query(None),
    date_from: Optional[str] = Query(None),
    date_to: Optional[str] = Query(None),
    page: int = Query(1, ge=1),
    limit: int = Query(50, ge=1, le=500),
    conn=Depends(get_master_db),
):
    sql = """SELECT r.id, r.receipt_number, r.receipt_date, r.amount, r.status,
                    r.reference, r.created_at, c.name AS customer_name
             FROM receipts r JOIN customers c ON c.id = r.customer_id WHERE 1=1"""
    params: list = []
    if customer_id:
        sql += " AND r.customer_id = ?"; params.append(customer_id)
    if status:
        sql += " AND r.status = ?"; params.append(status)
    if date_from:
        sql += " AND r.receipt_date >= ?"; params.append(date_from)
    if date_to:
        sql += " AND r.receipt_date <= ?"; params.append(date_to)
    total = conn.execute(f"SELECT COUNT(*) FROM ({sql})", params).fetchone()[0]
    sql += " ORDER BY r.created_at DESC LIMIT ? OFFSET ?"
    params += [limit, (page - 1) * limit]
    return PayListResponse(items=[dict(r) for r in conn.execute(sql, params).fetchall()],
                           total=total, page=page, limit=limit)


@router.get("/{receipt_id}", dependencies=[Depends(require_permission("payments", "view"))])
def get_receipt(receipt_id: str, conn=Depends(get_master_db)):
    row = conn.execute(
        "SELECT r.*, c.name AS customer_name FROM receipts r JOIN customers c ON c.id = r.customer_id WHERE r.id = ?",
        (receipt_id,),
    ).fetchone()
    if not row:
        raise HTTPException(404, "Receipt not found.")
    return dict(row)


@router.post("/", dependencies=[Depends(require_permission("payments", "create"))])
def create_receipt(body: ReceiptCreate, current_user: dict = Depends(get_current_user), conn=Depends(get_master_db)) -> IDResponse:
    cust = conn.execute("SELECT id, name, account_id FROM customers WHERE id=? AND is_active=1", (body.customer_id,)).fetchone()
    if not cust:
        raise HTTPException(404, "Customer not found.")
    receipt_id = str(uuid.uuid4())
    receipt_number = next_number("CR", conn)
    receivable_acc = body.receivable_account_id or cust["account_id"]
    conn.execute(
        """INSERT INTO receipts (id, receipt_number, customer_id, receipt_date, amount,
           bank_account_id, receivable_account_id, si_id, reference, status, notes, created_by, created_at, updated_at)
           VALUES (?,?,?,?,?,?,?,?,?,'draft',?,?,?,?)""",
        (receipt_id, receipt_number, body.customer_id, body.receipt_date, body.amount,
         body.bank_account_id, receivable_acc, body.si_id, body.reference,
         body.notes, current_user.get("sub"), _now(), _now()),
    )
    return IDResponse(id=receipt_id, number=receipt_number)


@router.post("/{receipt_id}/post", dependencies=[Depends(require_permission("payments", "post"))])
def post_receipt(receipt_id: str, current_user: dict = Depends(get_current_user), conn=Depends(get_master_db)):
    rec = conn.execute("SELECT * FROM receipts WHERE id=?", (receipt_id,)).fetchone()
    if not rec or rec["status"] != "draft":
        raise HTTPException(400, "Only draft receipts can be posted.")
    cust = conn.execute("SELECT name FROM customers WHERE id=?", (rec["customer_id"],)).fetchone()
    jv_id = create_payment_jv(conn, payment_id=receipt_id, payment_number=rec["receipt_number"],
        payment_date=rec["receipt_date"], payee_name=cust["name"] if cust else "Unknown",
        bank_account_id=rec["bank_account_id"], payable_account_id=rec["receivable_account_id"],
        amount=rec["amount"], wht_amount=0.0, wht_account_id=None,
        created_by=current_user.get("sub", "system"), payment_type="receipt")
    if rec["si_id"]:
        conn.execute("UPDATE sale_invoices SET amount_paid=amount_paid+?, amount_due=MAX(0,amount_due-?) WHERE id=?",
                     (rec["amount"], rec["amount"], rec["si_id"]))
    conn.execute("UPDATE receipts SET status='posted', jv_id=?, posted_by=?, posted_at=?, updated_at=? WHERE id=?",
                 (jv_id, current_user.get("sub"), _now(), _now(), receipt_id))
    return MessageResponse(detail="Receipt posted.")


@router.post("/{receipt_id}/cancel", dependencies=[Depends(require_permission("payments", "edit"))])
def cancel_receipt(receipt_id: str, conn=Depends(get_master_db)) -> MessageResponse:
    rec = conn.execute("SELECT status FROM receipts WHERE id=?", (receipt_id,)).fetchone()
    if not rec or rec["status"] in ("posted", "cancelled"):
        raise HTTPException(400, "Cannot cancel receipt in current status.")
    conn.execute("UPDATE receipts SET status='cancelled', updated_at=? WHERE id=?", (_now(), receipt_id))
    return MessageResponse(detail="Receipt cancelled.")
