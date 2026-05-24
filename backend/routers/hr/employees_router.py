"""employees_router.py — Employee CRUD + salary components."""

import uuid
from datetime import datetime, timezone
from typing import Optional

from fastapi import APIRouter, Depends, HTTPException, Query

from backend.security.auth import get_current_user
from backend.security.permissions import require_permission
from backend.database.connection import get_master_db
from backend.models.hr.schemas import (
    EmployeeCreate, EmployeeUpdate,
    IDResponse, MessageResponse, HRListResponse,
)
from backend.services.numbering import next_number

router = APIRouter()


def _now() -> str:
    return datetime.now(timezone.utc).isoformat()


@router.get("/", dependencies=[Depends(require_permission("hr", "view"))])
def list_employees(
    department_id: Optional[str] = Query(None),
    active_only:   bool = Query(True),
    search:        Optional[str] = Query(None),
    page:  int = Query(1, ge=1),
    limit: int = Query(50, ge=1, le=500),
    conn=Depends(get_master_db),
) -> HRListResponse:
    sql = """SELECT e.id, e.emp_code, e.full_name, e.phone, e.email,
                    e.employment_type, e.join_date, e.basic_salary, e.is_active,
                    d.name AS department_name, dsg.name AS designation_name
             FROM employees e
             LEFT JOIN departments d   ON d.id = e.department_id
             LEFT JOIN designations dsg ON dsg.id = e.designation_id
             WHERE 1=1"""
    params: list = []
    if active_only:
        sql += " AND e.is_active = 1"
    if department_id:
        sql += " AND e.department_id = ?"; params.append(department_id)
    if search:
        sql += " AND (e.full_name LIKE ? OR e.emp_code LIKE ? OR e.phone LIKE ?)"; term = f"%{search}%"; params += [term, term, term]
    total = conn.execute(f"SELECT COUNT(*) FROM ({sql})", params).fetchone()[0]
    sql += " ORDER BY e.emp_code LIMIT ? OFFSET ?"
    params += [limit, (page - 1) * limit]
    return HRListResponse(items=[dict(r) for r in conn.execute(sql, params).fetchall()], total=total, page=page, limit=limit)


@router.get("/dropdown", dependencies=[Depends(require_permission("hr", "view"))])
def employees_dropdown(conn=Depends(get_master_db)):
    """Lightweight list for dropdowns."""
    rows = conn.execute(
        "SELECT id, emp_code, full_name, department_id FROM employees WHERE is_active=1 ORDER BY full_name"
    ).fetchall()
    return [dict(r) for r in rows]


@router.get("/{employee_id}", dependencies=[Depends(require_permission("hr", "view"))])
def get_employee(employee_id: str, conn=Depends(get_master_db)):
    row = conn.execute(
        """SELECT e.*, d.name AS department_name, dsg.name AS designation_name
           FROM employees e
           LEFT JOIN departments d    ON d.id  = e.department_id
           LEFT JOIN designations dsg ON dsg.id = e.designation_id
           WHERE e.id = ?""",
        (employee_id,),
    ).fetchone()
    if not row:
        raise HTTPException(404, "Employee not found.")
    emp = dict(row)
    # Salary components
    comps = conn.execute(
        """SELECT es.*, sc.name, sc.code, sc.component_type
           FROM employee_salary es
           JOIN salary_components sc ON sc.id = es.component_id
           WHERE es.employee_id = ? AND (es.effective_to IS NULL OR es.effective_to >= DATE('now'))
           ORDER BY sc.component_type, sc.name""",
        (employee_id,),
    ).fetchall()
    emp['salary_components'] = [dict(c) for c in comps]
    return emp


@router.post("/", dependencies=[Depends(require_permission("hr", "create"))])
def create_employee(
    body: EmployeeCreate,
    current_user: dict = Depends(get_current_user),
    conn=Depends(get_master_db),
) -> IDResponse:
    emp_id   = str(uuid.uuid4())
    emp_code = next_number("EMP", conn)
    conn.execute(
        """INSERT INTO employees (
               id, emp_code, full_name, father_name, dob, gender, cnic, phone, email,
               address, department_id, designation_id, employment_type, join_date,
               basic_salary, bank_name, bank_account, bank_iban,
               is_active, created_by, created_at, updated_at
           ) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,1,?,?,?)""",
        (emp_id, emp_code, body.full_name, body.father_name, body.dob,
         body.gender, body.cnic, body.phone, body.email,
         body.address, body.department_id, body.designation_id,
         body.employment_type, body.join_date, body.basic_salary,
         body.bank_name, body.bank_account, body.bank_iban,
         current_user.get("sub"), _now(), _now()),
    )
    # Salary components
    today = datetime.now().strftime("%Y-%m-%d")
    for comp in body.salary_components:
        conn.execute(
            """INSERT INTO employee_salary (id, employee_id, component_id, amount, percentage, effective_from, created_by)
               VALUES (?,?,?,?,?,?,?)""",
            (str(uuid.uuid4()), emp_id, comp.component_id, comp.amount, comp.percentage, today, current_user.get("sub")),
        )
    return IDResponse(id=emp_id, number=emp_code)


@router.put("/{employee_id}", dependencies=[Depends(require_permission("hr", "edit"))])
def update_employee(
    employee_id: str,
    body: EmployeeUpdate,
    current_user: dict = Depends(get_current_user),
    conn=Depends(get_master_db),
) -> MessageResponse:
    emp = conn.execute("SELECT id FROM employees WHERE id=?", (employee_id,)).fetchone()
    if not emp:
        raise HTTPException(404, "Employee not found.")

    updates: dict = body.model_dump(exclude_none=True, exclude={"salary_components"})
    if not updates and body.salary_components is None:
        raise HTTPException(400, "No fields to update.")

    if updates:
        sets = ", ".join(f"{k}=?" for k in updates)
        vals = list(updates.values()) + [_now(), employee_id]
        conn.execute(f"UPDATE employees SET {sets}, updated_at=? WHERE id=?", vals)

    # Replace salary components if provided
    if body.salary_components is not None:
        today = datetime.now().strftime("%Y-%m-%d")
        # Close existing
        conn.execute(
            "UPDATE employee_salary SET effective_to=? WHERE employee_id=? AND effective_to IS NULL",
            (today, employee_id),
        )
        # Insert new
        for comp in body.salary_components:
            conn.execute(
                """INSERT INTO employee_salary (id, employee_id, component_id, amount, percentage, effective_from, created_by)
                   VALUES (?,?,?,?,?,?,?)""",
                (str(uuid.uuid4()), employee_id, comp.component_id, comp.amount, comp.percentage, today, current_user.get("sub")),
            )
    return MessageResponse(detail="Employee updated.")


@router.post("/{employee_id}/deactivate", dependencies=[Depends(require_permission("hr", "edit"))])
def deactivate_employee(employee_id: str, conn=Depends(get_master_db)) -> MessageResponse:
    emp = conn.execute("SELECT id FROM employees WHERE id=?", (employee_id,)).fetchone()
    if not emp:
        raise HTTPException(404, "Employee not found.")
    conn.execute("UPDATE employees SET is_active=0, updated_at=? WHERE id=?", (_now(), employee_id))
    return MessageResponse(detail="Employee deactivated.")
