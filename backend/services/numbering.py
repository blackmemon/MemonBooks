"""
numbering.py — Auto document number generation.
Format: PREFIX-YYYY-NNNN  e.g. PO-2024-0001, GRN-2024-0001
Numbers reset each fiscal year. Thread-safe via DB row-level lock.
"""

from datetime import datetime, timezone
from typing import Optional


def _now_year() -> int:
    return datetime.now(timezone.utc).year


def next_number(doc_type: str, conn, prefix: Optional[str] = None) -> str:
    """
    Atomically increment and return the next document number for doc_type.
    Thread-safe: uses SQLite exclusive transaction + UPDATE WHERE.

    Args:
        doc_type: One of PO, GRN, PI, PR, SO, DC, SI, SR, CR, BR, CP, BP, JV, PAY
        conn:     Active SQLCipher/SQLite connection (already open)
        prefix:   Override prefix (default: from doc_sequences table)

    Returns:
        Formatted number string e.g. "PO-2025-0001"
    """
    year = _now_year()

    # Lock row for update using BEGIN IMMEDIATE
    conn.execute("BEGIN IMMEDIATE")
    try:
        row = conn.execute(
            "SELECT id, prefix, last_number, year FROM doc_sequences WHERE doc_type = ?",
            (doc_type.upper(),),
        ).fetchone()

        if not row:
            # Auto-create if missing
            import uuid
            new_id = str(uuid.uuid4())
            p = prefix or doc_type.upper()
            conn.execute(
                "INSERT INTO doc_sequences(id, doc_type, prefix, last_number, year, updated_at) VALUES(?,?,?,0,?,?)",
                (new_id, doc_type.upper(), p, year, datetime.now(timezone.utc).isoformat()),
            )
            row = conn.execute(
                "SELECT id, prefix, last_number, year FROM doc_sequences WHERE doc_type = ?",
                (doc_type.upper(),),
            ).fetchone()

        row_id      = row["id"]
        row_prefix  = prefix or row["prefix"]
        last_num    = row["last_number"]
        row_year    = row["year"]

        # Year rollover → reset sequence
        if row_year != year:
            last_num = 0

        next_num = last_num + 1

        conn.execute(
            "UPDATE doc_sequences SET last_number = ?, year = ?, updated_at = ? WHERE id = ?",
            (next_num, year, datetime.now(timezone.utc).isoformat(), row_id),
        )
        conn.commit()

        return f"{row_prefix}-{year}-{next_num:04d}"

    except Exception:
        conn.execute("ROLLBACK")
        raise


def peek_next(doc_type: str, conn) -> str:
    """
    Return what the next number WOULD be without incrementing.
    Used for previewing on forms.
    """
    year = _now_year()
    row = conn.execute(
        "SELECT prefix, last_number, year FROM doc_sequences WHERE doc_type = ?",
        (doc_type.upper(),),
    ).fetchone()

    if not row:
        return f"{doc_type.upper()}-{year}-0001"

    last_num = row["last_number"] if row["year"] == year else 0
    return f"{row['prefix']}-{year}-{last_num + 1:04d}"


def reset_sequence(doc_type: str, conn, start_from: int = 0) -> None:
    """Reset a sequence (admin only — use with caution)."""
    year = _now_year()
    conn.execute(
        "UPDATE doc_sequences SET last_number = ?, year = ?, updated_at = ? WHERE doc_type = ?",
        (start_from, year, datetime.now(timezone.utc).isoformat(), doc_type.upper()),
    )
    conn.commit()
