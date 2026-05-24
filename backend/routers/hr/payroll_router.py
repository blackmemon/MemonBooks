"""payroll_router.py — Payroll Runs: create, approve, post, view payslips."""

import uuid
from datetime import datetime, timezone
from typing import Optional

from fastapi import APIRouter, Depends, HTTPException, Query

from backend.security.auth import get_current_user
from backend.security.permissions import require_permission
from backend.database.connection import get_master_db
from backend.models.hr.schemas import PayrollRunCreate, IDResponse, MessageResponse, HRListResponse
from backend.services.numbering import next_number
from backend.services.payroll_calculator import compute_employee_payroll, get_employee_attendance_summary
from backend.services.auto_jv import create_payroll_jv

router = APIRouter()

# System-config account IDs — fetched from settings or COA by name at runtime
_SALARY_EXP_ACCOUNT_CODE  = "6100"   # Salary Expense
_TAX_PAYABLE_ACCOUNT_CODE  = "2110"   # Income Tax Payable
_DED_PAYABLE_ACCOUNT_CODE  = "2120"   # Deductions Payable
_BANK_ACCOUNT_CODE         = "1110"   # Default Bank Account


def _now() -> str:
    return datetime.now(timezone.utc).isoformat()


def _get_account_by_code(conn, code: str) -> Optional[str]:
    """Return account ID by COA code."""
    row = conn.execute("SELECT id FROM chart_of_accounts WHERE code=?", (code,)).fetchone()
    return row["id"] if row else None


@router.get("/", dependencies=[Depends(require_permission("hr", "view"))])
def list_payroll_runs(
    year:   Optional[int] = Query(None),
    month:  Optional[int] = Query(None),
    status: Optional[str] = Query(None),
    page:   int = Query(1, ge=1),
    limit:  int = Query(50, ge=1, le=200),
    conn=Depends(get_master_db),
) -> HRListResponse:
    sql = "SELECT id, run_number, month, year, run_date, status, total_gross, total_net, notes, created_at FROM payroll_runs WHERE 1=1"
    params: list = []
    if year:
        sql += " AND year=?"; params.append(year)
    if month:
        sql += " AND month=?"; params.append(month)
    if status:
        sql += " AND status=?"; params.append(status)
    total = conn.execute(f"SELECT COUNT(*) FROM ({sql})", params).fetchone()[0]
    sql += " ORDER BY year DESC, month DESC LIMIT ? OFFSET ?"
    params += [limit, (page - 1) * limit]
    return HRListResponse(items=[dict(r) for r in conn.execute(sql, params).fetchall()], total=total, page=page, limit=limit)


@router.get("/{run_id}", dependencies=[Depends(require_permission("hr", "view"))])
def get_payroll_run(run_id: str, conn=Depends(get_master_db)):
    run = conn.execute("SELECT * FROM payroll_runs WHERE id=?", (run_id,)).fetchone()
    if not run:
        raise HTTPException(404, "Payroll run not found.")
    lines = conn.execute(
        """SELECT pl.*, e.emp_code, e.full_name, e.department_id,
                  d.name AS department_name, e.bank_account, e.bank_name
           FROM payroll_lines pl
           JOIN employees e ON e.id = pl.employee_id
           LEFT JOIN departments d ON d.id = e.department_id
           WHERE pl.run_id = ?
           ORDER BY e.emp_code""",
        (run_id,),
    ).fetchall()
    result = dict(run)
    result["lines"] = [dict(l) for l in lines]
    return result


@router.post("/", dependencies=[Depends(require_permission("hr", "create"))])
def create_payroll_run(
    body: PayrollRunCreate,
    current_user: dict = Depends(get_current_user),
    conn=Depends(get_master_db),
) -> IDResponse:
    """
    Generate payroll for all active employees (or a specific department).
    - Reads attendance for the given month/year
    - Computes gross, deductions, income tax, net for each employee
    - Saves payroll_runs header + payroll_lines
    """
    # Prevent duplicate run
    existing = conn.execute(
        "SELECT id FROM payroll_runs WHERE month=? AND year=? AND status != 'cancelled'",
        (body.month, body.year),
    ).fetchone()
    if existing:
        raise HTTPException(400, f"A payroll run already exists for {body.year}-{body.month:02d}.")

    # Fetch employees
    emp_sql = "SELECT * FROM employees WHERE is_active=1"
    emp_params: list = []
    if body.department_id:
        emp_sql += " AND department_id=?"; emp_params.append(body.department_id)
    employees = [dict(r) for r in conn.execute(emp_sql, emp_params).fetchall()]
    if not employees:
        raise HTTPException(400, "No active employees found.")

    run_id     = str(uuid.uuid4())
    run_number = next_number("PAY", conn)
    total_gross = total_net = total_deductions = total_income_tax = 0.0

    conn.execute(
        """INSERT INTO payroll_runs (id, run_number, month, year, run_date, status,
               total_gross, total_deductions, total_income_tax, total_net, notes, created_by, created_at, updated_at)
           VALUES (?,?,?,?,?,'draft',0,0,0,0,?,?,?,?)""",
        (run_id, run_number, body.month, body.year, body.run_date,
         body.notes, current_user.get("sub"), _now(), _now()),
    )

    for emp in employees:
        # Attendance summary
        att = get_employee_attendance_summary(conn, emp["id"], body.year, body.month)
        # Salary components
        comps = conn.execute(
            """SELECT es.component_id, es.amount, es.percentage, sc.name, sc.component_type
               FROM employee_salary es
               JOIN salary_components sc ON sc.id = es.component_id
               WHERE es.employee_id=? AND (es.effective_to IS NULL OR es.effective_to >= DATE('now'))""",
            (emp["id"],),
        ).fetchall()

        result = compute_employee_payroll(
            employee=emp,
            components=[dict(c) for c in comps],
            year=body.year,
            month=body.month,
            present_days=int(att["present_days"]),
            absent_days=att["absent_days"],
            leave_days=att["leave_days"],
            overtime_hours=att["overtime_hours"],
        )

        conn.execute(
            """INSERT INTO payroll_lines (id, run_id, employee_id, working_days, present_days,
                   absent_days, leave_days, overtime_hours, gross_salary, total_deductions,
                   income_tax, net_salary, created_at)
               VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?)""",
            (str(uuid.uuid4()), run_id, emp["id"],
             result["working_days"], result["present_days"],
             result["absent_days"], result["leave_days"], result["overtime_hours"],
             result["gross_salary"], result["total_deductions"],
             result["income_tax"], result["net_salary"], _now()),
        )
        total_gross         += result["gross_salary"]
        total_net           += result["net_salary"]
        total_deductions    += result["total_deductions"]
        total_income_tax    += result["income_tax"]

    # Update run totals
    conn.execute(
        """UPDATE payroll_runs SET total_gross=?, total_deductions=?, total_income_tax=?, total_net=?, updated_at=?
           WHERE id=?""",
        (round(total_gross, 2), round(total_deductions, 2), round(total_income_tax, 2), round(total_net, 2), _now(), run_id),
    )
    return IDResponse(id=run_id, number=run_number)


@router.post("/{run_id}/approve", dependencies=[Depends(require_permission("hr", "post"))])
def approve_payroll(run_id: str, current_user: dict = Depends(get_current_user), conn=Depends(get_master_db)) -> MessageResponse:
    run = conn.execute("SELECT status FROM payroll_runs WHERE id=?", (run_id,)).fetchone()
    if not run or run["status"] != "draft":
        raise HTTPException(400, "Only draft payroll runs can be approved.")
    conn.execute(
        "UPDATE payroll_runs SET status='approved', updated_at=? WHERE id=?",
        (_now(), run_id),
    )
    return MessageResponse(detail="Payroll run approved.")


@router.post("/{run_id}/post", dependencies=[Depends(require_permission("hr", "post"))])
def post_payroll(run_id: str, current_user: dict = Depends(get_current_user), conn=Depends(get_master_db)) -> MessageResponse:
    run = conn.execute("SELECT * FROM payroll_runs WHERE id=?", (run_id,)).fetchone()
    if not run or run["status"] not in ("draft", "approved"):
        raise HTTPException(400, "Payroll must be draft or approved to post.")

    # Get JV accounts by code
    sal_acc = _get_account_by_code(conn, _SALARY_EXP_ACCOUNT_CODE)
    tax_acc = _get_account_by_code(conn, _TAX_PAYABLE_ACCOUNT_CODE)
    ded_acc = _get_account_by_code(conn, _DED_PAYABLE_ACCOUNT_CODE)
    bnk_acc = _get_account_by_code(conn, _BANK_ACCOUNT_CODE)

    if not all([sal_acc, tax_acc, ded_acc, bnk_acc]):
        raise HTTPException(500, "COA accounts for payroll JV not found. Ensure accounts with codes 6100, 2110, 2120, 1110 exist.")

    jv_id = create_payroll_jv(
        conn,
        run_id=run_id,
        run_number=run["run_number"],
        run_date=run["run_date"],
        month=run["month"],
        year=run["year"],
        total_gross=run["total_gross"],
        total_income_tax=run["total_income_tax"],
        total_deductions=run["total_deductions"],
        total_net=run["total_net"],
        salary_expense_account_id=sal_acc,
        tax_payable_account_id=tax_acc,
        deduction_payable_account_id=ded_acc,
        bank_account_id=bnk_acc,
        created_by=current_user.get("sub", "system"),
    )
    conn.execute(
        "UPDATE payroll_runs SET status='posted', jv_id=?, posted_by=?, posted_at=?, updated_at=? WHERE id=?",
        (jv_id, current_user.get("sub"), _now(), _now(), run_id),
    )
    return MessageResponse(detail="Payroll posted. Journal voucher created.")


@router.post("/{run_id}/cancel", dependencies=[Depends(require_permission("hr", "edit"))])
def cancel_payroll(run_id: str, conn=Depends(get_master_db)) -> MessageResponse:
    run = conn.execute("SELECT status FROM payroll_runs WHERE id=?", (run_id,)).fetchone()
    if not run or run["status"] == "posted":
        raise HTTPException(400, "Cannot cancel a posted payroll run.")
    conn.execute("UPDATE payroll_runs SET status='cancelled', updated_at=? WHERE id=?", (_now(), run_id))
    return MessageResponse(detail="Payroll run cancelled.")


@router.get("/{run_id}/payslip/{employee_id}", dependencies=[Depends(require_permission("hr", "view"))])
def get_payslip(run_id: str, employee_id: str, conn=Depends(get_master_db)) -> dict:
    """Return payslip data for one employee in a run."""
    run = conn.execute("SELECT * FROM payroll_runs WHERE id=?", (run_id,)).fetchone()
    if not run:
        raise HTTPException(404, "Payroll run not found.")
    line = conn.execute(
        """SELECT pl.*, e.emp_code, e.full_name, e.designation_id,
                  e.bank_name, e.bank_account, e.bank_iban,
                  d.name AS department_name, dsg.name AS designation_name
           FROM payroll_lines pl
           JOIN employees e ON e.id = pl.employee_id
           LEFT JOIN departments d    ON d.id  = e.department_id
           LEFT JOIN designations dsg ON dsg.id = e.designation_id
           WHERE pl.run_id=? AND pl.employee_id=?""",
        (run_id, employee_id),
    ).fetchone()
    if not line:
        raise HTTPException(404, "Employee not found in this payroll run.")
    return {
        "run":  dict(run),
        "line": dict(line),
    }
