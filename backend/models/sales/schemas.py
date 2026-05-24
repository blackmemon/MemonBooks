"""
schemas.py — Pydantic v2 schemas for Sales module.
Covers: Sale Order (SO), Delivery Challan (DC), Sale Invoice (SI), Sale Return (SR).
"""

from __future__ import annotations
from typing import Optional, List
from pydantic import BaseModel, Field, field_validator


class IDResponse(BaseModel):
    id:     str
    number: Optional[str] = None
    detail: str = "Created successfully."


class MessageResponse(BaseModel):
    detail: str


class SalesListResponse(BaseModel):
    items: list
    total: int
    page:  int = 1
    limit: int = 50


# ─────────────────────────────────────────────────────────────────────────────
# SALE ORDER
# ─────────────────────────────────────────────────────────────────────────────

class SOLineCreate(BaseModel):
    item_id:      str
    description:  Optional[str] = None
    uom_id:       Optional[str] = None
    qty:          float = Field(..., gt=0)
    unit_price:   float = Field(..., ge=0)
    discount_pct: float = Field(0.0, ge=0, le=100)
    tax_id:       Optional[str] = None
    location_id:  Optional[str] = None


class SOCreate(BaseModel):
    customer_id:     str
    so_date:         str = Field(..., pattern=r'^\d{4}-\d{2}-\d{2}$')
    delivery_date:   Optional[str] = Field(None, pattern=r'^\d{4}-\d{2}-\d{2}$')
    currency_id:     Optional[str] = None
    payment_term_id: Optional[str] = None
    location_id:     Optional[str] = None
    notes:           Optional[str] = None
    lines:           List[SOLineCreate] = Field(..., min_length=1)


class SOUpdate(BaseModel):
    customer_id:     Optional[str] = None
    so_date:         Optional[str] = None
    delivery_date:   Optional[str] = None
    currency_id:     Optional[str] = None
    payment_term_id: Optional[str] = None
    location_id:     Optional[str] = None
    notes:           Optional[str] = None
    lines:           Optional[List[SOLineCreate]] = None


# ─────────────────────────────────────────────────────────────────────────────
# DELIVERY CHALLAN
# ─────────────────────────────────────────────────────────────────────────────

class DCLineCreate(BaseModel):
    so_line_id:    str
    item_id:       str
    uom_id:        Optional[str] = None
    qty_delivered: float = Field(..., ge=0)
    location_id:   Optional[str] = None


class DCCreate(BaseModel):
    so_id:       str
    dc_date:     str = Field(..., pattern=r'^\d{4}-\d{2}-\d{2}$')
    location_id: Optional[str] = None
    notes:       Optional[str] = None
    lines:       List[DCLineCreate] = Field(..., min_length=1)


# ─────────────────────────────────────────────────────────────────────────────
# SALE INVOICE
# ─────────────────────────────────────────────────────────────────────────────

class SILineCreate(BaseModel):
    so_line_id:   Optional[str] = None
    dc_line_id:   Optional[str] = None
    item_id:      str
    description:  Optional[str] = None
    uom_id:       Optional[str] = None
    qty:          float = Field(..., gt=0)
    unit_price:   float = Field(..., ge=0)
    discount_pct: float = Field(0.0, ge=0, le=100)
    tax_id:       Optional[str] = None


class SICreate(BaseModel):
    customer_id:          str
    so_id:                Optional[str] = None
    dc_id:                Optional[str] = None
    invoice_date:         str = Field(..., pattern=r'^\d{4}-\d{2}-\d{2}$')
    due_date:             Optional[str] = Field(None, pattern=r'^\d{4}-\d{2}-\d{2}$')
    currency_id:          Optional[str] = None
    payment_term_id:      Optional[str] = None
    receivable_account_id: Optional[str] = None
    notes:                Optional[str] = None
    lines:                List[SILineCreate] = Field(..., min_length=1)


class SIUpdate(BaseModel):
    invoice_date: Optional[str] = None
    due_date:     Optional[str] = None
    notes:        Optional[str] = None


# ─────────────────────────────────────────────────────────────────────────────
# SALE RETURN
# ─────────────────────────────────────────────────────────────────────────────

class SRLineCreate(BaseModel):
    si_line_id:  Optional[str] = None
    item_id:     str
    uom_id:      Optional[str] = None
    qty:         float = Field(..., gt=0)
    unit_price:  float = Field(..., ge=0)
    reason:      Optional[str] = None


class SRCreate(BaseModel):
    customer_id: str
    si_id:       Optional[str] = None
    return_date: str = Field(..., pattern=r'^\d{4}-\d{2}-\d{2}$')
    notes:       Optional[str] = None
    lines:       List[SRLineCreate] = Field(..., min_length=1)
