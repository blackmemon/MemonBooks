"""
schemas.py — Pydantic v2 request/response schemas for all Masters modules.
COA, Customers, Vendors, Items, UOM, Tax, PaymentTerms, Currencies,
Inventory Locations, Fiscal Years.
"""

from __future__ import annotations
from datetime import date
from typing import Optional, List
from pydantic import BaseModel, Field, field_validator, model_validator
import re


# ─────────────────────────────────────────────────────────────────────────────
# Common
# ─────────────────────────────────────────────────────────────────────────────

class IDResponse(BaseModel):
    id: str
    detail: str = "Created successfully."


class MessageResponse(BaseModel):
    detail: str


# ─────────────────────────────────────────────────────────────────────────────
# CHART OF ACCOUNTS
# ─────────────────────────────────────────────────────────────────────────────

class COACreate(BaseModel):
    code:         str        = Field(..., min_length=2,  max_length=20)
    name:         str        = Field(..., min_length=2,  max_length=150)
    account_type: str        = Field(..., pattern=r'^(asset|liability|equity|income|expense)$')
    parent_id:    Optional[str] = None
    is_control:   bool       = False
    currency_id:  Optional[str] = None
    description:  Optional[str] = None

    @field_validator('code')
    @classmethod
    def code_must_be_numeric_or_alphanumeric(cls, v: str) -> str:
        if not re.match(r'^[A-Za-z0-9\-]+$', v):
            raise ValueError('COA code must be alphanumeric.')
        return v.upper()


class COAUpdate(BaseModel):
    name:         Optional[str] = Field(None, min_length=2, max_length=150)
    is_control:   Optional[bool] = None
    currency_id:  Optional[str] = None
    description:  Optional[str] = None
    is_active:    Optional[bool] = None


class COANode(BaseModel):
    id:           str
    code:         str
    name:         str
    account_type: str
    parent_id:    Optional[str]
    level:        int
    is_control:   bool
    is_active:    bool
    children:     List['COANode'] = []

    model_config = {"from_attributes": True}


class COAFlat(BaseModel):
    id:           str
    code:         str
    name:         str
    account_type: str
    parent_id:    Optional[str]
    level:        int
    is_control:   bool
    is_active:    bool
    display_name: Optional[str] = None   # indented for dropdowns

    model_config = {"from_attributes": True}


# ─────────────────────────────────────────────────────────────────────────────
# CUSTOMERS
# ─────────────────────────────────────────────────────────────────────────────

class CustomerCreate(BaseModel):
    code:            str              = Field(..., min_length=1, max_length=20)
    name:            str              = Field(..., min_length=2, max_length=150)
    ntn:             Optional[str]    = Field(None, max_length=20)
    strn:            Optional[str]    = Field(None, max_length=20)
    cnic:            Optional[str]    = Field(None, max_length=20)
    phone:           Optional[str]    = Field(None, max_length=20)
    email:           Optional[str]    = Field(None, max_length=100)
    address:         Optional[str]    = Field(None, max_length=300)
    city:            Optional[str]    = Field(None, max_length=50)
    bank_iban:       Optional[str]    = Field(None, max_length=34)
    credit_limit:    float            = Field(0.0,  ge=0)
    credit_days:     int              = Field(30,   ge=0, le=365)
    payment_term_id: Optional[str]    = None
    currency_id:     Optional[str]    = None
    account_id:      Optional[str]    = None

    @field_validator('email')
    @classmethod
    def valid_email(cls, v):
        if v and not re.match(r'^[^\s@]+@[^\s@]+\.[^\s@]+$', v):
            raise ValueError('Invalid email address.')
        return v

    @field_validator('cnic')
    @classmethod
    def valid_cnic(cls, v):
        if v and not re.match(r'^\d{5}-\d{7}-\d$', v.replace(' ', '')):
            raise ValueError('CNIC format: 12345-6789012-3')
        return v

    @field_validator('bank_iban')
    @classmethod
    def valid_iban(cls, v):
        if v:
            clean = v.replace(' ', '').upper()
            if not re.match(r'^[A-Z]{2}\d{2}[A-Z0-9]{1,30}$', clean):
                raise ValueError('Invalid IBAN format.')
            return clean
        return v


class CustomerUpdate(CustomerCreate):
    code: Optional[str] = None
    name: Optional[str] = None
    is_active: Optional[bool] = None


class CustomerOut(BaseModel):
    id:              str
    code:            str
    name:            str
    ntn:             Optional[str]
    strn:            Optional[str]
    phone:           Optional[str]
    email:           Optional[str]
    address:         Optional[str]
    city:            Optional[str]
    credit_limit:    float
    credit_days:     int
    is_active:       bool
    created_at:      str

    model_config = {"from_attributes": True}


class CustomerDetail(CustomerOut):
    cnic:            Optional[str]
    bank_iban:       Optional[str]
    payment_term_id: Optional[str]
    currency_id:     Optional[str]
    account_id:      Optional[str]
    updated_at:      str


# ─────────────────────────────────────────────────────────────────────────────
# VENDORS
# ─────────────────────────────────────────────────────────────────────────────

class VendorCreate(BaseModel):
    code:            str           = Field(..., min_length=1, max_length=20)
    name:            str           = Field(..., min_length=2, max_length=150)
    ntn:             Optional[str] = Field(None, max_length=20)
    strn:            Optional[str] = Field(None, max_length=20)
    cnic:            Optional[str] = Field(None, max_length=20)
    phone:           Optional[str] = Field(None, max_length=20)
    email:           Optional[str] = Field(None, max_length=100)
    address:         Optional[str] = Field(None, max_length=300)
    city:            Optional[str] = Field(None, max_length=50)
    bank_name:       Optional[str] = Field(None, max_length=100)
    bank_iban:       Optional[str] = Field(None, max_length=34)
    bank_account:    Optional[str] = Field(None, max_length=30)
    wht_rate:        float         = Field(0.0, ge=0, le=100)
    wht_account_id:  Optional[str] = None
    credit_limit:    float         = Field(0.0, ge=0)
    credit_days:     int           = Field(30,  ge=0, le=365)
    payment_term_id: Optional[str] = None
    currency_id:     Optional[str] = None
    account_id:      Optional[str] = None

    @field_validator('email')
    @classmethod
    def valid_email(cls, v):
        if v and not re.match(r'^[^\s@]+@[^\s@]+\.[^\s@]+$', v):
            raise ValueError('Invalid email address.')
        return v

    @field_validator('cnic')
    @classmethod
    def valid_cnic(cls, v):
        if v and not re.match(r'^\d{5}-\d{7}-\d$', v.replace(' ', '')):
            raise ValueError('CNIC format: 12345-6789012-3')
        return v

    @field_validator('bank_iban')
    @classmethod
    def valid_iban(cls, v):
        if v:
            clean = v.replace(' ', '').upper()
            if not re.match(r'^[A-Z]{2}\d{2}[A-Z0-9]{1,30}$', clean):
                raise ValueError('Invalid IBAN format.')
            return clean
        return v


class VendorUpdate(VendorCreate):
    code: Optional[str] = None
    name: Optional[str] = None
    is_active: Optional[bool] = None


class VendorOut(BaseModel):
    id:           str
    code:         str
    name:         str
    ntn:          Optional[str]
    strn:         Optional[str]
    phone:        Optional[str]
    email:        Optional[str]
    city:         Optional[str]
    wht_rate:     float
    credit_limit: float
    credit_days:  int
    is_active:    bool
    created_at:   str

    model_config = {"from_attributes": True}


class VendorDetail(VendorOut):
    cnic:            Optional[str]
    address:         Optional[str]
    bank_name:       Optional[str]
    bank_iban:       Optional[str]
    bank_account:    Optional[str]
    wht_account_id:  Optional[str]
    payment_term_id: Optional[str]
    currency_id:     Optional[str]
    account_id:      Optional[str]
    updated_at:      str


# ─────────────────────────────────────────────────────────────────────────────
# ITEMS
# ─────────────────────────────────────────────────────────────────────────────

class ItemCreate(BaseModel):
    code:                str           = Field(..., min_length=1, max_length=30)
    name:                str           = Field(..., min_length=2, max_length=150)
    description:         Optional[str] = None
    item_type:           str           = Field(..., pattern=r'^(product|service|raw_material)$')
    uom_id:              Optional[str] = None
    purchase_uom_id:     Optional[str] = None
    sale_uom_id:         Optional[str] = None
    purchase_account_id: Optional[str] = None
    sale_account_id:     Optional[str] = None
    inventory_account_id:Optional[str] = None
    purchase_tax_id:     Optional[str] = None
    sale_tax_id:         Optional[str] = None
    standard_cost:       float         = Field(0.0, ge=0)
    standard_price:      float         = Field(0.0, ge=0)
    min_stock:           float         = Field(0.0, ge=0)
    max_stock:           float         = Field(0.0, ge=0)


class ItemUpdate(ItemCreate):
    code: Optional[str] = None
    name: Optional[str] = None
    item_type: Optional[str] = None
    is_active: Optional[bool] = None


class ItemOut(BaseModel):
    id:              str
    code:            str
    name:            str
    item_type:       str
    standard_cost:   float
    standard_price:  float
    min_stock:       float
    max_stock:       float
    uom_code:        Optional[str] = None
    is_active:       bool
    created_at:      str

    model_config = {"from_attributes": True}


class PriceListCreate(BaseModel):
    item_id:     str
    customer_id: Optional[str] = None
    min_qty:     float = Field(1.0, ge=0)
    price:       float = Field(..., ge=0)
    valid_from:  Optional[str] = None
    valid_to:    Optional[str] = None


# ─────────────────────────────────────────────────────────────────────────────
# UOM
# ─────────────────────────────────────────────────────────────────────────────

class UOMCreate(BaseModel):
    code: str = Field(..., min_length=1, max_length=10)
    name: str = Field(..., min_length=1, max_length=50)

    @field_validator('code')
    @classmethod
    def uppercase_code(cls, v): return v.upper()


class UOMUpdate(BaseModel):
    name:      Optional[str]  = None
    is_active: Optional[bool] = None


class UOMOut(BaseModel):
    id:        str
    code:      str
    name:      str
    is_active: bool

    model_config = {"from_attributes": True}


class UOMConversionCreate(BaseModel):
    from_uom_id: str
    to_uom_id:   str
    factor:      float = Field(..., gt=0)


# ─────────────────────────────────────────────────────────────────────────────
# CURRENCY
# ─────────────────────────────────────────────────────────────────────────────

class CurrencyCreate(BaseModel):
    code:          str   = Field(..., min_length=3, max_length=3)
    name:          str   = Field(..., min_length=2, max_length=80)
    symbol:        str   = Field(..., min_length=1, max_length=5)
    exchange_rate: float = Field(..., gt=0)
    is_base:       bool  = False

    @field_validator('code')
    @classmethod
    def uppercase_code(cls, v): return v.upper()


class CurrencyUpdate(BaseModel):
    exchange_rate: Optional[float] = Field(None, gt=0)
    name:          Optional[str]   = None
    symbol:        Optional[str]   = None
    is_active:     Optional[bool]  = None


class CurrencyOut(BaseModel):
    id:            str
    code:          str
    name:          str
    symbol:        str
    exchange_rate: float
    is_base:       bool
    is_active:     bool

    model_config = {"from_attributes": True}


# ─────────────────────────────────────────────────────────────────────────────
# TAX CONFIG
# ─────────────────────────────────────────────────────────────────────────────

class TaxCreate(BaseModel):
    name:       str   = Field(..., min_length=2, max_length=100)
    tax_type:   str   = Field(..., pattern=r'^(GST|WHT|FED|SED|custom)$')
    rate:       float = Field(..., ge=0, le=100)
    account_id: Optional[str] = None
    applies_to: str   = Field(..., pattern=r'^(purchases|sales|both)$')


class TaxUpdate(BaseModel):
    name:       Optional[str]   = None
    rate:       Optional[float] = Field(None, ge=0, le=100)
    account_id: Optional[str]   = None
    is_active:  Optional[bool]  = None


class TaxOut(BaseModel):
    id:         str
    name:       str
    tax_type:   str
    rate:       float
    applies_to: str
    is_active:  bool

    model_config = {"from_attributes": True}


# ─────────────────────────────────────────────────────────────────────────────
# PAYMENT TERMS
# ─────────────────────────────────────────────────────────────────────────────

class PaymentTermCreate(BaseModel):
    name:          str            = Field(..., min_length=2, max_length=80)
    due_days:      int            = Field(0,   ge=0, le=365)
    discount_days: Optional[int]  = Field(None, ge=0)
    discount_pct:  Optional[float]= Field(None, ge=0, le=100)


class PaymentTermOut(BaseModel):
    id:            str
    name:          str
    due_days:      int
    discount_days: Optional[int]
    discount_pct:  Optional[float]
    is_active:     bool

    model_config = {"from_attributes": True}


# ─────────────────────────────────────────────────────────────────────────────
# INVENTORY LOCATIONS
# ─────────────────────────────────────────────────────────────────────────────

class LocationCreate(BaseModel):
    code:    str           = Field(..., min_length=1, max_length=20)
    name:    str           = Field(..., min_length=2, max_length=100)
    address: Optional[str] = None

    @field_validator('code')
    @classmethod
    def uppercase_code(cls, v): return v.upper()


class LocationUpdate(BaseModel):
    name:      Optional[str]  = None
    address:   Optional[str]  = None
    is_active: Optional[bool] = None


class LocationOut(BaseModel):
    id:        str
    code:      str
    name:      str
    address:   Optional[str]
    is_active: bool

    model_config = {"from_attributes": True}


# ─────────────────────────────────────────────────────────────────────────────
# FISCAL YEARS
# ─────────────────────────────────────────────────────────────────────────────

class FiscalYearCreate(BaseModel):
    name:       str = Field(..., min_length=2, max_length=30)
    start_date: str = Field(..., pattern=r'^\d{4}-\d{2}-\d{2}$')
    end_date:   str = Field(..., pattern=r'^\d{4}-\d{2}-\d{2}$')

    @model_validator(mode='after')
    def end_after_start(self) -> 'FiscalYearCreate':
        if self.start_date and self.end_date:
            if self.end_date <= self.start_date:
                raise ValueError('end_date must be after start_date.')
        return self


class FiscalYearOut(BaseModel):
    id:         str
    name:       str
    start_date: str
    end_date:   str
    is_active:  bool
    is_closed:  bool

    model_config = {"from_attributes": True}


# ─────────────────────────────────────────────────────────────────────────────
# LIST RESPONSE WRAPPER
# ─────────────────────────────────────────────────────────────────────────────

class ListResponse(BaseModel):
    items:  list
    total:  int
    page:   int = 1
    limit:  int = 50
