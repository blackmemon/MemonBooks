"""ledger_router.py — General Ledger, Trial Balance, Account Balance."""

from typing import Optional
from fastapi import APIRouter, Depends, Query
from backend.security.permissions import require_permission
from backend.database.connection import get_master_db

router = APIRouter()


@router.get("/ledger", dependencies=[Depends(require_permission("accounts", "view"))])
def get_ledger(
    account_id: str = Query(..., description="COA account ID"),
    date_from:  Optional[str] = Query(None),
    date_to:    Optional[str] = Query(None),
    page:  int = Query(1, ge=1),
    limit: int = Query(100, ge=1, le=500),
    conn=Depends(get_master_db),
):
    """Return all posted JV lines for a given account, with running balance."""
    sql = """SELECT jvl.id, jvl.debit, jvl.credit, jvl.narration,
                    jv.jv_number, jv.jv_date, jv.source_type, jv.status
             FROM journal_voucher_lines jvl
             JOIN journal_vouchers jv ON jv.id = jvl.jv_id
             WHERE jvl.account_id = ? AND jv.status = 'posted'"""
    params: list = [account_id]
    if date_from:
        sql += " AND jv.jv_date >= ?"; params.append(date_from)
    if date_to:
        sql += " AND jv.jv_date <= ?"; params.append(date_to)

    total_row = conn.execute(f"SELECT COUNT(*) FROM ({sql})", params).fetchone()[0]
    sql += " ORDER BY jv.jv_date, jv.jv_number LIMIT ? OFFSET ?"
    params += [limit, (page - 1) * limit]
    rows = [dict(r) for r in conn.execute(sql, params).fetchall()]

    # Opening balance (all transactions before date_from)
    opening_dr = opening_cr = 0.0
    if date_from:
        ob = conn.execute(
            """SELECT COALESCE(SUM(jvl.debit),0) AS dr, COALESCE(SUM(jvl.credit),0) AS cr
               FROM journal_voucher_lines jvl
               JOIN journal_vouchers jv ON jv.id=jvl.jv_id
               WHERE jvl.account_id=? AND jv.status='posted' AND jv.jv_date<?""",
            (account_id, date_from),
        ).fetchone()
        opening_dr = ob["dr"]; opening_cr = ob["cr"]

    # Running balance
    running = opening_dr - opening_cr
    for row in rows:
        running += row["debit"] - row["credit"]
        row["running_balance"] = round(running, 2)

    # Account info
    acc = conn.execute("SELECT code, name, account_type FROM chart_of_accounts WHERE id=?", (account_id,)).fetchone()
    return {
        "account": dict(acc) if acc else {},
        "opening_balance": round(opening_dr - opening_cr, 2),
        "items":  rows,
        "total":  total_row,
        "page":   page,
        "limit":  limit,
    }


@router.get("/trial-balance", dependencies=[Depends(require_permission("accounts", "view"))])
def trial_balance(
    date_from: Optional[str] = Query(None),
    date_to:   Optional[str] = Query(None),
    conn=Depends(get_master_db),
):
    """Return trial balance: one row per account with total Dr, Cr, Net."""
    sql = """SELECT coa.id, coa.code, coa.name, coa.account_type, coa.level,
                    COALESCE(SUM(jvl.debit),0)  AS total_debit,
                    COALESCE(SUM(jvl.credit),0) AS total_credit
             FROM chart_of_accounts coa
             LEFT JOIN journal_voucher_lines jvl ON jvl.account_id = coa.id
             LEFT JOIN journal_vouchers jv ON jv.id = jvl.jv_id AND jv.status = 'posted'"""
    params: list = []
    if date_from:
        sql += " AND (jv.jv_date IS NULL OR jv.jv_date >= ?)"; params.append(date_from)
    if date_to:
        sql += " AND (jv.jv_date IS NULL OR jv.jv_date <= ?)"; params.append(date_to)
    sql += " GROUP BY coa.id, coa.code, coa.name, coa.account_type, coa.level ORDER BY coa.code"

    rows = [dict(r) for r in conn.execute(sql, params).fetchall()]
    for row in rows:
        row["net"] = round(row["total_debit"] - row["total_credit"], 2)

    total_dr = sum(r["total_debit"]  for r in rows)
    total_cr = sum(r["total_credit"] for r in rows)
    return {
        "items":       rows,
        "total_debit": round(total_dr, 2),
        "total_credit": round(total_cr, 2),
        "is_balanced": abs(total_dr - total_cr) < 0.01,
    }


@router.get("/balance/{account_id}", dependencies=[Depends(require_permission("accounts", "view"))])
def account_balance(account_id: str, as_of: Optional[str] = Query(None), conn=Depends(get_master_db)):
    """Quick balance for a single account."""
    sql = """SELECT COALESCE(SUM(jvl.debit),0) AS dr, COALESCE(SUM(jvl.credit),0) AS cr
             FROM journal_voucher_lines jvl
             JOIN journal_vouchers jv ON jv.id=jvl.jv_id
             WHERE jvl.account_id=? AND jv.status='posted'"""
    params: list = [account_id]
    if as_of:
        sql += " AND jv.jv_date <= ?"; params.append(as_of)
    row = conn.execute(sql, params).fetchone()
    net = round((row["dr"] or 0) - (row["cr"] or 0), 2)
    return {"account_id": account_id, "debit": row["dr"], "credit": row["cr"], "net": net}
