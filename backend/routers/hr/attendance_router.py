"""attendance_router.py — Employee Attendance tracking."""

import uuid
from datetime import datetime, timezone
from typing import Optional

from fastapi import APIRouter, Depends, HTTPException, Query

from backend.security.auth import get_current_user
from backend.security.permissions import require_permission
from backend.database.connection import get_master_db
from backend.models.hr.schemas import (
    AttendanceMarkRequest, AttendanceBulkRequest,
    MessageResponse, HRListResponse,
)

router = APIRouter()


def _now() -> str:
    return datetime.now(timezone.utc).isoformat()


@router.get("/", dependencies=[Depends(require_permission("hr", "view"))])
def list_attendance(
    employee_id: Optional[str] = Query(None),
    month:       Optional[int] = Query(None, ge=1, le=12),
    year:        Optional[int] = Query(None, ge=2000),
    date_from:   Optional[str] = Query(None),
    date_to:     Optional[str] = Query(None),
    status:      Optional[str] = Query(None),
    page:  int = Query(1, ge=1),
    limit: int = Query(100, ge=1, le=500),
    conn=Depends(get_master_db),
) -> HRListResponse:
    sql = """SELECT a.id, a.employee_id, a.att_date, a.status,
                    a.check_in, a.check_out, a.overtime_hours, a.notes,
                    e.emp_code, e.full_name AS employee_name
             FROM attendance a
             JOIN employees e ON e.id = a.employee_id
             WHERE 1=1"""
    params: list = []
    if employee_id:
        sql += " AND a.employee_id = ?"; params.append(employee_id)
    if status:
        sql += " AND a.status = ?"; params.append(status)
    if year and month:
        month_str = f"{year:04d}-{month:02d}"
        sql += " AND a.att_date LIKE ?"; params.append(f"{month_str}-%")
    else:
        if date_from:
            sql += " AND a.att_date >= ?"; params.append(date_from)
        if date_to:
            sql += " AND a.att_date <= ?"; params.append(date_to)
    total = conn.execute(f"SELECT COUNT(*) FROM ({sql})", params).fetchone()[0]
    sql += " ORDER BY a.att_date DESC, e.emp_code LIMIT ? OFFSET ?"
    params += [limit, (page - 1) * limit]
    return HRListResponse(items=[dict(r) for r in conn.execute(sql, params).fetchall()], total=total, page=page, limit=limit)


@router.post("/", dependencies=[Depends(require_permission("hr", "create"))])
def mark_attendance(
    body: AttendanceMarkRequest,
    current_user: dict = Depends(get_current_user),
    conn=Depends(get_master_db),
) -> MessageResponse:
    emp = conn.execute("SELECT id FROM employees WHERE id=? AND is_active=1", (body.employee_id,)).fetchone()
    if not emp:
        raise HTTPException(404, "Employee not found or inactive.")

    # Upsert: if a record exists for this emp+date, update it
    existing = conn.execute(
        "SELECT id FROM attendance WHERE employee_id=? AND att_date=?",
        (body.employee_id, body.att_date),
    ).fetchone()

    if existing:
        conn.execute(
            """UPDATE attendance SET status=?, check_in=?, check_out=?, overtime_hours=?, notes=?
               WHERE id=?""",
            (body.status, body.check_in, body.check_out, body.overtime_hours, body.notes, existing["id"]),
        )
    else:
        conn.execute(
            """INSERT INTO attendance (id, employee_id, att_date, status, check_in, check_out, overtime_hours, notes)
               VALUES (?,?,?,?,?,?,?,?)""",
            (str(uuid.uuid4()), body.employee_id, body.att_date, body.status,
             body.check_in, body.check_out, body.overtime_hours, body.notes),
        )
    return MessageResponse(detail="Attendance recorded.")


@router.post("/bulk", dependencies=[Depends(require_permission("hr", "create"))])
def mark_attendance_bulk(
    body: AttendanceBulkRequest,
    current_user: dict = Depends(get_current_user),
    conn=Depends(get_master_db),
) -> MessageResponse:
    """Mark same status for multiple employees on a given date."""
    count = 0
    for emp_id in body.employee_ids:
        emp = conn.execute("SELECT id FROM employees WHERE id=? AND is_active=1", (emp_id,)).fetchone()
        if not emp:
            continue
        existing = conn.execute(
            "SELECT id FROM attendance WHERE employee_id=? AND att_date=?",
            (emp_id, body.att_date),
        ).fetchone()
        if existing:
            conn.execute(
                "UPDATE attendance SET status=?, check_in=?, check_out=? WHERE id=?",
                (body.status, body.check_in, body.check_out, existing["id"]),
            )
        else:
            conn.execute(
                """INSERT INTO attendance (id, employee_id, att_date, status, check_in, check_out, overtime_hours)
                   VALUES (?,?,?,?,?,?,0)""",
                (str(uuid.uuid4()), emp_id, body.att_date, body.status, body.check_in, body.check_out),
            )
        count += 1
    return MessageResponse(detail=f"Attendance marked for {count} employees.")


@router.get("/summary/{employee_id}", dependencies=[Depends(require_permission("hr", "view"))])
def attendance_summary(
    employee_id: str,
    month: int = Query(..., ge=1, le=12),
    year:  int = Query(..., ge=2000),
    conn=Depends(get_master_db),
) -> dict:
    """Return attendance summary (counts per status) for a month."""
    month_str = f"{year:04d}-{month:02d}"
    rows = conn.execute(
        """SELECT status, COUNT(*) AS cnt, SUM(overtime_hours) AS ot
           FROM attendance WHERE employee_id=? AND att_date LIKE ?
           GROUP BY status""",
        (employee_id, f"{month_str}-%"),
    ).fetchall()
    summary = {
        "present": 0, "absent": 0, "half_day": 0, "leave": 0, "holiday": 0, "overtime_hours": 0.0,
    }
    for r in rows:
        summary[r["status"]] = r["cnt"]
        summary["overtime_hours"] += r["ot"] or 0.0
    summary["overtime_hours"] = round(summary["overtime_hours"], 2)
    return summary


@router.delete("/{attendance_id}", dependencies=[Depends(require_permission("hr", "edit"))])
def delete_attendance(attendance_id: str, conn=Depends(get_master_db)) -> MessageResponse:
    row = conn.execute("SELECT id FROM attendance WHERE id=?", (attendance_id,)).fetchone()
    if not row:
        raise HTTPException(404, "Attendance record not found.")
    conn.execute("DELETE FROM attendance WHERE id=?", (attendance_id,))
    return MessageResponse(detail="Attendance record deleted.")
