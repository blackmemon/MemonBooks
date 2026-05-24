"""items_router.py — Items CRUD + price lists + stock balance."""

import uuid
from datetime import datetime, timezone
from typing import Optional

from fastapi import APIRouter, Depends, HTTPException, Query

from backend.security.auth import get_current_user
from backend.security.permissions import require_permission
from backend.database.connection import get_master_db
from backend.models.masters.schemas import (
    ItemCreate, ItemUpdate, PriceListCreate, IDResponse, MessageResponse
)

router = APIRouter()


def _now() -> str:
    return datetime.now(timezone.utc).isoformat()


@router.get("/", dependencies=[Depends(require_permission("masters", "view"))])
def list_items(
    search: Optional[str] = Query(None),
    item_type: Optional[str] = Query(None),
    active_only: bool = Query(True),
    page: int = Query(1, ge=1),
    limit: int = Query(50, ge=1, le=500),
    conn=Depends(get_master_db),
):
    sql = """
        SELECT i.id, i.code, i.name, i.item_type, i.standard_cost, i.standard_price,
               i.min_stock, i.max_stock, i.is_active, i.created_at,
               u.code AS uom_code
        FROM items i
        LEFT JOIN uom u ON u.id = i.uom_id
        WHERE i.is_deleted = 0
    """
    params: list = []
    if active_only:
        sql += " AND i.is_active=1"
    if item_type:
        sql += " AND i.item_type=?"
        params.append(item_type)
    if search:
        sql += " AND (i.name LIKE ? OR i.code LIKE ?)"
        s = f"%{search}%"
        params.extend([s, s])

    total = conn.execute(f"SELECT count(*) FROM ({sql})", params).fetchone()[0]
    offset = (page - 1) * limit
    sql += f" ORDER BY i.name LIMIT {limit} OFFSET {offset}"
    rows = [dict(r) for r in conn.execute(sql, params).fetchall()]
    return {"items": rows, "total": total, "page": page, "limit": limit}


@router.get("/dropdown", dependencies=[Depends(require_permission("masters", "view"))])
def items_dropdown(conn=Depends(get_master_db)):
    """Lightweight list for dropdowns."""
    rows = conn.execute(
        """SELECT i.id, i.code, i.name, i.standard_price, i.standard_cost,
                  u.code AS uom_code, i.sale_tax_id, i.purchase_tax_id
           FROM items i LEFT JOIN uom u ON u.id=i.uom_id
           WHERE i.is_active=1 AND i.is_deleted=0 ORDER BY i.name"""
    ).fetchall()
    return [dict(r) for r in rows]


@router.get("/{item_id}", dependencies=[Depends(require_permission("masters", "view"))])
def get_item(item_id: str, conn=Depends(get_master_db)):
    row = conn.execute("SELECT * FROM items WHERE id=? AND is_deleted=0", (item_id,)).fetchone()
    if not row:
        raise HTTPException(404, "Item not found.")
    item = dict(row)

    # Stock balance
    stock = conn.execute(
        "SELECT COALESCE(SUM(qty_in)-SUM(qty_out),0) AS balance FROM stock_ledger WHERE item_id=?",
        (item_id,),
    ).fetchone()
    item["stock_balance"] = stock["balance"] if stock else 0

    # Price lists
    prices = conn.execute(
        """SELECT pl.id, pl.customer_id, c.name AS customer_name,
                  pl.min_qty, pl.price, pl.valid_from, pl.valid_to
           FROM item_price_lists pl
           LEFT JOIN customers c ON c.id=pl.customer_id
           WHERE pl.item_id=? ORDER BY pl.min_qty""",
        (item_id,),
    ).fetchall()
    item["price_lists"] = [dict(p) for p in prices]

    return item


@router.get("/{item_id}/stock", dependencies=[Depends(require_permission("masters", "view"))])
def get_item_stock(item_id: str, conn=Depends(get_master_db)):
    """Stock balance per location."""
    rows = conn.execute(
        """SELECT l.name AS location, COALESCE(SUM(sl.qty_in)-SUM(sl.qty_out),0) AS balance
           FROM stock_ledger sl
           LEFT JOIN inventory_locations l ON l.id=sl.location_id
           WHERE sl.item_id=?
           GROUP BY sl.location_id ORDER BY l.name""",
        (item_id,),
    ).fetchall()
    total = conn.execute(
        "SELECT COALESCE(SUM(qty_in)-SUM(qty_out),0) FROM stock_ledger WHERE item_id=?",
        (item_id,),
    ).fetchone()[0]
    return {"total_balance": total, "by_location": [dict(r) for r in rows]}


@router.post("/", dependencies=[Depends(require_permission("masters", "create"))])
def create_item(
    body: ItemCreate,
    current_user: dict = Depends(get_current_user),
    conn=Depends(get_master_db),
) -> IDResponse:
    if conn.execute("SELECT id FROM items WHERE code=? AND is_deleted=0", (body.code,)).fetchone():
        raise HTTPException(409, f"Item code '{body.code}' already exists.")
    iid = str(uuid.uuid4())
    d = body.model_dump()
    conn.execute(
        """INSERT INTO items
           (id,code,name,description,item_type,uom_id,purchase_uom_id,sale_uom_id,
            purchase_account_id,sale_account_id,inventory_account_id,
            purchase_tax_id,sale_tax_id,standard_cost,standard_price,
            min_stock,max_stock,is_active,is_deleted,created_by,created_at,updated_at)
           VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,1,0,?,?,?)""",
        (iid, d["code"], d["name"], d.get("description"), d["item_type"],
         d.get("uom_id"), d.get("purchase_uom_id"), d.get("sale_uom_id"),
         d.get("purchase_account_id"), d.get("sale_account_id"), d.get("inventory_account_id"),
         d.get("purchase_tax_id"), d.get("sale_tax_id"),
         d["standard_cost"], d["standard_price"], d["min_stock"], d["max_stock"],
         current_user.get("sub"), _now(), _now()),
    )
    return IDResponse(id=iid)


@router.put("/{item_id}", dependencies=[Depends(require_permission("masters", "edit"))])
def update_item(
    item_id: str,
    body: ItemUpdate,
    conn=Depends(get_master_db),
) -> MessageResponse:
    if not conn.execute("SELECT id FROM items WHERE id=? AND is_deleted=0", (item_id,)).fetchone():
        raise HTTPException(404, "Item not found.")
    updates = {k: v for k, v in body.model_dump(exclude_none=True).items()}
    if not updates:
        return MessageResponse(detail="No changes.")
    updates["updated_at"] = _now()
    set_clause = ", ".join(f"{k}=?" for k in updates)
    conn.execute(f"UPDATE items SET {set_clause} WHERE id=?", (*updates.values(), item_id))
    return MessageResponse(detail="Item updated.")


@router.delete("/{item_id}", dependencies=[Depends(require_permission("masters", "delete"))])
def delete_item(item_id: str, conn=Depends(get_master_db)) -> MessageResponse:
    if conn.execute("SELECT id FROM stock_ledger WHERE item_id=? LIMIT 1", (item_id,)).fetchone():
        raise HTTPException(400, "Item has stock transactions.")
    conn.execute("UPDATE items SET is_deleted=1,is_active=0,updated_at=? WHERE id=?", (_now(), item_id))
    return MessageResponse(detail="Item deleted.")


# ── Price Lists ───────────────────────────────────────────────────────────────

@router.post("/{item_id}/prices", dependencies=[Depends(require_permission("masters", "edit"))])
def add_price(
    item_id: str,
    body: PriceListCreate,
    current_user: dict = Depends(get_current_user),
    conn=Depends(get_master_db),
) -> IDResponse:
    if not conn.execute("SELECT id FROM items WHERE id=? AND is_deleted=0", (item_id,)).fetchone():
        raise HTTPException(404, "Item not found.")
    pid = str(uuid.uuid4())
    conn.execute(
        """INSERT INTO item_price_lists(id,item_id,customer_id,min_qty,price,valid_from,valid_to,created_by,created_at)
           VALUES (?,?,?,?,?,?,?,?,?)""",
        (pid, item_id, body.customer_id, body.min_qty, body.price,
         body.valid_from, body.valid_to, current_user.get("sub"), _now()),
    )
    return IDResponse(id=pid)


@router.delete("/prices/{price_id}", dependencies=[Depends(require_permission("masters", "edit"))])
def delete_price(price_id: str, conn=Depends(get_master_db)) -> MessageResponse:
    conn.execute("DELETE FROM item_price_lists WHERE id=?", (price_id,))
    return MessageResponse(detail="Price deleted.")
