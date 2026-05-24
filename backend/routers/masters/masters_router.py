"""masters_router.py — UOM, Currencies, Tax Config, Payment Terms, Locations, Fiscal Years."""

import uuid
from datetime import datetime, timezone

from fastapi import APIRouter, Depends, HTTPException, Query
from typing import Optional

from backend.security.auth import get_current_user
from backend.security.permissions import require_permission
from backend.database.connection import get_master_db
from backend.models.masters.schemas import (
    UOMCreate, UOMUpdate, UOMConversionCreate,
    CurrencyCreate, CurrencyUpdate,
    TaxCreate, TaxUpdate,
    PaymentTermCreate,
    LocationCreate, LocationUpdate,
    FiscalYearCreate,
    IDResponse, MessageResponse,
)

router = APIRouter()


def _now() -> str:
    return datetime.now(timezone.utc).isoformat()


# ── UOM ───────────────────────────────────────────────────────────────────────

@router.get("/uom", dependencies=[Depends(require_permission("masters", "view"))])
def list_uom(active_only: bool = Query(True), conn=Depends(get_master_db)):
    sql = "SELECT id,code,name,is_active FROM uom"
    if active_only:
        sql += " WHERE is_active=1"
    sql += " ORDER BY code"
    return [dict(r) for r in conn.execute(sql).fetchall()]


@router.post("/uom", dependencies=[Depends(require_permission("masters", "create"))])
def create_uom(body: UOMCreate, current_user: dict = Depends(get_current_user), conn=Depends(get_master_db)) -> IDResponse:
    if conn.execute("SELECT id FROM uom WHERE code=?", (body.code,)).fetchone():
        raise HTTPException(409, f"UOM code '{body.code}' already exists.")
    uid = str(uuid.uuid4())
    conn.execute("INSERT INTO uom(id,code,name,is_active,created_at) VALUES(?,?,?,1,?)",
                 (uid, body.code, body.name, _now()))
    return IDResponse(id=uid)


@router.put("/uom/{uom_id}", dependencies=[Depends(require_permission("masters", "edit"))])
def update_uom(uom_id: str, body: UOMUpdate, conn=Depends(get_master_db)) -> MessageResponse:
    updates = {k: v for k, v in body.model_dump(exclude_none=True).items()}
    if updates:
        set_clause = ", ".join(f"{k}=?" for k in updates)
        conn.execute(f"UPDATE uom SET {set_clause} WHERE id=?", (*updates.values(), uom_id))
    return MessageResponse(detail="UOM updated.")


@router.post("/uom/conversions", dependencies=[Depends(require_permission("masters", "create"))])
def create_uom_conversion(body: UOMConversionCreate, conn=Depends(get_master_db)) -> IDResponse:
    cid = str(uuid.uuid4())
    conn.execute(
        "INSERT OR REPLACE INTO uom_conversions(id,from_uom_id,to_uom_id,factor,created_at) VALUES(?,?,?,?,?)",
        (cid, body.from_uom_id, body.to_uom_id, body.factor, _now()),
    )
    return IDResponse(id=cid)


# ── CURRENCIES ────────────────────────────────────────────────────────────────

@router.get("/currencies", dependencies=[Depends(require_permission("masters", "view"))])
def list_currencies(active_only: bool = Query(True), conn=Depends(get_master_db)):
    sql = "SELECT id,code,name,symbol,exchange_rate,is_base,is_active FROM currencies"
    if active_only:
        sql += " WHERE is_active=1"
    sql += " ORDER BY code"
    return [dict(r) for r in conn.execute(sql).fetchall()]


@router.post("/currencies", dependencies=[Depends(require_permission("masters", "create"))])
def create_currency(body: CurrencyCreate, conn=Depends(get_master_db)) -> IDResponse:
    if conn.execute("SELECT id FROM currencies WHERE code=?", (body.code,)).fetchone():
        raise HTTPException(409, f"Currency '{body.code}' already exists.")
    if body.is_base:
        conn.execute("UPDATE currencies SET is_base=0")
    cid = str(uuid.uuid4())
    conn.execute(
        "INSERT INTO currencies(id,code,name,symbol,exchange_rate,is_base,is_active,updated_at,created_at) VALUES(?,?,?,?,?,?,1,?,?)",
        (cid, body.code, body.name, body.symbol, body.exchange_rate, int(body.is_base), _now(), _now()),
    )
    return IDResponse(id=cid)


@router.put("/currencies/{currency_id}", dependencies=[Depends(require_permission("masters", "edit"))])
def update_currency(currency_id: str, body: CurrencyUpdate, conn=Depends(get_master_db)) -> MessageResponse:
    updates = {k: v for k, v in body.model_dump(exclude_none=True).items()}
    if updates:
        updates["updated_at"] = _now()
        set_clause = ", ".join(f"{k}=?" for k in updates)
        conn.execute(f"UPDATE currencies SET {set_clause} WHERE id=?", (*updates.values(), currency_id))
    return MessageResponse(detail="Currency updated.")


# ── TAX CONFIG ────────────────────────────────────────────────────────────────

@router.get("/tax", dependencies=[Depends(require_permission("masters", "view"))])
def list_tax(applies_to: Optional[str] = Query(None), active_only: bool = Query(True), conn=Depends(get_master_db)):
    sql = "SELECT id,name,tax_type,rate,applies_to,is_active FROM tax_config WHERE 1=1"
    params: list = []
    if active_only:
        sql += " AND is_active=1"
    if applies_to:
        sql += " AND (applies_to=? OR applies_to='both')"
        params.append(applies_to)
    sql += " ORDER BY name"
    return [dict(r) for r in conn.execute(sql, params).fetchall()]


@router.post("/tax", dependencies=[Depends(require_permission("masters", "create"))])
def create_tax(body: TaxCreate, current_user: dict = Depends(get_current_user), conn=Depends(get_master_db)) -> IDResponse:
    tid = str(uuid.uuid4())
    conn.execute(
        "INSERT INTO tax_config(id,name,tax_type,rate,account_id,applies_to,is_active,created_by,created_at) VALUES(?,?,?,?,?,?,1,?,?)",
        (tid, body.name, body.tax_type, body.rate, body.account_id, body.applies_to, current_user.get("sub"), _now()),
    )
    return IDResponse(id=tid)


@router.put("/tax/{tax_id}", dependencies=[Depends(require_permission("masters", "edit"))])
def update_tax(tax_id: str, body: TaxUpdate, conn=Depends(get_master_db)) -> MessageResponse:
    updates = {k: v for k, v in body.model_dump(exclude_none=True).items()}
    if updates:
        set_clause = ", ".join(f"{k}=?" for k in updates)
        conn.execute(f"UPDATE tax_config SET {set_clause} WHERE id=?", (*updates.values(), tax_id))
    return MessageResponse(detail="Tax updated.")


# ── PAYMENT TERMS ─────────────────────────────────────────────────────────────

@router.get("/payment-terms", dependencies=[Depends(require_permission("masters", "view"))])
def list_payment_terms(active_only: bool = Query(True), conn=Depends(get_master_db)):
    sql = "SELECT id,name,due_days,discount_days,discount_pct,is_active FROM payment_terms"
    if active_only:
        sql += " WHERE is_active=1"
    sql += " ORDER BY due_days"
    return [dict(r) for r in conn.execute(sql).fetchall()]


@router.post("/payment-terms", dependencies=[Depends(require_permission("masters", "create"))])
def create_payment_term(body: PaymentTermCreate, conn=Depends(get_master_db)) -> IDResponse:
    pid = str(uuid.uuid4())
    conn.execute(
        "INSERT INTO payment_terms(id,name,due_days,discount_days,discount_pct,is_active,created_at) VALUES(?,?,?,?,?,1,?)",
        (pid, body.name, body.due_days, body.discount_days, body.discount_pct, _now()),
    )
    return IDResponse(id=pid)


# ── INVENTORY LOCATIONS ───────────────────────────────────────────────────────

@router.get("/locations", dependencies=[Depends(require_permission("masters", "view"))])
def list_locations(active_only: bool = Query(True), conn=Depends(get_master_db)):
    sql = "SELECT id,code,name,address,is_active FROM inventory_locations"
    if active_only:
        sql += " WHERE is_active=1"
    sql += " ORDER BY name"
    return [dict(r) for r in conn.execute(sql).fetchall()]


@router.post("/locations", dependencies=[Depends(require_permission("masters", "create"))])
def create_location(body: LocationCreate, current_user: dict = Depends(get_current_user), conn=Depends(get_master_db)) -> IDResponse:
    if conn.execute("SELECT id FROM inventory_locations WHERE code=?", (body.code,)).fetchone():
        raise HTTPException(409, f"Location code '{body.code}' already exists.")
    lid = str(uuid.uuid4())
    conn.execute(
        "INSERT INTO inventory_locations(id,code,name,address,is_active,created_by,created_at) VALUES(?,?,?,?,1,?,?)",
        (lid, body.code, body.name, body.address, current_user.get("sub"), _now()),
    )
    return IDResponse(id=lid)


@router.put("/locations/{location_id}", dependencies=[Depends(require_permission("masters", "edit"))])
def update_location(location_id: str, body: LocationUpdate, conn=Depends(get_master_db)) -> MessageResponse:
    updates = {k: v for k, v in body.model_dump(exclude_none=True).items()}
    if updates:
        set_clause = ", ".join(f"{k}=?" for k in updates)
        conn.execute(f"UPDATE inventory_locations SET {set_clause} WHERE id=?", (*updates.values(), location_id))
    return MessageResponse(detail="Location updated.")


# ── FISCAL YEARS ──────────────────────────────────────────────────────────────

@router.get("/fiscal-years", dependencies=[Depends(require_permission("masters", "view"))])
def list_fiscal_years(conn=Depends(get_master_db)):
    rows = conn.execute("SELECT id,name,start_date,end_date,is_active,is_closed FROM fiscal_years ORDER BY start_date DESC").fetchall()
    return [dict(r) for r in rows]


@router.post("/fiscal-years", dependencies=[Depends(require_permission("system", "create"))])
def create_fiscal_year(body: FiscalYearCreate, current_user: dict = Depends(get_current_user), conn=Depends(get_master_db)) -> IDResponse:
    fid = str(uuid.uuid4())
    conn.execute(
        "INSERT INTO fiscal_years(id,name,start_date,end_date,is_active,is_closed,created_by,created_at) VALUES(?,?,?,?,1,0,?,?)",
        (fid, body.name, body.start_date, body.end_date, current_user.get("sub"), _now()),
    )
    return IDResponse(id=fid)


@router.post("/fiscal-years/{fy_id}/close", dependencies=[Depends(require_permission("system", "post"))])
def close_fiscal_year(fy_id: str, current_user: dict = Depends(get_current_user), conn=Depends(get_master_db)) -> MessageResponse:
    row = conn.execute("SELECT id,is_closed FROM fiscal_years WHERE id=?", (fy_id,)).fetchone()
    if not row:
        raise HTTPException(404, "Fiscal year not found.")
    if row["is_closed"]:
        raise HTTPException(400, "Fiscal year is already closed.")
    conn.execute(
        "UPDATE fiscal_years SET is_closed=1,is_active=0,closed_by=?,closed_at=? WHERE id=?",
        (current_user.get("sub"), _now(), fy_id),
    )
    return MessageResponse(detail="Fiscal year closed.")


# ── DOC SEQUENCES ─────────────────────────────────────────────────────────────

@router.get("/doc-sequences", dependencies=[Depends(require_permission("system", "view"))])
def list_doc_sequences(conn=Depends(get_master_db)):
    rows = conn.execute("SELECT doc_type,prefix,last_number,year FROM doc_sequences ORDER BY doc_type").fetchall()
    return [dict(r) for r in rows]


@router.get("/doc-sequences/{doc_type}/preview", dependencies=[Depends(require_permission("masters", "view"))])
def preview_next_number(doc_type: str, conn=Depends(get_master_db)):
    from backend.services.numbering import peek_next
    return {"next_number": peek_next(doc_type.upper(), conn)}
