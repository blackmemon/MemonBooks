"""Accounting + party (Vendor/Customer) models — spec sections 3.1, 3.2, 3.4, 10.

Vendor and Customer are colocated here rather than in a separate file: spec
section 14 does not list a dedicated model group for them, and both carry
Debit/Credit/Balance derived from the same JournalEntryLine ledger defined
in this module.

AccountGroup (spec: "if required") is omitted — Account's self-referential
parent_account_id already provides the hierarchy needed for the Tree View
(spec 3.4), so a separate grouping table would be redundant.
"""
from __future__ import annotations

import datetime as dt
from decimal import Decimal
from typing import Optional

from sqlalchemy import ForeignKey, Numeric, String, Text, UniqueConstraint
from sqlalchemy.orm import Mapped, mapped_column, relationship, object_session
from sqlalchemy import select, func

from database.database import UserBase
from backend.models.mixins import MasterStatusMixin, DocumentStatusMixin

VENDOR_TYPES = ("Supplier", "Contractor", "Distributor", "Manufacturer", "Other")
PAYMENT_METHODS = ("Cash", "Cheque", "Bank Transfer", "Digital Payment")
ACCOUNT_TYPES = ("Asset", "Liability", "Equity", "Revenue", "Expense", "Other")


class Vendor(MasterStatusMixin, UserBase):
    __tablename__ = "vendors"

    id: Mapped[int] = mapped_column(primary_key=True)
    vendor_code: Mapped[str] = mapped_column(String(40), unique=True, index=True)
    vendor_name: Mapped[str] = mapped_column(String(150))
    company_name: Mapped[str] = mapped_column(String(150))
    vendor_type: Mapped[str] = mapped_column(String(20), default="Supplier")
    phone: Mapped[str] = mapped_column(String(40))
    email: Mapped[Optional[str]] = mapped_column(String(150), nullable=True)
    website: Mapped[Optional[str]] = mapped_column(String(200), nullable=True)
    mailing_address: Mapped[str] = mapped_column(Text)
    shipping_address: Mapped[Optional[str]] = mapped_column(Text, nullable=True)
    payment_terms: Mapped[Optional[str]] = mapped_column(String(100), nullable=True)
    payment_method: Mapped[str] = mapped_column(String(30), default="Cash")
    description: Mapped[Optional[str]] = mapped_column(Text, nullable=True)

    @property
    def debit(self) -> Decimal:
        return _party_side_total(self, "Vendor", "debit")

    @property
    def credit(self) -> Decimal:
        return _party_side_total(self, "Vendor", "credit")

    @property
    def balance(self) -> Decimal:
        return self.credit - self.debit


class Customer(MasterStatusMixin, UserBase):
    __tablename__ = "customers"

    id: Mapped[int] = mapped_column(primary_key=True)
    customer_code: Mapped[str] = mapped_column(String(40), unique=True, index=True)
    customer_name: Mapped[str] = mapped_column(String(150))
    company_name: Mapped[Optional[str]] = mapped_column(String(150), nullable=True)
    phone: Mapped[Optional[str]] = mapped_column(String(40), nullable=True)
    email: Mapped[Optional[str]] = mapped_column(String(150), nullable=True)
    ntn: Mapped[Optional[str]] = mapped_column(String(40), nullable=True)
    strn: Mapped[Optional[str]] = mapped_column(String(40), nullable=True)
    mailing_address: Mapped[Optional[str]] = mapped_column(Text, nullable=True)
    shipping_address: Mapped[Optional[str]] = mapped_column(Text, nullable=True)
    description: Mapped[Optional[str]] = mapped_column(Text, nullable=True)

    @property
    def debit(self) -> Decimal:
        return _party_side_total(self, "Customer", "debit")

    @property
    def credit(self) -> Decimal:
        return _party_side_total(self, "Customer", "credit")

    @property
    def balance(self) -> Decimal:
        return self.debit - self.credit


class Account(MasterStatusMixin, UserBase):
    __tablename__ = "accounts"
    __table_args__ = (UniqueConstraint("account_code", name="uq_account_code"),)

    id: Mapped[int] = mapped_column(primary_key=True)
    account_code: Mapped[str] = mapped_column(String(40), index=True)
    account_name: Mapped[str] = mapped_column(String(150))
    account_type: Mapped[str] = mapped_column(String(20))
    parent_account_id: Mapped[Optional[int]] = mapped_column(
        ForeignKey("accounts.id"), nullable=True
    )
    description: Mapped[Optional[str]] = mapped_column(Text, nullable=True)

    parent: Mapped[Optional["Account"]] = relationship(remote_side=[id], back_populates="children")
    children: Mapped[list["Account"]] = relationship(back_populates="parent")

    @property
    def debit(self) -> Decimal:
        return _account_side_total(self, "debit")

    @property
    def credit(self) -> Decimal:
        return _account_side_total(self, "credit")

    @property
    def balance(self) -> Decimal:
        return self.debit - self.credit


class JournalEntry(DocumentStatusMixin, UserBase):
    __tablename__ = "journal_entries"

    id: Mapped[int] = mapped_column(primary_key=True)
    jv_no: Mapped[str] = mapped_column(String(40), unique=True, index=True)
    jv_date: Mapped[dt.date] = mapped_column()
    voucher_type: Mapped[str] = mapped_column(String(20), default="Adjustment")
    reference: Mapped[Optional[str]] = mapped_column(String(150), nullable=True)
    narration: Mapped[Optional[str]] = mapped_column(Text, nullable=True)
    source_document_type: Mapped[Optional[str]] = mapped_column(String(40), nullable=True)
    source_document_id: Mapped[Optional[int]] = mapped_column(nullable=True)

    lines: Mapped[list["JournalEntryLine"]] = relationship(
        back_populates="entry", cascade="all, delete-orphan"
    )


class JournalEntryLine(UserBase):
    __tablename__ = "journal_entry_lines"

    id: Mapped[int] = mapped_column(primary_key=True)
    entry_id: Mapped[int] = mapped_column(ForeignKey("journal_entries.id"))
    account_id: Mapped[int] = mapped_column(ForeignKey("accounts.id"))
    vendor_id: Mapped[Optional[int]] = mapped_column(ForeignKey("vendors.id"), nullable=True)
    customer_id: Mapped[Optional[int]] = mapped_column(ForeignKey("customers.id"), nullable=True)
    narration: Mapped[Optional[str]] = mapped_column(Text, nullable=True)
    debit: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)
    credit: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)

    entry: Mapped["JournalEntry"] = relationship(back_populates="lines")


def _account_side_total(account: Account, side: str) -> Decimal:
    """Sum of posted JournalEntryLine debit/credit for this account. Only
    Posted journal entries affect balances (spec section 10)."""
    session = object_session(account)
    if session is None or account.id is None:
        return Decimal("0")
    col = JournalEntryLine.debit if side == "debit" else JournalEntryLine.credit
    stmt = (
        select(func.coalesce(func.sum(col), 0))
        .join(JournalEntry, JournalEntryLine.entry_id == JournalEntry.id)
        .where(JournalEntryLine.account_id == account.id, JournalEntry.status == "Posted")
    )
    return Decimal(str(session.execute(stmt).scalar_one()))


def _party_side_total(party, party_type: str, side: str) -> Decimal:
    session = object_session(party)
    if session is None or party.id is None:
        return Decimal("0")
    col = JournalEntryLine.debit if side == "debit" else JournalEntryLine.credit
    party_col = JournalEntryLine.vendor_id if party_type == "Vendor" else JournalEntryLine.customer_id
    stmt = (
        select(func.coalesce(func.sum(col), 0))
        .join(JournalEntry, JournalEntryLine.entry_id == JournalEntry.id)
        .where(party_col == party.id, JournalEntry.status == "Posted")
    )
    return Decimal(str(session.execute(stmt).scalar_one()))
