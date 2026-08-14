"""Data access for Maintain Management (spec sections 3.1-3.5). Pure CRUD +
filtered listing — no validation, no numbering, no audit; that belongs to
backend/controllers/maintain_controller.py per spec section 16 rule 1."""
from __future__ import annotations

from typing import Optional, Type, TypeVar

from sqlalchemy import select
from sqlalchemy.orm import Session

from backend.models.account_models import Account, Customer, Vendor
from backend.models.inventory_models import InventoryLocation, Item

T = TypeVar("T")


def _list(db: Session, model: Type[T], order_by, status: str = "All") -> list[T]:
    stmt = select(model).order_by(order_by)
    if status != "All":
        stmt = stmt.where(model.status == status)
    return list(db.execute(stmt).scalars())


def _get(db: Session, model: Type[T], record_id: int) -> Optional[T]:
    return db.get(model, record_id)


def _create(db: Session, model: Type[T], **fields) -> T:
    instance = model(**fields)
    db.add(instance)
    db.flush()
    return instance


def _update(db: Session, model: Type[T], record_id: int, **fields) -> T:
    instance = db.get(model, record_id)
    if instance is None:
        raise ValueError(f"{model.__name__} {record_id} not found.")
    for key, value in fields.items():
        setattr(instance, key, value)
    db.flush()
    return instance


# ---- Vendor -----------------------------------------------------------

def list_vendors(db: Session, status: str = "All") -> list[Vendor]:
    return _list(db, Vendor, Vendor.vendor_name, status)


def get_vendor(db: Session, vendor_id: int) -> Optional[Vendor]:
    return _get(db, Vendor, vendor_id)


def create_vendor(db: Session, **fields) -> Vendor:
    return _create(db, Vendor, **fields)


def update_vendor(db: Session, vendor_id: int, **fields) -> Vendor:
    return _update(db, Vendor, vendor_id, **fields)


# ---- Customer -----------------------------------------------------------

def list_customers(db: Session, status: str = "All") -> list[Customer]:
    return _list(db, Customer, Customer.customer_name, status)


def get_customer(db: Session, customer_id: int) -> Optional[Customer]:
    return _get(db, Customer, customer_id)


def create_customer(db: Session, **fields) -> Customer:
    return _create(db, Customer, **fields)


def update_customer(db: Session, customer_id: int, **fields) -> Customer:
    return _update(db, Customer, customer_id, **fields)


# ---- Item -----------------------------------------------------------

def list_items(db: Session, status: str = "All") -> list[Item]:
    return _list(db, Item, Item.item_name, status)


def get_item(db: Session, item_id: int) -> Optional[Item]:
    return _get(db, Item, item_id)


def create_item(db: Session, **fields) -> Item:
    return _create(db, Item, **fields)


def update_item(db: Session, item_id: int, **fields) -> Item:
    return _update(db, Item, item_id, **fields)


# ---- Chart of Accounts -----------------------------------------------------------

def list_accounts(db: Session, status: str = "All") -> list[Account]:
    return _list(db, Account, Account.account_code, status)


def get_account(db: Session, account_id: int) -> Optional[Account]:
    return _get(db, Account, account_id)


def create_account(db: Session, **fields) -> Account:
    return _create(db, Account, **fields)


def update_account(db: Session, account_id: int, **fields) -> Account:
    return _update(db, Account, account_id, **fields)


def account_has_children(db: Session, account_id: int) -> bool:
    stmt = select(Account.id).where(Account.parent_account_id == account_id)
    return db.execute(stmt).first() is not None


# ---- Inventory Location -----------------------------------------------------------

def list_locations(db: Session, status: str = "All") -> list[InventoryLocation]:
    return _list(db, InventoryLocation, InventoryLocation.location_name, status)


def get_location(db: Session, location_id: int) -> Optional[InventoryLocation]:
    return _get(db, InventoryLocation, location_id)


def create_location(db: Session, **fields) -> InventoryLocation:
    return _create(db, InventoryLocation, **fields)


def update_location(db: Session, location_id: int, **fields) -> InventoryLocation:
    return _update(db, InventoryLocation, location_id, **fields)
