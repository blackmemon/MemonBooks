"""Small formatting helpers shared across pages."""
from __future__ import annotations

import datetime as dt
from decimal import Decimal

from config import CURRENCY


def format_currency(amount) -> str:
    value = Decimal(amount or 0)
    return f"{CURRENCY} {value:,.2f}"


def format_date(value: dt.date | dt.datetime | None) -> str:
    if value is None:
        return ""
    return value.strftime("%d/%m/%Y")


def parse_date(value: str | None) -> dt.date | None:
    if not value:
        return None
    return dt.datetime.strptime(value.strip(), "%Y-%m-%d").date()
