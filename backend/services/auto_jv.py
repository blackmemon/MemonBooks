"""
auto_jv.py — Automatic Journal Voucher creation service.

Called by Purchase Invoice post, Sale Invoice post, Payment post etc.
Each function returns the jv_id created.
"""

import uuid
from datetime import datetime, timezone
from typing import Optional
from fastapi import HTTPException


def _now() -> str:
    return datetime.now(timezone.utc).isoformat()


def _next_jv_number(conn) -> str:
    """Get next JV number using the doc_sequences table."""
    try:
        from backend.services.numbering import next_number
        return next_number("JV", conn)
    except Exception:
        return f"JV-{datetime.now().strftime('%Y%m%d%H%M%S')}"


def _insert_jv(conn, *, jv_number: str, jv_date: str, narration: str,
                created_by: str, source_type: str, source_id: str) -> str:
    """Insert journal voucher header and return jv_id."""
    jv_id = str(uuid.uuid4())
    conn.execute(
        """INSERT INTO journal_vouchers
           (id, jv_number, jv_date, narration, status,
            source_type, source_id, created_by, created_at, updated_at)
           VALUES (?,?,?,?,'posted',?,?,?,?,?)""",
        (jv_id, jv_number, jv_date, narration,
         source_type, source_id, created_by, _now(), _now()),
    )
    return jv_id


def _insert_lines(conn, jv_id: str, lines: list[dict]) -> None:
    """
    lines: list of dicts with keys: account_id, debit, credit, narration
    """
    for i, line in enumerate(lines, 1):
        conn.execute(
            """INSERT INTO journal_voucher_lines
               (id, jv_id, line_no, account_id, debit, credit, narration)
               VALUES (?,?,?,?,?,?,?)""",
            (
                str(uuid.uuid4()), jv_id, i,
                line["account_id"],
                line.get("debit", 0.0),
                line.get("credit", 0.0),
                line.get("narration", ""),
            ),
        )


# ─────────────────────────────────────────────────────────────────────────────
# Purchase Invoice → JV
# ─────────────────────────────────────────────────────────────────────────────

def create_purchase_invoice_jv(
    conn,
    *,
    pi_id: str,
    pi_number: str,
    pi_date: str,
    vendor_name: str,
    vendor_ref: Optional[str],
    payable_account_id: str,
    lines: list[dict],   # each: {purchase_account_id, amount_net, tax_amt, wht_amt}
    tax_account_id: Optional[str],
    wht_account_id: Optional[str],
    grand_total: float,
    wht_total: float,
    created_by: str,
) -> str:
    """
    DR  Purchase/Expense accounts   (net amounts per line)
    DR  Input Tax Receivable         (tax total)
    CR  WHT Payable                  (wht total, if any)
    CR  Accounts Payable (Vendor)    (grand_total - wht_total)
    """
    jv_number = _next_jv_number(conn)
    ref = f" (Vendor Ref: {vendor_ref})" if vendor_ref else ""
    narration = f"Purchase Invoice {pi_number} — {vendor_name}{ref}"
    jv_id = _insert_jv(
        conn, jv_number=jv_number, jv_date=pi_date, narration=narration,
        created_by=created_by, source_type="purchase_invoice", source_id=pi_id,
    )

    jv_lines: list[dict] = []

    # DR: purchase/expense accounts per line
    for line in lines:
        acc = line.get("purchase_account_id")
        if not acc:
            continue
        net = line.get("amount_net", 0.0)
        if net:
            jv_lines.append({"account_id": acc, "debit": net, "credit": 0.0, "narration": narration})

    # DR: input tax (if configured)
    tax_total = sum(l.get("tax_amt", 0.0) for l in lines)
    if tax_total and tax_account_id:
        jv_lines.append({"account_id": tax_account_id, "debit": tax_total, "credit": 0.0, "narration": "Input Tax"})

    # CR: WHT payable
    if wht_total and wht_account_id:
        jv_lines.append({"account_id": wht_account_id, "debit": 0.0, "credit": wht_total, "narration": "WHT Payable"})

    # CR: accounts payable
    payable_amt = grand_total - wht_total
    jv_lines.append({"account_id": payable_account_id, "debit": 0.0, "credit": payable_amt, "narration": narration})

    _insert_lines(conn, jv_id, jv_lines)
    return jv_id


# ─────────────────────────────────────────────────────────────────────────────
# Sale Invoice → JV
# ─────────────────────────────────────────────────────────────────────────────

def create_sale_invoice_jv(
    conn,
    *,
    si_id: str,
    si_number: str,
    si_date: str,
    customer_name: str,
    receivable_account_id: str,
    lines: list[dict],   # each: {sale_account_id, amount_net, tax_amt}
    tax_account_id: Optional[str],
    grand_total: float,
    created_by: str,
) -> str:
    """
    DR  Accounts Receivable (Customer)   (grand_total)
    CR  Sales Revenue accounts           (net per line)
    CR  Output Tax Payable               (tax total)
    """
    jv_number = _next_jv_number(conn)
    narration = f"Sale Invoice {si_number} — {customer_name}"
    jv_id = _insert_jv(
        conn, jv_number=jv_number, jv_date=si_date, narration=narration,
        created_by=created_by, source_type="sale_invoice", source_id=si_id,
    )

    jv_lines: list[dict] = []

    # DR: receivable
    jv_lines.append({"account_id": receivable_account_id, "debit": grand_total, "credit": 0.0, "narration": narration})

    # CR: revenue per line
    for line in lines:
        acc = line.get("sale_account_id")
        net = line.get("amount_net", 0.0)
        if acc and net:
            jv_lines.append({"account_id": acc, "debit": 0.0, "credit": net, "narration": narration})

    # CR: output tax
    tax_total = sum(l.get("tax_amt", 0.0) for l in lines)
    if tax_total and tax_account_id:
        jv_lines.append({"account_id": tax_account_id, "debit": 0.0, "credit": tax_total, "narration": "Output Tax"})

    _insert_lines(conn, jv_id, jv_lines)
    return jv_id


# ─────────────────────────────────────────────────────────────────────────────
# Cash/Bank Payment → JV
# ─────────────────────────────────────────────────────────────────────────────

def create_payment_jv(
    conn,
    *,
    payment_id: str,
    payment_number: str,
    payment_date: str,
    payee_name: str,
    bank_account_id: str,
    payable_account_id: str,
    amount: float,
    wht_amount: float,
    wht_account_id: Optional[str],
    created_by: str,
    payment_type: str = "payment",  # "payment" | "receipt"
) -> str:
    """
    For payment (AP):
      DR  Accounts Payable    (amount + wht)
      CR  Bank/Cash           (amount)
      CR  WHT Payable         (wht, if any)

    For receipt (AR):
      DR  Bank/Cash           (amount)
      CR  Accounts Receivable (amount)
    """
    jv_number = _next_jv_number(conn)
    narration = f"{'Receipt' if payment_type == 'receipt' else 'Payment'}: {payment_number} — {payee_name}"
    jv_id = _insert_jv(
        conn, jv_number=jv_number, jv_date=payment_date, narration=narration,
        created_by=created_by,
        source_type=f"{'receipt' if payment_type == 'receipt' else 'payment'}",
        source_id=payment_id,
    )

    jv_lines: list[dict] = []

    if payment_type == "receipt":
        jv_lines.append({"account_id": bank_account_id,   "debit": amount, "credit": 0.0,    "narration": narration})
        jv_lines.append({"account_id": payable_account_id, "debit": 0.0,   "credit": amount, "narration": narration})
    else:
        total_payable = amount + wht_amount
        jv_lines.append({"account_id": payable_account_id, "debit": total_payable, "credit": 0.0,    "narration": narration})
        jv_lines.append({"account_id": bank_account_id,    "debit": 0.0,           "credit": amount, "narration": narration})
        if wht_amount and wht_account_id:
            jv_lines.append({"account_id": wht_account_id, "debit": 0.0, "credit": wht_amount, "narration": "WHT deducted"})

    _insert_lines(conn, jv_id, jv_lines)
    return jv_id


# ─────────────────────────────────────────────────────────────────────────────
# Purchase Return → JV
# ─────────────────────────────────────────────────────────────────────────────

def create_purchase_return_jv(
    conn,
    *,
    pr_id: str,
    pr_number: str,
    return_date: str,
    vendor_name: str,
    payable_account_id: str,
    purchase_account_id: str,
    grand_total: float,
    created_by: str,
) -> str:
    """
    DR  Accounts Payable     (vendor credit)
    CR  Purchase/Expense a/c (reversal)
    """
    jv_number = _next_jv_number(conn)
    narration = f"Purchase Return {pr_number} — {vendor_name}"
    jv_id = _insert_jv(
        conn, jv_number=jv_number, jv_date=return_date, narration=narration,
        created_by=created_by, source_type="purchase_return", source_id=pr_id,
    )
    _insert_lines(conn, jv_id, [
        {"account_id": payable_account_id,  "debit": grand_total, "credit": 0.0,         "narration": narration},
        {"account_id": purchase_account_id, "debit": 0.0,         "credit": grand_total, "narration": narration},
    ])
    return jv_id


# ─────────────────────────────────────────────────────────────────────────────
# Sale Return → JV
# ─────────────────────────────────────────────────────────────────────────────

def create_sale_return_jv(
    conn,
    *,
    sr_id: str,
    sr_number: str,
    return_date: str,
    customer_name: str,
    receivable_account_id: str,
    sale_account_id: str,
    grand_total: float,
    created_by: str,
) -> str:
    """
    DR  Sales Returns (revenue reversal)
    CR  Accounts Receivable
    """
    jv_number = _next_jv_number(conn)
    narration = f"Sale Return {sr_number} — {customer_name}"
    jv_id = _insert_jv(
        conn, jv_number=jv_number, jv_date=return_date, narration=narration,
        created_by=created_by, source_type="sale_return", source_id=sr_id,
    )
    _insert_lines(conn, jv_id, [
        {"account_id": sale_account_id,        "debit": grand_total, "credit": 0.0,         "narration": narration},
        {"account_id": receivable_account_id,  "debit": 0.0,         "credit": grand_total, "narration": narration},
    ])
    return jv_id


# ─────────────────────────────────────────────────────────────────────────────
# Payroll → JV
# ─────────────────────────────────────────────────────────────────────────────

def create_payroll_jv(
    conn,
    *,
    run_id: str,
    run_number: str,
    run_date: str,
    month: int,
    year: int,
    total_gross: float,
    total_income_tax: float,
    total_deductions: float,
    total_net: float,
    salary_expense_account_id: str,
    tax_payable_account_id: str,
    deduction_payable_account_id: str,
    bank_account_id: str,
    created_by: str,
) -> str:
    """
    DR  Salary Expense          (total_gross)
    CR  Income Tax Payable      (total_income_tax)
    CR  Deductions Payable      (total_deductions)
    CR  Bank / Payable Account  (total_net)
    """
    jv_number = _next_jv_number(conn)
    narration = f"Payroll {run_number} — {year}-{month:02d}"
    jv_id = _insert_jv(
        conn, jv_number=jv_number, jv_date=run_date, narration=narration,
        created_by=created_by, source_type="payroll", source_id=run_id,
    )
    lines = [
        {"account_id": salary_expense_account_id, "debit": total_gross, "credit": 0.0, "narration": narration},
    ]
    if total_income_tax > 0:
        lines.append({"account_id": tax_payable_account_id, "debit": 0.0, "credit": total_income_tax, "narration": "Income Tax Payable"})
    if total_deductions > 0:
        lines.append({"account_id": deduction_payable_account_id, "debit": 0.0, "credit": total_deductions, "narration": "Salary Deductions Payable"})
    lines.append({"account_id": bank_account_id, "debit": 0.0, "credit": total_net, "narration": "Net Salary Payable"})

    _insert_lines(conn, jv_id, lines)
    return jv_id
