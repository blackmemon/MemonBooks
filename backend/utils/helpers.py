"""
helpers.py — Utility functions for MemonBook ERP.
"""

from __future__ import annotations
import re
from datetime import datetime, timezone
from typing import Any, Optional


def now_iso() -> str:
    """Return current UTC time as ISO 8601 string."""
    return datetime.now(timezone.utc).isoformat()


def today_str() -> str:
    """Return today's date as YYYY-MM-DD string."""
    return datetime.now().strftime("%Y-%m-%d")


def month_range(year: int, month: int) -> tuple[str, str]:
    """Return (first_day, last_day) for a given year/month."""
    import calendar
    last_day = calendar.monthrange(year, month)[1]
    return (
        f"{year:04d}-{month:02d}-01",
        f"{year:04d}-{month:02d}-{last_day:02d}",
    )


def sanitize_str(value: Any, max_length: int = 500) -> Optional[str]:
    """Strip and truncate a string value, return None if empty."""
    if value is None:
        return None
    s = str(value).strip()
    return s[:max_length] if s else None


def rows_to_dict_list(rows) -> list[dict]:
    """Convert SQLite Row objects to plain dicts."""
    return [dict(r) for r in rows]


def paginate(sql: str, params: list, conn, page: int, limit: int) -> tuple[int, list[dict]]:
    """
    Execute a count + paginated query.
    Returns (total_count, rows_as_dicts).
    """
    total = conn.execute(f"SELECT COUNT(*) FROM ({sql})", params).fetchone()[0]
    sql_p = sql + " LIMIT ? OFFSET ?"
    params_p = params + [limit, (page - 1) * limit]
    rows = [dict(r) for r in conn.execute(sql_p, params_p).fetchall()]
    return total, rows


def build_search_clause(term: str, columns: list[str]) -> tuple[str, list[str]]:
    """
    Build a WHERE clause fragment for a simple text search.
    Returns (sql_fragment, params_list).

    Example:
        sql_frag, params = build_search_clause("hello", ["name", "code"])
        # sql_frag = "(name LIKE ? OR code LIKE ?)"
        # params   = ["%hello%", "%hello%"]
    """
    if not term or not columns:
        return "", []
    like = f"%{term}%"
    fragment = " OR ".join(f"{col} LIKE ?" for col in columns)
    return f"({fragment})", [like] * len(columns)


def format_currency(amount: float, currency: str = "PKR") -> str:
    """Format a number as currency string."""
    return f"{currency} {amount:,.2f}"
