"""
stock_ledger.py — Stock ledger update service.

Records every inward/outward movement in stock_ledger table and
updates the running balance in item_stock (aggregated by item + location).

Movement types:
  GRN_ACCEPT   — GRN accepted qty inward
  GRN_REJECT   — GRN rejected qty (no stock effect — just audit)
  DC_DELIVER   — Delivery Challan delivery outward
  PR_RETURN    — Purchase Return outward (reversal of GRN)
  SR_RETURN    — Sale Return inward
  ADJ_IN       — Manual positive adjustment
  ADJ_OUT      — Manual negative adjustment
"""

import uuid
from datetime import datetime, timezone
from typing import Optional


def _now() -> str:
    return datetime.now(timezone.utc).isoformat()


def record_movement(
    conn,
    *,
    item_id:     str,
    location_id: Optional[str],
    move_type:   str,
    qty:         float,          # positive = inward, negative = outward
    unit_cost:   float = 0.0,
    source_type: str = "",
    source_id:   str = "",
    source_line_id: str = "",
    notes:       str = "",
    created_by:  str = "system",
) -> str:
    """
    Record one stock movement line.
    Returns the stock_ledger row id.
    """
    sl_id = str(uuid.uuid4())

    # Get current balance for this item + location
    row = conn.execute(
        "SELECT qty_balance FROM stock_ledger WHERE item_id=? AND location_id=? ORDER BY created_at DESC, rowid DESC LIMIT 1",
        (item_id, location_id or ""),
    ).fetchone()
    prev_balance = row["qty_balance"] if row else 0.0
    new_balance  = prev_balance + qty

    conn.execute(
        """INSERT INTO stock_ledger
           (id, item_id, location_id, move_type, qty_in, qty_out, qty_balance,
            unit_cost, source_type, source_id, source_line_id,
            notes, created_by, created_at)
           VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?)""",
        (
            sl_id,
            item_id,
            location_id or "",
            move_type,
            max(qty, 0),         # qty_in
            max(-qty, 0),        # qty_out
            new_balance,
            unit_cost,
            source_type,
            source_id,
            source_line_id,
            notes,
            created_by,
            _now(),
        ),
    )

    # Update aggregate stock table
    _upsert_item_stock(conn, item_id=item_id, location_id=location_id, balance=new_balance)
    return sl_id


def _upsert_item_stock(conn, *, item_id: str, location_id: Optional[str], balance: float) -> None:
    """Maintain item_stock aggregate table for fast queries."""
    loc = location_id or ""
    existing = conn.execute(
        "SELECT id FROM item_stock WHERE item_id=? AND location_id=?", (item_id, loc)
    ).fetchone()
    if existing:
        conn.execute(
            "UPDATE item_stock SET qty_on_hand=?, updated_at=? WHERE item_id=? AND location_id=?",
            (balance, _now(), item_id, loc),
        )
    else:
        conn.execute(
            "INSERT INTO item_stock (id, item_id, location_id, qty_on_hand, qty_reserved, updated_at) VALUES (?,?,?,?,0,?)",
            (str(uuid.uuid4()), item_id, loc, balance, _now()),
        )


def reserve_stock(conn, *, item_id: str, location_id: Optional[str], qty: float) -> None:
    """Increment reserved qty (e.g. on SO approval)."""
    loc = location_id or ""
    conn.execute(
        """INSERT INTO item_stock (id, item_id, location_id, qty_on_hand, qty_reserved, updated_at)
           VALUES (?,?,?,0,?,?)
           ON CONFLICT(item_id, location_id) DO UPDATE SET qty_reserved = qty_reserved + ?, updated_at = ?""",
        (str(uuid.uuid4()), item_id, loc, qty, _now(), qty, _now()),
    )


def release_reservation(conn, *, item_id: str, location_id: Optional[str], qty: float) -> None:
    """Decrement reserved qty (e.g. SO cancellation or DC posted)."""
    loc = location_id or ""
    conn.execute(
        "UPDATE item_stock SET qty_reserved = MAX(0, qty_reserved - ?), updated_at = ? WHERE item_id=? AND location_id=?",
        (qty, _now(), item_id, loc),
    )


def get_stock_balance(conn, item_id: str, location_id: Optional[str] = None) -> dict:
    """Return on-hand / reserved / available for item (optionally per location)."""
    if location_id:
        row = conn.execute(
            "SELECT qty_on_hand, qty_reserved FROM item_stock WHERE item_id=? AND location_id=?",
            (item_id, location_id),
        ).fetchone()
        if row:
            return {
                "qty_on_hand":  row["qty_on_hand"],
                "qty_reserved": row["qty_reserved"],
                "qty_available": row["qty_on_hand"] - row["qty_reserved"],
            }
        return {"qty_on_hand": 0.0, "qty_reserved": 0.0, "qty_available": 0.0}

    rows = conn.execute(
        "SELECT SUM(qty_on_hand) AS oh, SUM(qty_reserved) AS res FROM item_stock WHERE item_id=?",
        (item_id,),
    ).fetchone()
    oh  = rows["oh"]  or 0.0
    res = rows["res"] or 0.0
    return {"qty_on_hand": oh, "qty_reserved": res, "qty_available": oh - res}


def process_grn_lines(conn, *, grn_id: str, grn_number: str, grn_date: str, lines: list[dict], created_by: str) -> None:
    """
    Post accepted GRN quantities to stock ledger.
    lines: list of dicts with item_id, location_id, qty_accepted, unit_cost, grn_line_id
    """
    for line in lines:
        qty_accepted = line.get("qty_accepted", 0.0)
        if qty_accepted <= 0:
            continue
        record_movement(
            conn,
            item_id=line["item_id"],
            location_id=line.get("location_id"),
            move_type="GRN_ACCEPT",
            qty=qty_accepted,
            unit_cost=line.get("unit_cost", 0.0),
            source_type="grn",
            source_id=grn_id,
            source_line_id=line.get("grn_line_id", ""),
            notes=f"GRN {grn_number}",
            created_by=created_by,
        )


def process_dc_lines(conn, *, dc_id: str, dc_number: str, dc_date: str, lines: list[dict], created_by: str) -> None:
    """
    Deduct DC delivered quantities from stock ledger.
    """
    for line in lines:
        qty = line.get("qty_delivered", 0.0)
        if qty <= 0:
            continue
        record_movement(
            conn,
            item_id=line["item_id"],
            location_id=line.get("location_id"),
            move_type="DC_DELIVER",
            qty=-qty,
            unit_cost=line.get("unit_cost", 0.0),
            source_type="delivery_challan",
            source_id=dc_id,
            source_line_id=line.get("dc_line_id", ""),
            notes=f"DC {dc_number}",
            created_by=created_by,
        )
