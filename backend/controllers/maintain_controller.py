"""Business rules for Maintain Management (spec sections 3.1-3.5, 15, 16).

Pages call only these functions — never the repository or ORM models
directly (spec section 16 rule 1). Each function opens its own user.db
session, validates input, assigns numbering, writes an audit entry, and
returns plain dicts (not ORM instances) so the UI never holds objects bound
to a session that's about to close.
"""
from __future__ import annotations

from typing import Optional

from database.database import get_user_session
from backend.repositories import maintain_repository as repo
from backend.models.account_models import ACCOUNT_TYPES, PAYMENT_METHODS, VENDOR_TYPES
from backend.models.inventory_models import CATEGORY_MAP, ITEM_TYPES, LOCATION_TYPES, UNITS_OF_MEASURE
from utils import validators as v
from utils.audit import record as audit_record
from utils.numbering import generate_number
from security.auth import current_username


# ---------------------------------------------------------------- Vendor

def _serialize_vendor(vendor) -> dict:
    return {
        "id": vendor.id,
        "vendor_code": vendor.vendor_code,
        "vendor_name": vendor.vendor_name,
        "company_name": vendor.company_name,
        "vendor_type": vendor.vendor_type,
        "phone": vendor.phone,
        "email": vendor.email,
        "website": vendor.website,
        "mailing_address": vendor.mailing_address,
        "shipping_address": vendor.shipping_address,
        "payment_terms": vendor.payment_terms,
        "payment_method": vendor.payment_method,
        "description": vendor.description,
        "status": vendor.status,
        "debit": vendor.debit,
        "credit": vendor.credit,
        "balance": vendor.balance,
        "created_at": vendor.created_at,
        "updated_at": vendor.updated_at,
    }


def _validate_vendor_fields(data: dict) -> dict:
    return {
        "vendor_name": v.require_min_length(data.get("vendor_name"), "Vendor Name", 3),
        "company_name": v.require_min_length(data.get("company_name"), "Company Name", 3),
        "vendor_type": v.validate_choice(data.get("vendor_type"), "Vendor Type", VENDOR_TYPES),
        "phone": v.validate_phone(data.get("phone"), "Phone", required=True),
        "email": v.validate_email(data.get("email"), "Email"),
        "website": (data.get("website") or "").strip() or None,
        "mailing_address": v.require_min_length(data.get("mailing_address"), "Mailing Address", 5),
        "shipping_address": (data.get("shipping_address") or "").strip() or None,
        "payment_terms": (data.get("payment_terms") or "").strip() or None,
        "payment_method": v.validate_choice(data.get("payment_method"), "Payment Method", PAYMENT_METHODS),
        "description": (data.get("description") or "").strip() or None,
    }


def list_vendors(status: str = "All") -> list[dict]:
    with get_user_session() as db:
        return [_serialize_vendor(x) for x in repo.list_vendors(db, status)]


def create_vendor(data: dict) -> dict:
    fields = _validate_vendor_fields(data)
    with get_user_session() as db:
        fields["vendor_code"] = generate_number("vendor")
        fields["created_by"] = fields["updated_by"] = current_username()
        vendor = repo.create_vendor(db, **fields)
        db.flush()
        audit_record(db, username=current_username(), action="Create", record_type="Vendor", record_id=vendor.id)
        return _serialize_vendor(vendor)


def update_vendor(vendor_id: int, data: dict) -> dict:
    fields = _validate_vendor_fields(data)
    with get_user_session() as db:
        fields["updated_by"] = current_username()
        vendor = repo.update_vendor(db, vendor_id, **fields)
        audit_record(db, username=current_username(), action="Update", record_type="Vendor", record_id=vendor_id)
        return _serialize_vendor(vendor)


def set_vendor_status(vendor_id: int, status: str) -> dict:
    with get_user_session() as db:
        vendor = repo.update_vendor(db, vendor_id, status=status, updated_by=current_username())
        audit_record(db, username=current_username(), action="Update", record_type="Vendor", record_id=vendor_id, new_value=status)
        return _serialize_vendor(vendor)


# ---------------------------------------------------------------- Customer

def _serialize_customer(customer) -> dict:
    return {
        "id": customer.id,
        "customer_code": customer.customer_code,
        "customer_name": customer.customer_name,
        "company_name": customer.company_name,
        "phone": customer.phone,
        "email": customer.email,
        "ntn": customer.ntn,
        "strn": customer.strn,
        "mailing_address": customer.mailing_address,
        "shipping_address": customer.shipping_address,
        "description": customer.description,
        "status": customer.status,
        "debit": customer.debit,
        "credit": customer.credit,
        "balance": customer.balance,
        "created_at": customer.created_at,
        "updated_at": customer.updated_at,
    }


def _validate_customer_fields(data: dict) -> dict:
    return {
        "customer_name": v.require_not_empty(data.get("customer_name"), "Customer Name"),
        "company_name": (data.get("company_name") or "").strip() or None,
        "phone": v.validate_phone(data.get("phone"), "Phone"),
        "email": v.validate_email(data.get("email"), "Email"),
        "ntn": (data.get("ntn") or "").strip() or None,
        "strn": (data.get("strn") or "").strip() or None,
        "mailing_address": (data.get("mailing_address") or "").strip() or None,
        "shipping_address": (data.get("shipping_address") or "").strip() or None,
        "description": (data.get("description") or "").strip() or None,
    }


def list_customers(status: str = "All") -> list[dict]:
    with get_user_session() as db:
        return [_serialize_customer(x) for x in repo.list_customers(db, status)]


def create_customer(data: dict) -> dict:
    fields = _validate_customer_fields(data)
    with get_user_session() as db:
        fields["customer_code"] = generate_number("customer")
        fields["created_by"] = fields["updated_by"] = current_username()
        customer = repo.create_customer(db, **fields)
        db.flush()
        audit_record(db, username=current_username(), action="Create", record_type="Customer", record_id=customer.id)
        return _serialize_customer(customer)


def update_customer(customer_id: int, data: dict) -> dict:
    fields = _validate_customer_fields(data)
    with get_user_session() as db:
        fields["updated_by"] = current_username()
        customer = repo.update_customer(db, customer_id, **fields)
        audit_record(db, username=current_username(), action="Update", record_type="Customer", record_id=customer_id)
        return _serialize_customer(customer)


def set_customer_status(customer_id: int, status: str) -> dict:
    with get_user_session() as db:
        customer = repo.update_customer(db, customer_id, status=status, updated_by=current_username())
        audit_record(db, username=current_username(), action="Update", record_type="Customer", record_id=customer_id, new_value=status)
        return _serialize_customer(customer)


# ---------------------------------------------------------------- Item

def _serialize_item(item) -> dict:
    return {
        "id": item.id,
        "item_code": item.item_code,
        "item_name": item.item_name,
        "item_type": item.item_type,
        "category": item.category,
        "sub_category": item.sub_category,
        "unit_of_measure": item.unit_of_measure,
        "purchase_price": item.purchase_price,
        "sale_price": item.sale_price,
        "description": item.description,
        "status": item.status,
        "is_stockable": item.is_stockable,
        "quantity_on_hand": item.quantity_on_hand,
        "debit": 0,
        "credit": 0,
        "balance": 0,
        "created_at": item.created_at,
        "updated_at": item.updated_at,
    }


def _validate_item_fields(data: dict) -> dict:
    item_type = v.validate_choice(data.get("item_type"), "Item Type", ITEM_TYPES)
    category = (data.get("category") or "").strip() or None
    sub_category = (data.get("sub_category") or "").strip() or None
    if category and category not in CATEGORY_MAP.get(item_type, ()):
        raise ValueError(f"Category '{category}' is not valid for item type '{item_type}'.")
    unit = data.get("unit_of_measure")
    if item_type != "Service":
        unit = v.validate_choice(unit, "Unit of Measure", UNITS_OF_MEASURE, required=True)
    else:
        unit = v.validate_choice(unit, "Unit of Measure", UNITS_OF_MEASURE, required=False)
    purchase_price = data.get("purchase_price") or 0
    sale_price = data.get("sale_price") or 0
    v.validate_non_negative(purchase_price, "Purchase Price")
    v.validate_non_negative(sale_price, "Sale Price")
    return {
        "item_name": v.require_not_empty(data.get("item_name"), "Item Name"),
        "item_type": item_type,
        "category": category,
        "sub_category": sub_category,
        "unit_of_measure": unit,
        "purchase_price": purchase_price,
        "sale_price": sale_price,
        "description": (data.get("description") or "").strip() or None,
    }


def list_items(status: str = "All") -> list[dict]:
    with get_user_session() as db:
        return [_serialize_item(x) for x in repo.list_items(db, status)]


def create_item(data: dict) -> dict:
    fields = _validate_item_fields(data)
    with get_user_session() as db:
        fields["item_code"] = generate_number("item")
        fields["created_by"] = fields["updated_by"] = current_username()
        item = repo.create_item(db, **fields)
        db.flush()
        audit_record(db, username=current_username(), action="Create", record_type="Item", record_id=item.id)
        return _serialize_item(item)


def update_item(item_id: int, data: dict) -> dict:
    fields = _validate_item_fields(data)
    with get_user_session() as db:
        fields["updated_by"] = current_username()
        item = repo.update_item(db, item_id, **fields)
        audit_record(db, username=current_username(), action="Update", record_type="Item", record_id=item_id)
        return _serialize_item(item)


def set_item_status(item_id: int, status: str) -> dict:
    with get_user_session() as db:
        item = repo.update_item(db, item_id, status=status, updated_by=current_username())
        audit_record(db, username=current_username(), action="Update", record_type="Item", record_id=item_id, new_value=status)
        return _serialize_item(item)


# ---------------------------------------------------------------- Chart of Accounts

def _serialize_account(account) -> dict:
    return {
        "id": account.id,
        "account_code": account.account_code,
        "account_name": account.account_name,
        "account_type": account.account_type,
        "parent_account_id": account.parent_account_id,
        "parent_account_name": account.parent.account_name if account.parent else None,
        "description": account.description,
        "status": account.status,
        "debit": account.debit,
        "credit": account.credit,
        "balance": account.balance,
        "created_at": account.created_at,
        "updated_at": account.updated_at,
    }


def _validate_account_fields(db, account_id: Optional[int], data: dict) -> dict:
    parent_id = data.get("parent_account_id") or None
    if parent_id is not None:
        parent_id = int(parent_id)
        if account_id is not None and parent_id == account_id:
            raise ValueError("An account cannot be its own parent.")
        if account_id is not None:
            # walk up the proposed parent chain to prevent a circular relationship
            current = repo.get_account(db, parent_id)
            visited = set()
            while current is not None:
                if current.id == account_id:
                    raise ValueError("Circular parent relationships are prohibited.")
                if current.id in visited:
                    break
                visited.add(current.id)
                current = current.parent
    return {
        "account_name": v.require_min_length(data.get("account_name"), "Account Name", 3),
        "account_type": v.validate_choice(data.get("account_type"), "Account Type", ACCOUNT_TYPES),
        "parent_account_id": parent_id,
        "description": (data.get("description") or "").strip() or None,
    }


def list_accounts(status: str = "All") -> list[dict]:
    with get_user_session() as db:
        return [_serialize_account(x) for x in repo.list_accounts(db, status)]


def list_cash_bank_accounts() -> list[dict]:
    """Asset accounts named like a cash/bank account, for the dashboard's
    Cash & Bank Accounts widget. There's no dedicated is_cash_or_bank flag
    on Account yet, so this is a name heuristic matching the seeded
    defaults ("Cash in Hand", "Bank Accounts") rather than a real account
    subtype — good enough until a Cash/Bank account-type field is added."""
    with get_user_session() as db:
        accounts = repo.list_accounts(db, "Active")
        return [
            _serialize_account(a)
            for a in accounts
            if a.account_type == "Asset" and ("cash" in a.account_name.lower() or "bank" in a.account_name.lower())
        ]


def create_account(data: dict) -> dict:
    with get_user_session() as db:
        fields = _validate_account_fields(db, None, data)
        fields["account_code"] = generate_number("account")
        fields["created_by"] = fields["updated_by"] = current_username()
        account = repo.create_account(db, **fields)
        db.flush()
        audit_record(db, username=current_username(), action="Create", record_type="Account", record_id=account.id)
        return _serialize_account(account)


def update_account(account_id: int, data: dict) -> dict:
    with get_user_session() as db:
        fields = _validate_account_fields(db, account_id, data)
        fields["updated_by"] = current_username()
        account = repo.update_account(db, account_id, **fields)
        audit_record(db, username=current_username(), action="Update", record_type="Account", record_id=account_id)
        return _serialize_account(account)


def set_account_status(account_id: int, status: str) -> dict:
    with get_user_session() as db:
        account = repo.update_account(db, account_id, status=status, updated_by=current_username())
        audit_record(db, username=current_username(), action="Update", record_type="Account", record_id=account_id, new_value=status)
        return _serialize_account(account)


# ---------------------------------------------------------------- Inventory Location

def _serialize_location(location) -> dict:
    return {
        "id": location.id,
        "location_code": location.location_code,
        "location_name": location.location_name,
        "location_type": location.location_type,
        "is_stockable": location.is_stockable,
        "contact_person": location.contact_person,
        "phone": location.phone,
        "email": location.email,
        "mailing_address": location.mailing_address,
        "shipping_address": location.shipping_address,
        "capacity": location.capacity,
        "description": location.description,
        "status": location.status,
        "debit": 0,
        "credit": 0,
        "balance": 0,
        "created_at": location.created_at,
        "updated_at": location.updated_at,
    }


def _validate_location_fields(data: dict) -> dict:
    location_type = v.validate_choice(data.get("location_type"), "Location Type", LOCATION_TYPES)
    capacity = data.get("capacity")
    if capacity in ("", None):
        capacity = None
    else:
        capacity = float(capacity)
        v.validate_non_negative(capacity, "Capacity")
    is_stockable = data.get("is_stockable")
    if is_stockable is None:
        is_stockable = location_type != "Office"
    return {
        "location_name": v.require_not_empty(data.get("location_name"), "Location Name"),
        "location_type": location_type,
        "is_stockable": bool(is_stockable),
        "contact_person": (data.get("contact_person") or "").strip() or None,
        "phone": v.validate_phone(data.get("phone"), "Phone"),
        "email": v.validate_email(data.get("email"), "Email"),
        "mailing_address": (data.get("mailing_address") or "").strip() or None,
        "shipping_address": (data.get("shipping_address") or "").strip() or None,
        "capacity": capacity,
        "description": (data.get("description") or "").strip() or None,
    }


def list_locations(status: str = "All") -> list[dict]:
    with get_user_session() as db:
        return [_serialize_location(x) for x in repo.list_locations(db, status)]


def create_location(data: dict) -> dict:
    fields = _validate_location_fields(data)
    with get_user_session() as db:
        fields["location_code"] = generate_number("location")
        fields["created_by"] = fields["updated_by"] = current_username()
        location = repo.create_location(db, **fields)
        db.flush()
        audit_record(db, username=current_username(), action="Create", record_type="InventoryLocation", record_id=location.id)
        return _serialize_location(location)


def update_location(location_id: int, data: dict) -> dict:
    fields = _validate_location_fields(data)
    with get_user_session() as db:
        fields["updated_by"] = current_username()
        location = repo.update_location(db, location_id, **fields)
        audit_record(db, username=current_username(), action="Update", record_type="InventoryLocation", record_id=location_id)
        return _serialize_location(location)


def set_location_status(location_id: int, status: str) -> dict:
    with get_user_session() as db:
        location = repo.update_location(db, location_id, status=status, updated_by=current_username())
        audit_record(db, username=current_username(), action="Update", record_type="InventoryLocation", record_id=location_id, new_value=status)
        return _serialize_location(location)


def location_stock(location_id: int) -> list[dict]:
    with get_user_session() as db:
        location = repo.get_location(db, location_id)
        if location is None:
            return []
        return location.stock_summary()
