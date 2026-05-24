"""
schemas.py — Pydantic schemas for Payments module.
Cash/Bank Receipts, Cash/Bank Payments, Journal Vouchers.
"""

from __future__ import annotations
from typing import Optional, List
from pydantic import BaseModel, Field


class IDResponse(BaseModel):
    id:     str
    number: Optional[str] = None
    detail: str = "Created successfully."


class MessageResponse(BaseModel):
    detail: str


class PayListResponse(BaseModel):
    items: list
    total: int
    page:  int = 1
    limit: int = 50


# ─────────────────────────────────────────────────────────────────────────────
# RECEIPT (Customer → Us)
# ─────────────────────────────────────────────────────────────────────────────

class ReceiptCreate(BaseModel):
    customer_id:          str
    receipt_date:         str  = Field(..., pattern=r'^\d{4}-\d{2}-\d{2}$')
    amount:               float = Field(..., gt=0)
    bank_account_id:      str   # COA account: cash or bank
    receivable_account_id: Optional[str] = None   # defaults to customer's AR account
    si_id:                Optional[str] = None    # link to sale invoice
    reference:            Optional[str] = Field(None, max_length=50)
    notes:                Optional[str] = None


# ─────────────────────────────────────────────────────────────────────────────
# PAYMENT (Us → Vendor)
# ─────────────────────────────────────────────────────────────────────────────

class PaymentCreate(BaseModel):
    vendor_id:         str
    payment_date:      str  = Field(..., pattern=r'^\d{4}-\d{2}-\d{2}$')
    amount:            float = Field(..., gt=0)
    wht_amount:        float = Field(0.0, ge=0)
    bank_account_id:   str   # COA account
    payable_account_id: Optional[str] = None   # defaults to vendor's AP account
    wht_account_id:    Optional[str] = None
    pi_id:             Optional[str] = None    # link to purchase invoice
    reference:         Optional[str] = Field(None, max_length=50)
    notes:             Optional[str] = None


# ─────────────────────────────────────────────────────────────────────────────
# JOURNAL VOUCHER (manual)
# ─────────────────────────────────────────────────────────────────────────────

class JVLineCreate(BaseModel):
    account_id: str
    debit:      float = Field(0.0, ge=0)
    credit:     float = Field(0.0, ge=0)
    narration:  Optional[str] = None


class JVCreate(BaseModel):
    jv_date:   str  = Field(..., pattern=r'^\d{4}-\d{2}-\d{2}$')
    narration: str  = Field(..., min_length=3, max_length=300)
    lines:     List[JVLineCreate] = Field(..., min_length=2)

    @classmethod
    def __get_validators__(cls):
        yield cls.validate_balanced

    @classmethod
    def validate_balanced(cls, v):
        if hasattr(v, 'lines') and v.lines:
            total_dr = sum(l.debit  for l in v.lines)
            total_cr = sum(l.credit for l in v.lines)
            if abs(total_dr - total_cr) > 0.01:
                raise ValueError(f"JV must be balanced: Dr={total_dr:.2f} Cr={total_cr:.2f}")
        return v
