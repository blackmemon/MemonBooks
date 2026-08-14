"""Purchase, Sales and Cash/Bank transaction documents — spec sections 6, 7, 8.

Models only in this pass (no posting/service logic yet — see stock_service /
accounting_service stubs). Header tables use DocumentStatusMixin (spec 2.1/2.2);
line tables carry just the commercial fields.

Journal Voucher is intentionally NOT duplicated here as JournalVoucher /
JournalVoucherLine even though spec section 14 lists it under this module:
account_models.JournalEntry / JournalEntryLine already implements exactly
the fields spec 8.10.3 describes for the JV document (auto document no,
date, voucher type, reference, narration, account/narration/debit/credit
lines), and is also the table accounting_service posts into for every other
document type. A second parallel ledger table would just duplicate the same
debit/credit lines under a different name.

Address fields marked "Auto" in the spec (e.g. Vendor Address on a PO) are
stored as a text snapshot taken at creation time rather than a live
lookup, so a document's printed address doesn't retroactively change if the
vendor's master address is edited later (spec section 10: historical
records must remain traceable).
"""
from __future__ import annotations

import datetime as dt
from decimal import Decimal
from typing import Optional

from sqlalchemy import ForeignKey, Numeric, String, Text
from sqlalchemy.orm import Mapped, mapped_column, relationship

from database.database import UserBase
from backend.models.mixins import DocumentStatusMixin

SHIPPING_VIA = ("Road", "Air", "Sea", "Courier")
PRICE_TYPES = ("Standard", "Wholesale", "Retail", "Custom")
PAYMENT_MODES = ("Cash", "Cheque", "NEFT", "RTGS", "UPI")
RECEIPT_TYPES = ("Party", "Petty", "Multiparty")


# ---------------------------------------------------------------- Purchase


class PurchaseOrder(DocumentStatusMixin, UserBase):
    __tablename__ = "purchase_orders"

    id: Mapped[int] = mapped_column(primary_key=True)
    po_no: Mapped[str] = mapped_column(String(40), unique=True, index=True)
    po_date: Mapped[dt.date] = mapped_column()
    vendor_id: Mapped[int] = mapped_column(ForeignKey("vendors.id"))
    vendor_reference_no: Mapped[Optional[str]] = mapped_column(String(100), nullable=True)
    vendor_address: Mapped[Optional[str]] = mapped_column(Text, nullable=True)
    expected_date: Mapped[Optional[dt.date]] = mapped_column(nullable=True)
    shipping_via: Mapped[Optional[str]] = mapped_column(String(20), nullable=True)
    subtotal: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)
    discount_total: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)
    tax_total: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)
    grand_total: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)
    terms_conditions: Mapped[Optional[str]] = mapped_column(Text, nullable=True)
    notes: Mapped[Optional[str]] = mapped_column(Text, nullable=True)

    lines: Mapped[list["PurchaseOrderLine"]] = relationship(cascade="all, delete-orphan")


class PurchaseOrderLine(UserBase):
    __tablename__ = "purchase_order_lines"

    id: Mapped[int] = mapped_column(primary_key=True)
    po_id: Mapped[int] = mapped_column(ForeignKey("purchase_orders.id"))
    line_no: Mapped[int] = mapped_column()
    item_id: Mapped[int] = mapped_column(ForeignKey("items.id"))
    description: Mapped[Optional[str]] = mapped_column(Text, nullable=True)
    qty: Mapped[Decimal] = mapped_column(Numeric(18, 3), default=0)
    rate: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)
    amount: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)
    tax_percent: Mapped[Decimal] = mapped_column(Numeric(5, 2), default=0)
    tax_amount: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)
    total: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)


class GoodsReceipt(DocumentStatusMixin, UserBase):
    __tablename__ = "goods_receipts"

    id: Mapped[int] = mapped_column(primary_key=True)
    grn_no: Mapped[str] = mapped_column(String(40), unique=True, index=True)
    grn_date: Mapped[dt.date] = mapped_column()
    po_id: Mapped[Optional[int]] = mapped_column(ForeignKey("purchase_orders.id"), nullable=True)
    vendor_id: Mapped[int] = mapped_column(ForeignKey("vendors.id"))
    vendor_invoice_no: Mapped[Optional[str]] = mapped_column(String(100), nullable=True)
    receiving_location_id: Mapped[int] = mapped_column(ForeignKey("inventory_locations.id"))
    vendor_address: Mapped[Optional[str]] = mapped_column(Text, nullable=True)
    total_received: Mapped[Decimal] = mapped_column(Numeric(18, 3), default=0)
    total_accepted: Mapped[Decimal] = mapped_column(Numeric(18, 3), default=0)
    total_rejected: Mapped[Decimal] = mapped_column(Numeric(18, 3), default=0)
    total_amount: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)
    notes: Mapped[Optional[str]] = mapped_column(Text, nullable=True)

    lines: Mapped[list["GoodsReceiptLine"]] = relationship(cascade="all, delete-orphan")


class GoodsReceiptLine(UserBase):
    __tablename__ = "goods_receipt_lines"

    id: Mapped[int] = mapped_column(primary_key=True)
    grn_id: Mapped[int] = mapped_column(ForeignKey("goods_receipts.id"))
    line_no: Mapped[int] = mapped_column()
    item_id: Mapped[int] = mapped_column(ForeignKey("items.id"))
    description: Mapped[Optional[str]] = mapped_column(Text, nullable=True)
    ordered_qty: Mapped[Decimal] = mapped_column(Numeric(18, 3), default=0)
    received_qty: Mapped[Decimal] = mapped_column(Numeric(18, 3), default=0)
    accepted_qty: Mapped[Decimal] = mapped_column(Numeric(18, 3), default=0)
    rejected_qty: Mapped[Decimal] = mapped_column(Numeric(18, 3), default=0)
    unit_cost: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)
    total: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)


class PurchaseInvoice(DocumentStatusMixin, UserBase):
    __tablename__ = "purchase_invoices"

    id: Mapped[int] = mapped_column(primary_key=True)
    pi_no: Mapped[str] = mapped_column(String(40), unique=True, index=True)
    pi_date: Mapped[dt.date] = mapped_column()
    grn_id: Mapped[Optional[int]] = mapped_column(ForeignKey("goods_receipts.id"), nullable=True)
    po_id: Mapped[Optional[int]] = mapped_column(ForeignKey("purchase_orders.id"), nullable=True)
    vendor_id: Mapped[int] = mapped_column(ForeignKey("vendors.id"))
    vendor_invoice_no: Mapped[Optional[str]] = mapped_column(String(100), nullable=True)
    vendor_invoice_date: Mapped[Optional[dt.date]] = mapped_column(nullable=True)
    due_date: Mapped[Optional[dt.date]] = mapped_column(nullable=True)
    vendor_address: Mapped[Optional[str]] = mapped_column(Text, nullable=True)
    subtotal: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)
    discount_total: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)
    tax_total: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)
    grand_total: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)
    amount_paid: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)
    payment_account_id: Mapped[Optional[int]] = mapped_column(ForeignKey("accounts.id"), nullable=True)
    balance_due: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)
    notes: Mapped[Optional[str]] = mapped_column(Text, nullable=True)

    lines: Mapped[list["PurchaseInvoiceLine"]] = relationship(cascade="all, delete-orphan")


class PurchaseInvoiceLine(UserBase):
    __tablename__ = "purchase_invoice_lines"

    id: Mapped[int] = mapped_column(primary_key=True)
    pi_id: Mapped[int] = mapped_column(ForeignKey("purchase_invoices.id"))
    line_no: Mapped[int] = mapped_column()
    item_id: Mapped[int] = mapped_column(ForeignKey("items.id"))
    description: Mapped[Optional[str]] = mapped_column(Text, nullable=True)
    qty: Mapped[Decimal] = mapped_column(Numeric(18, 3), default=0)
    unit_price: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)
    discount: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)
    tax_percent: Mapped[Decimal] = mapped_column(Numeric(5, 2), default=0)
    tax_amount: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)
    total: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)


class PurchaseReturn(DocumentStatusMixin, UserBase):
    __tablename__ = "purchase_returns"

    id: Mapped[int] = mapped_column(primary_key=True)
    return_no: Mapped[str] = mapped_column(String(40), unique=True, index=True)
    return_date: Mapped[dt.date] = mapped_column()
    vendor_id: Mapped[int] = mapped_column(ForeignKey("vendors.id"))
    purchase_invoice_id: Mapped[Optional[int]] = mapped_column(
        ForeignKey("purchase_invoices.id"), nullable=True
    )
    address: Mapped[Optional[str]] = mapped_column(Text, nullable=True)
    reason: Mapped[Optional[str]] = mapped_column(Text, nullable=True)

    lines: Mapped[list["PurchaseReturnLine"]] = relationship(cascade="all, delete-orphan")


class PurchaseReturnLine(UserBase):
    __tablename__ = "purchase_return_lines"

    id: Mapped[int] = mapped_column(primary_key=True)
    return_id: Mapped[int] = mapped_column(ForeignKey("purchase_returns.id"))
    line_no: Mapped[int] = mapped_column()
    item_id: Mapped[int] = mapped_column(ForeignKey("items.id"))
    description: Mapped[Optional[str]] = mapped_column(Text, nullable=True)
    original_qty: Mapped[Decimal] = mapped_column(Numeric(18, 3), default=0)
    return_qty: Mapped[Decimal] = mapped_column(Numeric(18, 3), default=0)
    unit_price: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)
    tax_percent: Mapped[Decimal] = mapped_column(Numeric(5, 2), default=0)
    tax_amount: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)
    total: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)


# ------------------------------------------------------------------- Sales


class Quotation(DocumentStatusMixin, UserBase):
    __tablename__ = "quotations"

    id: Mapped[int] = mapped_column(primary_key=True)
    quotation_no: Mapped[str] = mapped_column(String(40), unique=True, index=True)
    quotation_date: Mapped[dt.date] = mapped_column()
    valid_until: Mapped[Optional[dt.date]] = mapped_column(nullable=True)
    customer_id: Mapped[int] = mapped_column(ForeignKey("customers.id"))
    customer_reference: Mapped[Optional[str]] = mapped_column(String(100), nullable=True)
    customer_address: Mapped[Optional[str]] = mapped_column(Text, nullable=True)
    subtotal: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)
    discount_total: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)
    tax_total: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)
    grand_total: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)
    terms_conditions: Mapped[Optional[str]] = mapped_column(Text, nullable=True)
    notes: Mapped[Optional[str]] = mapped_column(Text, nullable=True)

    lines: Mapped[list["QuotationLine"]] = relationship(cascade="all, delete-orphan")


class QuotationLine(UserBase):
    __tablename__ = "quotation_lines"

    id: Mapped[int] = mapped_column(primary_key=True)
    quotation_id: Mapped[int] = mapped_column(ForeignKey("quotations.id"))
    line_no: Mapped[int] = mapped_column()
    item_id: Mapped[int] = mapped_column(ForeignKey("items.id"))
    description: Mapped[Optional[str]] = mapped_column(Text, nullable=True)
    qty: Mapped[Decimal] = mapped_column(Numeric(18, 3), default=0)
    price_type: Mapped[Optional[str]] = mapped_column(String(20), nullable=True)
    unit_price: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)
    discount: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)
    tax_percent: Mapped[Decimal] = mapped_column(Numeric(5, 2), default=0)
    tax_amount: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)
    total: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)


class SalesOrder(DocumentStatusMixin, UserBase):
    __tablename__ = "sales_orders"

    id: Mapped[int] = mapped_column(primary_key=True)
    order_no: Mapped[str] = mapped_column(String(40), unique=True, index=True)
    order_date: Mapped[dt.date] = mapped_column()
    quotation_id: Mapped[Optional[int]] = mapped_column(ForeignKey("quotations.id"), nullable=True)
    customer_id: Mapped[int] = mapped_column(ForeignKey("customers.id"))
    customer_po: Mapped[Optional[str]] = mapped_column(String(100), nullable=True)
    delivery_date: Mapped[Optional[dt.date]] = mapped_column(nullable=True)
    delivery_address: Mapped[Optional[str]] = mapped_column(Text, nullable=True)
    subtotal: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)
    discount_total: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)
    tax_total: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)
    grand_total: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)
    notes: Mapped[Optional[str]] = mapped_column(Text, nullable=True)

    lines: Mapped[list["SalesOrderLine"]] = relationship(cascade="all, delete-orphan")


class SalesOrderLine(UserBase):
    __tablename__ = "sales_order_lines"

    id: Mapped[int] = mapped_column(primary_key=True)
    order_id: Mapped[int] = mapped_column(ForeignKey("sales_orders.id"))
    line_no: Mapped[int] = mapped_column()
    item_id: Mapped[int] = mapped_column(ForeignKey("items.id"))
    description: Mapped[Optional[str]] = mapped_column(Text, nullable=True)
    qty: Mapped[Decimal] = mapped_column(Numeric(18, 3), default=0)
    price_type: Mapped[Optional[str]] = mapped_column(String(20), nullable=True)
    unit_price: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)
    discount: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)
    tax_percent: Mapped[Decimal] = mapped_column(Numeric(5, 2), default=0)
    tax_amount: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)
    total: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)


class DeliveryChallan(DocumentStatusMixin, UserBase):
    __tablename__ = "delivery_challans"

    id: Mapped[int] = mapped_column(primary_key=True)
    dc_no: Mapped[str] = mapped_column(String(40), unique=True, index=True)
    dc_date: Mapped[dt.date] = mapped_column()
    customer_id: Mapped[int] = mapped_column(ForeignKey("customers.id"))
    order_id: Mapped[Optional[int]] = mapped_column(ForeignKey("sales_orders.id"), nullable=True)
    vehicle_no: Mapped[Optional[str]] = mapped_column(String(40), nullable=True)
    driver_name: Mapped[Optional[str]] = mapped_column(String(150), nullable=True)
    driver_phone: Mapped[Optional[str]] = mapped_column(String(40), nullable=True)
    delivery_address: Mapped[Optional[str]] = mapped_column(Text, nullable=True)
    location_id: Mapped[int] = mapped_column(ForeignKey("inventory_locations.id"))
    notes: Mapped[Optional[str]] = mapped_column(Text, nullable=True)

    lines: Mapped[list["DeliveryChallanLine"]] = relationship(cascade="all, delete-orphan")


class DeliveryChallanLine(UserBase):
    __tablename__ = "delivery_challan_lines"

    id: Mapped[int] = mapped_column(primary_key=True)
    dc_id: Mapped[int] = mapped_column(ForeignKey("delivery_challans.id"))
    line_no: Mapped[int] = mapped_column()
    item_id: Mapped[int] = mapped_column(ForeignKey("items.id"))
    description: Mapped[Optional[str]] = mapped_column(Text, nullable=True)
    ordered_qty: Mapped[Decimal] = mapped_column(Numeric(18, 3), default=0)
    delivered_qty: Mapped[Decimal] = mapped_column(Numeric(18, 3), default=0)
    unit_price: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)
    total: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)


class SalesInvoice(DocumentStatusMixin, UserBase):
    __tablename__ = "sales_invoices"

    id: Mapped[int] = mapped_column(primary_key=True)
    invoice_no: Mapped[str] = mapped_column(String(40), unique=True, index=True)
    invoice_date: Mapped[dt.date] = mapped_column()
    dc_id: Mapped[Optional[int]] = mapped_column(ForeignKey("delivery_challans.id"), nullable=True)
    order_id: Mapped[Optional[int]] = mapped_column(ForeignKey("sales_orders.id"), nullable=True)
    customer_id: Mapped[int] = mapped_column(ForeignKey("customers.id"))
    due_date: Mapped[Optional[dt.date]] = mapped_column(nullable=True)
    location_id: Mapped[Optional[int]] = mapped_column(ForeignKey("inventory_locations.id"), nullable=True)
    delivery_address: Mapped[Optional[str]] = mapped_column(Text, nullable=True)
    subtotal: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)
    discount_total: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)
    tax_total: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)
    grand_total: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)
    amount_received: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)
    payment_account_id: Mapped[Optional[int]] = mapped_column(ForeignKey("accounts.id"), nullable=True)
    balance_due: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)
    notes: Mapped[Optional[str]] = mapped_column(Text, nullable=True)

    lines: Mapped[list["SalesInvoiceLine"]] = relationship(cascade="all, delete-orphan")


class SalesInvoiceLine(UserBase):
    __tablename__ = "sales_invoice_lines"

    id: Mapped[int] = mapped_column(primary_key=True)
    invoice_id: Mapped[int] = mapped_column(ForeignKey("sales_invoices.id"))
    line_no: Mapped[int] = mapped_column()
    item_id: Mapped[int] = mapped_column(ForeignKey("items.id"))
    description: Mapped[Optional[str]] = mapped_column(Text, nullable=True)
    qty: Mapped[Decimal] = mapped_column(Numeric(18, 3), default=0)
    price_type: Mapped[Optional[str]] = mapped_column(String(20), nullable=True)
    unit_price: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)
    discount: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)
    tax_percent: Mapped[Decimal] = mapped_column(Numeric(5, 2), default=0)
    tax_amount: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)
    total: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)


class SalesReturn(DocumentStatusMixin, UserBase):
    __tablename__ = "sales_returns"

    id: Mapped[int] = mapped_column(primary_key=True)
    return_no: Mapped[str] = mapped_column(String(40), unique=True, index=True)
    return_date: Mapped[dt.date] = mapped_column()
    customer_id: Mapped[int] = mapped_column(ForeignKey("customers.id"))
    sales_invoice_id: Mapped[Optional[int]] = mapped_column(ForeignKey("sales_invoices.id"), nullable=True)
    address: Mapped[Optional[str]] = mapped_column(Text, nullable=True)
    reason: Mapped[Optional[str]] = mapped_column(Text, nullable=True)

    lines: Mapped[list["SalesReturnLine"]] = relationship(cascade="all, delete-orphan")


class SalesReturnLine(UserBase):
    __tablename__ = "sales_return_lines"

    id: Mapped[int] = mapped_column(primary_key=True)
    return_id: Mapped[int] = mapped_column(ForeignKey("sales_returns.id"))
    line_no: Mapped[int] = mapped_column()
    item_id: Mapped[int] = mapped_column(ForeignKey("items.id"))
    description: Mapped[Optional[str]] = mapped_column(Text, nullable=True)
    original_qty: Mapped[Decimal] = mapped_column(Numeric(18, 3), default=0)
    return_qty: Mapped[Decimal] = mapped_column(Numeric(18, 3), default=0)
    unit_price: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)
    tax_percent: Mapped[Decimal] = mapped_column(Numeric(5, 2), default=0)
    tax_amount: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)
    total: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)


# ------------------------------------------------------------- Cash / Bank


class Receipt(DocumentStatusMixin, UserBase):
    __tablename__ = "receipts"

    id: Mapped[int] = mapped_column(primary_key=True)
    voucher_no: Mapped[str] = mapped_column(String(40), unique=True, index=True)
    receipt_type: Mapped[str] = mapped_column(String(20), default="Party")
    receipt_date: Mapped[dt.date] = mapped_column()
    customer_id: Mapped[Optional[int]] = mapped_column(ForeignKey("customers.id"), nullable=True)
    address: Mapped[Optional[str]] = mapped_column(Text, nullable=True)
    contra_account_id: Mapped[Optional[int]] = mapped_column(ForeignKey("accounts.id"), nullable=True)
    cash_bank_account_id: Mapped[int] = mapped_column(ForeignKey("accounts.id"))
    payment_mode: Mapped[Optional[str]] = mapped_column(String(20), nullable=True)
    cheque_no: Mapped[Optional[str]] = mapped_column(String(60), nullable=True)
    cheque_date: Mapped[Optional[dt.date]] = mapped_column(nullable=True)
    drawee_bank: Mapped[Optional[str]] = mapped_column(String(150), nullable=True)
    total_amount: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)
    notes: Mapped[Optional[str]] = mapped_column(Text, nullable=True)

    lines: Mapped[list["ReceiptLine"]] = relationship(cascade="all, delete-orphan")


class ReceiptLine(UserBase):
    __tablename__ = "receipt_lines"

    id: Mapped[int] = mapped_column(primary_key=True)
    receipt_id: Mapped[int] = mapped_column(ForeignKey("receipts.id"))
    line_no: Mapped[int] = mapped_column()
    party_customer_id: Mapped[Optional[int]] = mapped_column(ForeignKey("customers.id"), nullable=True)
    amount: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)
    invoice_references: Mapped[Optional[str]] = mapped_column(Text, nullable=True)


class Payment(DocumentStatusMixin, UserBase):
    __tablename__ = "payments"

    id: Mapped[int] = mapped_column(primary_key=True)
    voucher_no: Mapped[str] = mapped_column(String(40), unique=True, index=True)
    payment_type: Mapped[Optional[str]] = mapped_column(String(20), nullable=True)
    payment_date: Mapped[dt.date] = mapped_column()
    vendor_id: Mapped[Optional[int]] = mapped_column(ForeignKey("vendors.id"), nullable=True)
    address: Mapped[Optional[str]] = mapped_column(Text, nullable=True)
    contra_account_id: Mapped[Optional[int]] = mapped_column(ForeignKey("accounts.id"), nullable=True)
    cash_bank_account_id: Mapped[int] = mapped_column(ForeignKey("accounts.id"))
    payment_mode: Mapped[Optional[str]] = mapped_column(String(20), nullable=True)
    cheque_no: Mapped[Optional[str]] = mapped_column(String(60), nullable=True)
    cheque_date: Mapped[Optional[dt.date]] = mapped_column(nullable=True)
    drawee_bank: Mapped[Optional[str]] = mapped_column(String(150), nullable=True)
    total_amount: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)
    wht_rate_percent: Mapped[Decimal] = mapped_column(Numeric(5, 2), default=0)
    wht_amount: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)
    net_amount_to_pay: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)
    notes: Mapped[Optional[str]] = mapped_column(Text, nullable=True)

    lines: Mapped[list["PaymentLine"]] = relationship(cascade="all, delete-orphan")


class PaymentLine(UserBase):
    __tablename__ = "payment_lines"

    id: Mapped[int] = mapped_column(primary_key=True)
    payment_id: Mapped[int] = mapped_column(ForeignKey("payments.id"))
    line_no: Mapped[int] = mapped_column()
    vendor_payee_id: Mapped[Optional[int]] = mapped_column(ForeignKey("vendors.id"), nullable=True)
    amount: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)
    wht: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)
    wht_amount: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)
    net_amount: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)
    invoice_references: Mapped[Optional[str]] = mapped_column(Text, nullable=True)
