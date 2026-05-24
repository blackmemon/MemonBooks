"""
payroll_calculator.py — Payroll computation service.

Responsibilities:
- Calculate gross salary, deductions, income tax, net salary per employee
- Handle attendance-based proration (absent days reduce basic salary)
- Apply salary components (earnings + deductions + percentage-based)
- Pakistan income tax slabs (FY 2024-25)
"""

from __future__ import annotations
import calendar
from typing import Optional


# ─────────────────────────────────────────────────────────────────────────────
# Pakistan Income Tax Slabs — Annual (FY 2024-25)
# ─────────────────────────────────────────────────────────────────────────────

TAX_SLABS = [
    # (annual_income_up_to, base_tax, rate_on_excess, excess_above)
    (600_000,     0,         0.00, 0),
    (1_200_000,   0,         0.05, 600_000),
    (2_400_000,   30_000,    0.15, 1_200_000),
    (3_600_000,   210_000,   0.25, 2_400_000),
    (6_000_000,   510_000,   0.30, 3_600_000),
    (float('inf'), 1_230_000, 0.35, 6_000_000),
]


def _annual_income_tax(annual_income: float) -> float:
    """Compute Pakistan annual income tax from slabs."""
    for ceiling, base_tax, rate, excess_above in TAX_SLABS:
        if annual_income <= ceiling:
            return base_tax + (annual_income - excess_above) * rate
    return 0.0


def _monthly_income_tax(monthly_gross: float) -> float:
    """Convert monthly gross to annual, compute tax, return monthly portion."""
    annual = monthly_gross * 12
    annual_tax = _annual_income_tax(annual)
    return round(annual_tax / 12, 2)


# ─────────────────────────────────────────────────────────────────────────────
# Main calculator
# ─────────────────────────────────────────────────────────────────────────────

def compute_employee_payroll(
    *,
    employee: dict,
    components: list[dict],
    year: int,
    month: int,
    present_days: int,
    absent_days: int,
    leave_days: int,
    overtime_hours: float,
    overtime_rate_per_hour: float = 0.0,
) -> dict:
    """
    Compute payroll for one employee for a given month.

    Args:
        employee:            DB row dict (basic_salary, etc.)
        components:          list of {component_type, amount, percentage, code, name}
        year / month:        payroll period
        present_days:        days marked present
        absent_days:         days marked absent (unpaid)
        leave_days:          approved leave days (paid)
        overtime_hours:      total OT hours
        overtime_rate_per_hour: if 0, computed as (basic_salary / working_days / 8) * 1.5

    Returns dict with all computed fields.
    """
    working_days = calendar.monthrange(year, month)[1]  # total calendar days
    # Paid days = present + leave; absent days are unpaid
    paid_days = present_days + leave_days
    paid_days = min(paid_days, working_days)

    basic_salary: float = employee.get('basic_salary', 0.0) or 0.0

    # Prorate basic salary by attendance
    if working_days > 0:
        prorated_basic = basic_salary * paid_days / working_days
    else:
        prorated_basic = basic_salary

    # Overtime
    if overtime_rate_per_hour <= 0 and basic_salary > 0 and working_days > 0:
        daily_rate = basic_salary / working_days
        hourly_rate = daily_rate / 8
        overtime_rate_per_hour = hourly_rate * 1.5
    overtime_pay = round(overtime_hours * overtime_rate_per_hour, 2)

    # Salary components
    total_earnings   = 0.0
    total_deductions = 0.0
    component_details: list[dict] = []

    for comp in components:
        ctype = comp.get('component_type', 'earning')
        amount: float = comp.get('amount', 0.0) or 0.0
        pct: float    = comp.get('percentage', 0.0) or 0.0

        # Percentage-based components use basic as base
        if pct > 0:
            computed = round(basic_salary * pct / 100, 2)
        else:
            computed = round(amount, 2)

        # Prorate earnings (not deductions like EOBI/PESSI which are fixed)
        if ctype == 'earning' and paid_days < working_days and working_days > 0:
            computed = round(computed * paid_days / working_days, 2)

        if ctype == 'earning':
            total_earnings += computed
        elif ctype == 'deduction':
            total_deductions += computed

        component_details.append({
            'component_id':   comp.get('component_id') or comp.get('id'),
            'component_name': comp.get('name', ''),
            'component_type': ctype,
            'amount':         computed,
        })

    gross_salary = round(prorated_basic + overtime_pay + total_earnings, 2)
    income_tax   = _monthly_income_tax(gross_salary)
    total_deductions_final = round(total_deductions + income_tax, 2)
    net_salary   = round(gross_salary - total_deductions_final, 2)

    return {
        'employee_id':       employee['id'],
        'working_days':      working_days,
        'present_days':      present_days,
        'absent_days':       absent_days,
        'leave_days':        leave_days,
        'overtime_hours':    overtime_hours,
        'prorated_basic':    round(prorated_basic, 2),
        'overtime_pay':      overtime_pay,
        'total_earnings':    round(total_earnings, 2),
        'gross_salary':      gross_salary,
        'total_deductions':  round(total_deductions, 2),
        'income_tax':        income_tax,
        'net_salary':        net_salary,
        'components':        component_details,
    }


def get_employee_attendance_summary(
    conn,
    employee_id: str,
    year: int,
    month: int,
) -> dict:
    """Fetch attendance counts for an employee from the DB."""
    month_str = f"{year:04d}-{month:02d}"
    rows = conn.execute(
        """SELECT status, overtime_hours, COUNT(*) AS cnt
           FROM attendance
           WHERE employee_id = ? AND att_date LIKE ?
           GROUP BY status, overtime_hours""",
        (employee_id, f"{month_str}-%"),
    ).fetchall()

    summary = {
        'present_days':    0,
        'absent_days':     0,
        'half_days':       0,
        'leave_days':      0,
        'overtime_hours':  0.0,
    }
    for row in rows:
        s   = row['status']
        cnt = row['cnt']
        ot  = row['overtime_hours'] or 0.0
        if s == 'present':
            summary['present_days'] += cnt
            summary['overtime_hours'] += ot * cnt
        elif s == 'absent':
            summary['absent_days'] += cnt
        elif s == 'half_day':
            summary['half_days'] += cnt
            summary['present_days'] += cnt * 0.5  # half day counts as 0.5
        elif s == 'leave':
            summary['leave_days'] += cnt

    # Round overtime hours
    summary['overtime_hours'] = round(summary['overtime_hours'], 2)
    return summary
