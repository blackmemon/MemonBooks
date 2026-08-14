"""Shared field validation — spec section 12. Raises ValueError with a
user-friendly message; controllers catch these and surface them to the UI."""
from __future__ import annotations

import re

EMAIL_RE = re.compile(r"^[^@\s]+@[^@\s]+\.[^@\s]+$")
PHONE_RE = re.compile(r"^[0-9+\-\s()]{6,20}$")


def require_min_length(value: str | None, field: str, min_length: int) -> str:
    v = (value or "").strip()
    if len(v) < min_length:
        raise ValueError(f"{field} must be at least {min_length} characters.")
    return v


def require_not_empty(value: str | None, field: str) -> str:
    v = (value or "").strip()
    if not v:
        raise ValueError(f"{field} is required.")
    return v


def validate_email(value: str | None, field: str = "Email", required: bool = False) -> str | None:
    v = (value or "").strip()
    if not v:
        if required:
            raise ValueError(f"{field} is required.")
        return None
    if not EMAIL_RE.match(v):
        raise ValueError(f"{field} is not a valid email address.")
    return v


def validate_phone(value: str | None, field: str = "Phone", required: bool = False) -> str | None:
    v = (value or "").strip()
    if not v:
        if required:
            raise ValueError(f"{field} is required.")
        return None
    if not PHONE_RE.match(v):
        raise ValueError(f"{field} is not a valid phone number.")
    return v


def validate_non_negative(value, field: str):
    if value is not None and value < 0:
        raise ValueError(f"{field} cannot be negative.")
    return value


def validate_choice(value: str | None, field: str, choices: tuple[str, ...], required: bool = True) -> str | None:
    v = (value or "").strip()
    if not v:
        if required:
            raise ValueError(f"{field} is required.")
        return None
    if v not in choices:
        raise ValueError(f"{field} must be one of: {', '.join(choices)}.")
    return v


def validate_date_order(start, end, start_field: str, end_field: str) -> None:
    if start is not None and end is not None and end < start:
        raise ValueError(f"{end_field} cannot be earlier than {start_field}.")
