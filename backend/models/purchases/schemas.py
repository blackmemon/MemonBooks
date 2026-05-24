"""
schemas.py — Pydantic v2 schemas for Purchases module.
Covers: Purchase Order (PO), GRN, Purchase Invoice (PI), Purchase Return (PR).
"""

from __future__ import annotations
from typing import Optional, List
from pydantic import BaseModel, Field, field_validator, model_validator
import re


# ─────────────────────────────────────────────────────────────────────────────
# Common
# ─────────────────────────────────────────────────────────────────────────────

class IDResponse(BaseModel):
    id:     str
    number: Optional[str] = None
    detail: str = "Created successfully."


class MessageResponse(BaseModel):
    detail: str


# ─────────────────────────────────────────────────────────────────────────────
# PURCHASE ORDER
# ─────────────────────────────────────────────────────────────────────────────

class POLineCreate(BaseModel):
    item_id:       str
    description:   Optional[str] = None
    uom_id:        Optional[str] = None
    qty:           float = Field(..., gt=0)
    unit_price:    float = Field(..., ge=0)
    discount_pct:  float = Field(0.0, ge=0, le=100)
    tax_id:        Optional[str] = None
    location_id:   Optional[str] = None


class POLineUpdate(BaseModel):
    item_id:      Optional[str]   = None
    description:  Optional[str]   = None
    uom_id:       Optional[str]   = None
    qty:          Optional[float] = Field(None, gt=0)
    unit_price:   Optional[float] = Field(None, ge=0)
    discount_pct: Optional[float] = Field(None, ge=0, le=100)
    tax_id:       Optional[str]   = None
    location_id:  Optional[str]   = None


class POCreate(BaseModel):
    vendor_id:       str
    po_date:         str = Field(..., pattern=r'^\d{4}-\d{2}-\d{2}$')
    expected_date:   Optional[str] = Field(None, pattern=r'^\d{4}-\d{2}-\d{2}$')
    currency_id:     Optional[str] = None
    payment_term_id: Optional[str] = None
    location_id:     Optional[str] = None
    notes:           Optional[str] = None
    lines:           List[POLineCreate] = Field(..., min_length=1)

    @field_validator('lines')
    @classmethod
    def lines_not_empty(cls, v):
        if not v:
            raise ValueError('Purchase order must have at least one line.')
        return v


class POUpdate(BaseModel):
    vendor_id:       Optional[str] = None
    po_date:         Optional[str] = Field(None, pattern=r'^\d{4}-\d{2}-\d{2}$')
    expected_date:   Optional[str] = Field(None, pattern=r'^\d{4}-\d{2}-\d{2}$')
    currency_id:     Optional[str] = None
    payment_term_id: Optional[str] = None
    location_id:     Optional[str] = None
    notes:           Optional[str] = None
    lines:           Optional[List[POLineCreate]] = None


class POLineOut(BaseModel):
    id:            str
    item_id:       str
    item_code:     Optional[str] = None
    item_name:     Optional[str] = None
    description:   Optional[str] = None
    uom_id:        Optional[str] = None
    uom_code:      Optional[str] = None
    qty:           float
    unit_price:    float
    discount_pct:  float
    discount_amt:  float
    tax_rate:      float
    tax_amt:       float
    line_total:    float
    qty_received:  float
    qty_billed:    float
    location_id:   Optional[str] = None

    model_config = {"from_attributes": True}


class POOut(BaseModel):
    id:              str
    po_number:       str
    vendor_id:       str
    vendor_name:     Optional[str] = None
    po_date:         str
    expected_date:   Optional[str] = None
    status:          str
    currency_id:     Optional[str] = None
    payment_term_id: Optional[str] = None
    subtotal:        float
    tax_total:       float
    discount_total:  float
    grand_total:     float
    notes:           Optional[str] = None
    created_at:      str
    created_by:      Optional[str] = None
    lines:           List[POLineOut] = []

    model_config = {"from_attributes": True}


# ─────────────────────────────────────────────────────────────────────────────
# GRN (Goods Receipt Note)
# ─────────────────────────────────────────────────────────────────────────────

class GRNLineCreate(BaseModel):
    po_line_id:       str
    item_id:          str
    qty_received:     float = Field(..., ge=0)
    qty_accepted:     float = Field(..., ge=0)
    qty_rejected:     float = Field(0.0, ge=0)
    rejection_reason: Optional[str] = None
    location_id:      Optional[str] = None

    @model_validator(mode='after')
    def accepted_plus_rejected_eq_received(self) -> 'GRNLineCreate':
        total = self.qty_accepted + self.qty_rejected
        if abs(total - self.qty_received) > 0.0001:
            raise ValueError('Accepted + Rejected must equal Received quantity.')
        return self


class GRNCreate(BaseModel):
    po_id:        str
    grn_date:     str = Field(..., pattern=r'^\d{4}-\d{2}-\d{2}$')
    location_id:  Optional[str] = None
    notes:        Optional[str] = None
    lines:        List[GRNLineCreate] = Field(..., min_length=1)


class GRNLineOut(BaseModel):
    id:               str
    po_line_id:       str
    item_id:          str
    item_code:        Optional[str] = None
    item_name:        Optional[str] = None
    uom_code:         Optional[str] = None
    qty_ordered:      float
    qty_received:     float
    qty_accepted:     float
    qty_rejected:     float
    rejection_reason: Optional[str] = None
    location_id:      Optional[str] = None
    location_name:    Optional[str] = None

    model_config = {"from_attributes": True}


class GRNOut(BaseModel):
    id:          str
    grn_number:  str
    po_id:       str
    po_number:   Optional[str] = None
    vendor_name: Optional[str] = None
    grn_date:    str
    status:      str
    location_id: Optional[str] = None
    notes:       Optional[str] = None
    created_at:  str
    created_by:  Optional[str] = None
    lines:       List[GRNLineOut] = []

    model_config = {"from_attributes": True}


# ─────────────────────────────────────────────────────────────────────────────
# PURCHASE INVOICE (PI)
# ─────────────────────────────────────────────────────────────────────────────

class PILineCreate(BaseModel):
    grn_line_id:  Optional[str] = None
    po_line_id:   Optional[str] = None
    item_id:      str
    description:  Optional[str] = None
    uom_id:       Optional[str] = None
    qty:          float = Field(..., gt=0)
    unit_price:   float = Field(..., ge=0)
    discount_pct: float = Field(0.0, ge=0, le=100)
    tax_id:       Optional[str] = None
    wht_rate:     float = Field(0.0, ge=0, le=100)


class PICreate(BaseModel):
    vendor_id:       str
    po_id:           Optional[str] = None
    grn_id:          Optional[str] = None
    invoice_date:    str = Field(..., pattern=r'^\d{4}-\d{2}-\d{2}$')
    due_date:        Optional[str] = Field(None, pattern=r'^\d{4}-\d{2}-\d{2}$')
    vendor_ref:      Optional[str] = Field(None, max_length=50, description='Vendor invoice number')
    currency_id:     Optional[str] = None
    payment_term_id: Optional[str] = None
    payable_account_id: Optional[str] = None
    notes:           Optional[str] = None
    lines:           List[PILineCreate] = Field(..., min_length=1)


class PIUpdate(BaseModel):
    invoice_date:    Optional[str] = None
    due_date:        Optional[str] = None
    vendor_ref:      Optional[str] = None
    notes:           Optional[str] = None


class PILineOut(BaseModel):
    id:           str
    item_id:      str
    item_code:    Optional[str] = None
    item_name:    Optional[str] = None
    description:  Optional[str] = None
    uom_code:     Optional[str] = None
    qty:          float
    unit_price:   float
    discount_pct: float
    discount_amt: float
    tax_rate:     float
    tax_amt:      float
    wht_rate:     float
    wht_amt:      float
    line_total:   float

    model_config = {"from_attributes": True}


class PIOut(BaseModel):
    id:                 str
    pi_number:          str
    vendor_id:          str
    vendor_name:        Optional[str] = None
    po_id:              Optional[str] = None
    po_number:          Optional[str] = None
    grn_id:             Optional[str] = None
    grn_number:         Optional[str] = None
    invoice_date:       str
    due_date:           Optional[str] = None
    vendor_ref:         Optional[str] = None
    status:             str
    currency_id:        Optional[str] = None
    payment_term_id:    Optional[str] = None
    payable_account_id: Optional[str] = None
    subtotal:           float
    discount_total:     float
    tax_total:          float
    wht_total:          float
    grand_total:        float
    amount_paid:        float
    amount_due:         float
    notes:              Optional[str] = None
    jv_id:              Optional[str] = None
    created_at:         str
    created_by:         Optional[str] = None
    lines:              List[PILineOut] = []

    model_config = {"from_attributes": True}


# ─────────────────────────────────────────────────────────────────────────────
# PURCHASE RETURN (PR)
# ─────────────────────────────────────────────────────────────────────────────

class PRLineCreate(BaseModel):
    grn_line_id: Optional[str] = None
    item_id:     str
    uom_id:      Optional[str] = None
    qty:         float = Field(..., gt=0)
    unit_price:  float = Field(..., ge=0)
    reason:      Optional[str] = None


class PRCreate(BaseModel):
    vendor_id:   str
    grn_id:      Optional[str] = None
    po_id:       Optional[str] = None
    return_date: str = Field(..., pattern=r'^\d{4}-\d{2}-\d{2}$')
    notes:       Optional[str] = None
    lines:       List[PRLineCreate] = Field(..., min_length=1)


class PROut(BaseModel):
    id:          str
    pr_number:   str
    vendor_id:   str
    vendor_name: Optional[str] = None
    grn_id:      Optional[str] = None
    grn_number:  Optional[str] = None
    return_date: str
    status:      str
    grand_total: float
    notes:       Optional[str] = None
    jv_id:       Optional[str] = None
    created_at:  str

    model_config = {"from_attributes": True}


# ─────────────────────────────────────────────────────────────────────────────
# List wrapper
# ─────────────────────────────────────────────────────────────────────────────

class PurchaseListResponse(BaseModel):
    items: list
    total: int
    page:  int = 1
    limit: int = 50
