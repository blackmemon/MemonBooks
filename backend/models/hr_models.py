"""HR & Payroll models — spec sections 4, 5.

EmployeeCustomField (listed in spec section 14) is not duplicated here: the
generic CustomFieldValue table (backend/models/system_models.py) already
covers "dynamic name/value pairs" for Employee the same way it does for
Vendor/Customer/Item, so a second employee-only custom-field table would
just be the same schema twice.

Payroll Run is independent from Employee Maintenance per spec 5 — PayrollLine
carries per-period figures only; Employee Name/Department/etc. are derived
via join to Employee, never copied onto the line (spec 5.6).
"""
from __future__ import annotations

import datetime as dt
from decimal import Decimal
from typing import Optional

from sqlalchemy import ForeignKey, Numeric, String, Text
from sqlalchemy.orm import Mapped, mapped_column, relationship

from database.database import UserBase
from backend.models.mixins import MasterStatusMixin, DocumentStatusMixin, TimestampMixin

GENDERS = ("Male", "Female")
MARITAL_STATUSES = ("Single", "Married")
EMPLOYEE_TYPES = ("Full-time", "Part-time", "Contract")
PAY_SCHEDULES = ("Monthly", "Weekly")
TRANSPORT_MODES = ("Public", "Own", "Company")
EXPERIENCE_TYPES = ("Previous", "Fresh")
QUALIFICATION_STATUSES = ("Completed", "In Process")
PAY_RATE_TYPES = ("Salary", "Hourly")
YES_NO = ("Yes", "No")
BANK_OR_CASH = ("Bank", "Cash")


class Employee(MasterStatusMixin, UserBase):
    __tablename__ = "employees"

    id: Mapped[int] = mapped_column(primary_key=True)
    employee_code: Mapped[str] = mapped_column(String(40), unique=True, index=True)

    # 4.2 Personal Information
    first_name: Mapped[str] = mapped_column(String(100))
    last_name: Mapped[str] = mapped_column(String(100))
    national_id: Mapped[Optional[str]] = mapped_column(String(40), nullable=True)
    date_of_birth: Mapped[Optional[dt.date]] = mapped_column(nullable=True)
    gender: Mapped[Optional[str]] = mapped_column(String(10), nullable=True)
    marital_status: Mapped[Optional[str]] = mapped_column(String(10), nullable=True)

    # 4.3 Contact Information
    mobile_no: Mapped[Optional[str]] = mapped_column(String(40), nullable=True)
    email: Mapped[Optional[str]] = mapped_column(String(150), nullable=True)
    residential_address: Mapped[Optional[str]] = mapped_column(Text, nullable=True)
    emergency_contact_name: Mapped[Optional[str]] = mapped_column(String(150), nullable=True)
    emergency_contact_phone: Mapped[Optional[str]] = mapped_column(String(40), nullable=True)

    # 4.4 Job / Employment
    department: Mapped[Optional[str]] = mapped_column(String(100), nullable=True)
    designation: Mapped[Optional[str]] = mapped_column(String(100), nullable=True)
    employee_type: Mapped[str] = mapped_column(String(20), default="Full-time")
    hire_date: Mapped[Optional[dt.date]] = mapped_column(nullable=True)
    leave_date: Mapped[Optional[dt.date]] = mapped_column(nullable=True)
    pay_schedule: Mapped[str] = mapped_column(String(10), default="Monthly")
    work_location: Mapped[Optional[str]] = mapped_column(String(150), nullable=True)
    transport_mode: Mapped[Optional[str]] = mapped_column(String(20), nullable=True)

    # 4.7 Salary & Allowances (master/default values)
    pay_rate_type: Mapped[str] = mapped_column(String(10), default="Salary")
    basic_salary_or_hourly_rate: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)
    overtime_eligible: Mapped[str] = mapped_column(String(3), default="No")
    kpi_allowance: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)
    house_allowance: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)
    mobile_allowance: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)
    medical_allowance: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)
    transport_allowance: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)
    fuel_allowance: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)
    bonus_eligible: Mapped[str] = mapped_column(String(3), default="No")
    commission_eligible: Mapped[str] = mapped_column(String(3), default="No")

    # 4.8 Benefits, Deductions and Gratuity (master/default values)
    employee_pf_pension: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)
    employer_pf_pension: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)
    gratuity_eligible: Mapped[str] = mapped_column(String(3), default="No")
    gratuity_rule: Mapped[Optional[str]] = mapped_column(String(200), nullable=True)

    # 4.9 Bank & Payment
    payment_method: Mapped[str] = mapped_column(String(10), default="Bank")
    bank_name: Mapped[Optional[str]] = mapped_column(String(150), nullable=True)
    account_iban: Mapped[Optional[str]] = mapped_column(String(60), nullable=True)

    description: Mapped[Optional[str]] = mapped_column(Text, nullable=True)

    qualifications: Mapped[list["EmployeeQualification"]] = relationship(cascade="all, delete-orphan")
    experiences: Mapped[list["EmployeeExperience"]] = relationship(cascade="all, delete-orphan")


class EmployeeQualification(UserBase):
    __tablename__ = "employee_qualifications"

    id: Mapped[int] = mapped_column(primary_key=True)
    employee_id: Mapped[int] = mapped_column(ForeignKey("employees.id"))
    qualification_name: Mapped[str] = mapped_column(String(150))
    qualification_degree: Mapped[Optional[str]] = mapped_column(String(100), nullable=True)
    qualification_status: Mapped[str] = mapped_column(String(20), default="Completed")
    qualification_complete_date: Mapped[Optional[dt.date]] = mapped_column(nullable=True)


class EmployeeExperience(UserBase):
    __tablename__ = "employee_experiences"

    id: Mapped[int] = mapped_column(primary_key=True)
    employee_id: Mapped[int] = mapped_column(ForeignKey("employees.id"))
    experience_type: Mapped[str] = mapped_column(String(10), default="Fresh")
    previous_company: Mapped[Optional[str]] = mapped_column(String(150), nullable=True)
    previous_designation: Mapped[Optional[str]] = mapped_column(String(100), nullable=True)
    previous_salary: Mapped[Optional[Decimal]] = mapped_column(Numeric(18, 2), nullable=True)
    previous_leave_date: Mapped[Optional[dt.date]] = mapped_column(nullable=True)


class Attendance(TimestampMixin, UserBase):
    __tablename__ = "attendance"

    id: Mapped[int] = mapped_column(primary_key=True)
    employee_id: Mapped[int] = mapped_column(ForeignKey("employees.id"))
    attendance_date: Mapped[dt.date] = mapped_column()
    status: Mapped[str] = mapped_column(String(20), default="Present")
    check_in_time: Mapped[Optional[dt.time]] = mapped_column(nullable=True)
    check_out_time: Mapped[Optional[dt.time]] = mapped_column(nullable=True)
    notes: Mapped[Optional[str]] = mapped_column(Text, nullable=True)


class PayrollRun(DocumentStatusMixin, UserBase):
    __tablename__ = "payroll_runs"

    id: Mapped[int] = mapped_column(primary_key=True)
    run_no: Mapped[str] = mapped_column(String(40), unique=True, index=True)
    payroll_period: Mapped[str] = mapped_column(String(20))  # e.g. "2026-08"
    pay_schedule: Mapped[str] = mapped_column(String(10), default="Monthly")
    run_date: Mapped[dt.date] = mapped_column()
    notes: Mapped[Optional[str]] = mapped_column(Text, nullable=True)

    lines: Mapped[list["PayrollLine"]] = relationship(cascade="all, delete-orphan")


class PayrollLine(UserBase):
    __tablename__ = "payroll_lines"

    id: Mapped[int] = mapped_column(primary_key=True)
    payroll_run_id: Mapped[int] = mapped_column(ForeignKey("payroll_runs.id"))
    employee_id: Mapped[int] = mapped_column(ForeignKey("employees.id"))

    no_of_days_in_month: Mapped[Optional[int]] = mapped_column(nullable=True)
    days_worked: Mapped[Optional[Decimal]] = mapped_column(Numeric(6, 2), nullable=True)
    leave_days: Mapped[Optional[Decimal]] = mapped_column(Numeric(6, 2), nullable=True)
    overtime_hours: Mapped[Optional[Decimal]] = mapped_column(Numeric(6, 2), nullable=True)
    absence_days: Mapped[Optional[Decimal]] = mapped_column(Numeric(6, 2), nullable=True)

    basic_salary: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)
    overtime_pay: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)
    house_allowance: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)
    transport_allowance: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)
    mobile_allowance: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)
    medical_allowance: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)
    fuel_allowance: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)
    kpi_allowance: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)
    bonus: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)
    commission_incentive: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)
    adhoc_allowance: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)
    arrears: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)
    gross_salary: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)

    employee_pf_pension: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)
    employer_pf_pension: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)
    tax_deduction: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)
    loan_advance_deduction: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)
    other_deductions: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)
    total_deductions: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)

    last_increment_amount: Mapped[Optional[Decimal]] = mapped_column(Numeric(18, 2), nullable=True)
    last_increment_date: Mapped[Optional[dt.date]] = mapped_column(nullable=True)
    current_salary_after_increment: Mapped[Optional[Decimal]] = mapped_column(Numeric(18, 2), nullable=True)
    gratuity_accrued: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)
    gratuity_paid: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)

    net_pay: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)
    total_payable: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)

    payment_method: Mapped[Optional[str]] = mapped_column(String(10), nullable=True)
    bank_name: Mapped[Optional[str]] = mapped_column(String(150), nullable=True)
    account_iban: Mapped[Optional[str]] = mapped_column(String(60), nullable=True)
    approved_by: Mapped[Optional[str]] = mapped_column(String(100), nullable=True)

    allowances: Mapped[list["PayrollAllowance"]] = relationship(cascade="all, delete-orphan")
    deductions: Mapped[list["PayrollDeduction"]] = relationship(cascade="all, delete-orphan")


class PayrollAllowance(UserBase):
    """Ad-hoc/extra allowance line items beyond PayrollLine's fixed columns."""

    __tablename__ = "payroll_allowances"

    id: Mapped[int] = mapped_column(primary_key=True)
    payroll_line_id: Mapped[int] = mapped_column(ForeignKey("payroll_lines.id"))
    allowance_type: Mapped[str] = mapped_column(String(100))
    amount: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)
    note: Mapped[Optional[str]] = mapped_column(String(255), nullable=True)


class PayrollDeduction(UserBase):
    """Ad-hoc/extra deduction line items beyond PayrollLine's fixed columns."""

    __tablename__ = "payroll_deductions"

    id: Mapped[int] = mapped_column(primary_key=True)
    payroll_line_id: Mapped[int] = mapped_column(ForeignKey("payroll_lines.id"))
    deduction_type: Mapped[str] = mapped_column(String(100))
    amount: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)
    note: Mapped[Optional[str]] = mapped_column(String(255), nullable=True)
