"""
schemas.py — Pydantic schemas for HR & Payroll module.
Employees, Attendance, Leave, Payroll Runs.
"""

from __future__ import annotations
from typing import Optional, List
from pydantic import BaseModel, Field


# ─────────────────────────────────────────────────────────────────────────────
# Common
# ─────────────────────────────────────────────────────────────────────────────

class IDResponse(BaseModel):
    id:     str
    number: Optional[str] = None
    detail: str = "Created successfully."


class MessageResponse(BaseModel):
    detail: str


class HRListResponse(BaseModel):
    items: list
    total: int
    page:  int = 1
    limit: int = 50


# ─────────────────────────────────────────────────────────────────────────────
# EMPLOYEE
# ─────────────────────────────────────────────────────────────────────────────

class SalaryComponentEntry(BaseModel):
    component_id: str
    amount:       float = Field(0.0, ge=0)
    percentage:   float = Field(0.0, ge=0, le=100)


class EmployeeCreate(BaseModel):
    full_name:         str  = Field(..., min_length=2, max_length=150)
    father_name:       Optional[str] = None
    dob:               Optional[str] = None                     # YYYY-MM-DD
    gender:            Optional[str] = Field(None, pattern=r'^(male|female|other)$')
    cnic:              Optional[str] = None
    phone:             Optional[str] = None
    email:             Optional[str] = None
    address:           Optional[str] = None
    department_id:     Optional[str] = None
    designation_id:    Optional[str] = None
    employment_type:   str = Field('permanent', pattern=r'^(permanent|contract|probation|part_time)$')
    join_date:         str = Field(..., pattern=r'^\d{4}-\d{2}-\d{2}$')
    basic_salary:      float = Field(0.0, ge=0)
    bank_name:         Optional[str] = None
    bank_account:      Optional[str] = None
    bank_iban:         Optional[str] = None
    salary_components: List[SalaryComponentEntry] = []


class EmployeeUpdate(BaseModel):
    full_name:         Optional[str] = None
    father_name:       Optional[str] = None
    dob:               Optional[str] = None
    gender:            Optional[str] = None
    cnic:              Optional[str] = None
    phone:             Optional[str] = None
    email:             Optional[str] = None
    address:           Optional[str] = None
    department_id:     Optional[str] = None
    designation_id:    Optional[str] = None
    employment_type:   Optional[str] = None
    basic_salary:      Optional[float] = None
    bank_name:         Optional[str] = None
    bank_account:      Optional[str] = None
    bank_iban:         Optional[str] = None
    is_active:         Optional[bool] = None
    salary_components: Optional[List[SalaryComponentEntry]] = None


# ─────────────────────────────────────────────────────────────────────────────
# ATTENDANCE
# ─────────────────────────────────────────────────────────────────────────────

class AttendanceMarkRequest(BaseModel):
    employee_id:    str
    att_date:       str  = Field(..., pattern=r'^\d{4}-\d{2}-\d{2}$')
    status:         str  = Field(..., pattern=r'^(present|absent|half_day|leave|holiday)$')
    check_in:       Optional[str] = None   # HH:MM
    check_out:      Optional[str] = None   # HH:MM
    overtime_hours: float = Field(0.0, ge=0)
    notes:          Optional[str] = None


class AttendanceBulkRequest(BaseModel):
    """Mark attendance for multiple employees at once (same date/status)."""
    employee_ids:   List[str] = Field(..., min_length=1)
    att_date:       str = Field(..., pattern=r'^\d{4}-\d{2}-\d{2}$')
    status:         str = Field(..., pattern=r'^(present|absent|half_day|leave|holiday)$')
    check_in:       Optional[str] = None
    check_out:      Optional[str] = None


# ─────────────────────────────────────────────────────────────────────────────
# PAYROLL
# ─────────────────────────────────────────────────────────────────────────────

class PayrollRunCreate(BaseModel):
    month:       int = Field(..., ge=1, le=12)
    year:        int = Field(..., ge=2000, le=2100)
    run_date:    str = Field(..., pattern=r'^\d{4}-\d{2}-\d{2}$')
    department_id: Optional[str] = None   # None = all departments
    notes:       Optional[str] = None
