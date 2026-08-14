"""Item, Location and stock ledger models — spec sections 3.3, 3.5, 9.

ItemCategory is omitted: spec 3.3's Category/Sub-Category dependency is a
fixed lookup (Raw Material -> Metal/Plastic/Textile/Chemical/Other, etc.),
so category/sub_category are validated Strings (utils/validators.py)
rather than a separate table — same reasoning as omitting AccountGroup.

ItemLocationBalance is defined per spec section 14 for future use by
stock_service, but is not populated in this pass (no posting service
exists yet for GRN/DC/etc.). location_stock_dialog computes quantity on
hand live from StockMovement instead of relying on this cache table, to
avoid ever displaying a stale balance while the cache goes unmaintained.
"""
from __future__ import annotations

import datetime as dt
from decimal import Decimal
from typing import Optional

from sqlalchemy import ForeignKey, Numeric, String, Text, UniqueConstraint, select, func
from sqlalchemy.orm import Mapped, mapped_column, relationship, object_session

from database.database import UserBase
from backend.models.mixins import MasterStatusMixin, TimestampMixin

ITEM_TYPES = ("Raw Material", "Finished Goods", "Service", "Other")
UNITS_OF_MEASURE = ("Pcs", "Kg", "Liter", "Box", "Pack", "Meter", "Dozen", "Unit", "Other")
LOCATION_TYPES = ("Warehouse", "Store", "Office", "Other")

CATEGORY_MAP = {
    "Raw Material": ("Metal", "Plastic", "Textile", "Chemical", "Other"),
    "Finished Goods": ("Retail", "Wholesale", "Export", "Other"),
    "Service": ("Labor", "Consulting", "Maintenance", "Other"),
    "Other": ("Other",),
}


class Item(MasterStatusMixin, UserBase):
    __tablename__ = "items"

    id: Mapped[int] = mapped_column(primary_key=True)
    item_code: Mapped[str] = mapped_column(String(40), unique=True, index=True)
    item_name: Mapped[str] = mapped_column(String(150))
    item_type: Mapped[str] = mapped_column(String(20), default="Finished Goods")
    category: Mapped[Optional[str]] = mapped_column(String(40), nullable=True)
    sub_category: Mapped[Optional[str]] = mapped_column(String(40), nullable=True)
    unit_of_measure: Mapped[Optional[str]] = mapped_column(String(20), nullable=True)
    purchase_price: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)
    sale_price: Mapped[Decimal] = mapped_column(Numeric(18, 2), default=0)
    primary_image_path: Mapped[Optional[str]] = mapped_column(String(300), nullable=True)
    description: Mapped[Optional[str]] = mapped_column(Text, nullable=True)

    @property
    def is_stockable(self) -> bool:
        return self.item_type != "Service"

    @property
    def quantity_on_hand(self) -> Decimal:
        session = object_session(self)
        if session is None or self.id is None:
            return Decimal("0")
        stmt = select(
            func.coalesce(func.sum(StockMovement.quantity_in), 0)
            - func.coalesce(func.sum(StockMovement.quantity_out), 0)
        ).where(StockMovement.item_id == self.id)
        return Decimal(str(session.execute(stmt).scalar_one()))

    def quantity_by_location(self) -> dict[int, Decimal]:
        session = object_session(self)
        if session is None or self.id is None:
            return {}
        stmt = (
            select(
                StockMovement.location_id,
                func.coalesce(func.sum(StockMovement.quantity_in), 0)
                - func.coalesce(func.sum(StockMovement.quantity_out), 0),
            )
            .where(StockMovement.item_id == self.id)
            .group_by(StockMovement.location_id)
        )
        return {row[0]: Decimal(str(row[1])) for row in session.execute(stmt)}


class InventoryLocation(MasterStatusMixin, UserBase):
    __tablename__ = "inventory_locations"

    id: Mapped[int] = mapped_column(primary_key=True)
    location_code: Mapped[str] = mapped_column(String(40), unique=True, index=True)
    location_name: Mapped[str] = mapped_column(String(150))
    location_type: Mapped[str] = mapped_column(String(20), default="Warehouse")
    is_stockable: Mapped[bool] = mapped_column(default=True)
    contact_person: Mapped[Optional[str]] = mapped_column(String(150), nullable=True)
    phone: Mapped[Optional[str]] = mapped_column(String(40), nullable=True)
    email: Mapped[Optional[str]] = mapped_column(String(150), nullable=True)
    mailing_address: Mapped[Optional[str]] = mapped_column(Text, nullable=True)
    shipping_address: Mapped[Optional[str]] = mapped_column(Text, nullable=True)
    capacity: Mapped[Optional[Decimal]] = mapped_column(Numeric(18, 2), nullable=True)
    description: Mapped[Optional[str]] = mapped_column(Text, nullable=True)

    def stock_summary(self) -> list[dict]:
        """Item Code / Name / Unit / Qty on Hand for the location_stock_dialog."""
        session = object_session(self)
        if session is None or self.id is None:
            return []
        stmt = (
            select(
                Item.id,
                Item.item_code,
                Item.item_name,
                Item.unit_of_measure,
                func.coalesce(func.sum(StockMovement.quantity_in), 0)
                - func.coalesce(func.sum(StockMovement.quantity_out), 0),
                func.max(StockMovement.movement_date),
            )
            .join(StockMovement, StockMovement.item_id == Item.id)
            .where(StockMovement.location_id == self.id)
            .group_by(Item.id)
        )
        return [
            {
                "item_id": row[0],
                "item_code": row[1],
                "item_name": row[2],
                "unit": row[3],
                "quantity_on_hand": Decimal(str(row[4])),
                "last_movement_date": row[5],
            }
            for row in session.execute(stmt)
        ]


class StockMovement(TimestampMixin, UserBase):
    __tablename__ = "stock_movements"
    __table_args__ = (UniqueConstraint("posting_key", name="uq_stock_movement_posting_key"),)

    id: Mapped[int] = mapped_column(primary_key=True)
    movement_date: Mapped[dt.date] = mapped_column()
    item_id: Mapped[int] = mapped_column(ForeignKey("items.id"))
    location_id: Mapped[int] = mapped_column(ForeignKey("inventory_locations.id"))
    quantity_in: Mapped[Decimal] = mapped_column(Numeric(18, 3), default=0)
    quantity_out: Mapped[Decimal] = mapped_column(Numeric(18, 3), default=0)
    unit_cost: Mapped[Optional[Decimal]] = mapped_column(Numeric(18, 2), nullable=True)
    reference_document_type: Mapped[str] = mapped_column(String(40))
    reference_document_id: Mapped[int] = mapped_column()
    source_reason: Mapped[Optional[str]] = mapped_column(String(200), nullable=True)
    # Unique per (document type, document id, item, line) so re-posting or
    # re-saving a document can never duplicate a movement (spec 9.3).
    posting_key: Mapped[str] = mapped_column(String(150))


class ItemLocationBalance(UserBase):
    """Reserved for a future cached-balance optimization; not populated yet
    (see module docstring). Kept so stock_service has a stable schema to
    write into once GRN/DC posting is implemented."""

    __tablename__ = "item_location_balances"
    __table_args__ = (UniqueConstraint("item_id", "location_id", name="uq_item_location"),)

    id: Mapped[int] = mapped_column(primary_key=True)
    item_id: Mapped[int] = mapped_column(ForeignKey("items.id"))
    location_id: Mapped[int] = mapped_column(ForeignKey("inventory_locations.id"))
    quantity_on_hand: Mapped[Decimal] = mapped_column(Numeric(18, 3), default=0)
    reserved_quantity: Mapped[Decimal] = mapped_column(Numeric(18, 3), default=0)
    last_movement_date: Mapped[Optional[dt.date]] = mapped_column(nullable=True)
