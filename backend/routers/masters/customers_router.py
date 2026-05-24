"""customers_router.py — Customers CRUD with field-level encryption."""

import uuid
from datetime import datetime, timezone
from typing import Optional

from fastapi import APIRouter, Depends, HTTPException, Query

from backend.security.auth import get_current_user
from backend.security.permissions import require_permission
from backend.database.connection import get_master_db
from backend.security.encryption import encrypt_record_fields, decrypt_record_fields
from backend.models.masters.schemas import (
    CustomerCreate, CustomerUpdate, CustomerOut, CustomerDetail, IDResponse, MessageResponse
)
from backend.config import settings

router = APIRouter()


def _now() -> str:
    return datetime.now(timezone.utc).isoformat()


def _enc(record: dict) -> dict:
    return encrypt_record_fields("customers", record, settings.FIELD_ENCRYPT_KEY)


def _dec(record: dict) -> dict:
    return decrypt_record_fields("customers", record, settings.FIELD_ENCRYPT_KEY)


@router.get("/", dependencies=[Depends(require_permission("masters", "view"))])
def list_customers(
    search: Optional[str] = Query(None),
    active_only: bool = Query(True),
    page: int = Query(1, ge=1),
    limit: int = Query(50, ge=1, le=500),
    conn=Depends(get_master_db),
):
    offset = (page - 1) * limit
    sql = "SELECT id,code,name,ntn,strn,phone,email,city,credit_limit,credit_days,is_active,created_at FROM customers WHERE is_deleted=0"
    params: list = []
    if active_only:
        sql += " AND is_active=1"
    if search:
        sql += " AND (name LIKE ? OR code LIKE ? OR phone LIKE ?)"
        s = f"%{search}%"
        params.extend([s, s, s])

    total = conn.execute(f"SELECT count(*) FROM ({sql})", params).fetchone()[0]
    sql += f" ORDER BY name LIMIT {limit} OFFSET {offset}"
    rows = [dict(r) for r in conn.execute(sql, params).fetchall()]
    return {"items": rows, "total": total, "page": page, "limit": limit}


@router.get("/{customer_id}", dependencies=[Depends(require_permission("masters", "view"))])
def get_customer(customer_id: str, conn=Depends(get_master_db)):
    row = conn.execute("SELECT * FROM customers WHERE id=? AND is_deleted=0", (customer_id,)).fetchone()
    if not row:
        raise HTTPException(404, "Customer not found.")
    return _dec(dict(row))


@router.post("/", dependencies=[Depends(require_permission("masters", "create"))])
def create_customer(
    body: CustomerCreate,
    current_user: dict = Depends(get_current_user),
    conn=Depends(get_master_db),
) -> IDResponse:
    if conn.execute("SELECT id FROM customers WHERE code=? AND is_deleted=0", (body.code,)).fetchone():
        raise HTTPException(409, f"Customer code '{body.code}' already exists.")
    cid = str(uuid.uuid4())
    data = body.model_dump()
    data = _enc(data)
    conn.execute(
        """INSERT INTO customers
           (id,code,name,ntn,strn,cnic,phone,email,address,city,bank_iban,
            credit_limit,credit_days,payment_term_id,currency_id,account_id,
            is_active,is_deleted,created_by,created_at,updated_at)
           VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,1,0,?,?,?)""",
        (cid, data["code"], data["name"], data.get("ntn"), data.get("strn"),
         data.get("cnic"), data.get("phone"), data.get("email"), data.get("address"),
         data.get("city"), data.get("bank_iban"), data["credit_limit"], data["credit_days"],
         data.get("payment_term_id"), data.get("currency_id"), data.get("account_id"),
         current_user.get("sub"), _now(), _now()),
    )
    return IDResponse(id=cid)


@router.put("/{customer_id}", dependencies=[Depends(require_permission("masters", "edit"))])
def update_customer(
    customer_id: str,
    body: CustomerUpdate,
    conn=Depends(get_master_db),
) -> MessageResponse:
    if not conn.execute("SELECT id FROM customers WHERE id=? AND is_deleted=0", (customer_id,)).fetchone():
        raise HTTPException(404, "Customer not found.")
    updates = {k: v for k, v in body.model_dump(exclude_none=True).items()}
    if not updates:
        return MessageResponse(detail="No changes.")
    updates = _enc(updates)
    updates["updated_at"] = _now()
    set_clause = ", ".join(f"{k}=?" for k in updates)
    conn.execute(f"UPDATE customers SET {set_clause} WHERE id=?", (*updates.values(), customer_id))
    return MessageResponse(detail="Customer updated.")


@router.delete("/{customer_id}", dependencies=[Depends(require_permission("masters", "delete"))])
def delete_customer(customer_id: str, conn=Depends(get_master_db)) -> MessageResponse:
    if conn.execute("SELECT id FROM sale_invoices WHERE customer_id=? LIMIT 1", (customer_id,)).fetchone():
        raise HTTPException(400, "Customer has sale transactions and cannot be deleted.")
    conn.execute("UPDATE customers SET is_deleted=1,is_active=0,updated_at=? WHERE id=?", (_now(), customer_id))
    return MessageResponse(detail="Customer deleted.")
