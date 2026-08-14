"""Central document-numbering service — spec section 2.3.

Every auto-generated ID/document number in the system must come through
here rather than being built ad hoc inside a page or controller, so the
PREFIX-YYYYMMDD-HHMMSS format (and future changes to it) live in one place.
"""
from __future__ import annotations

import datetime as dt
import time

PREFIXES = {
    "vendor": "VEN",
    "customer": "CUS",
    "item": "PRD",
    "account": "COA",
    "location": "LOC",
    "employee": "EMP",
    "po": "PO",
    "grn": "GRN",
    "pi": "PI",
    "pr": "PR",
    "quotation": "QT",
    "so": "SO",
    "dc": "DC",
    "si": "SI",
    "sr": "SR",
    "receipt": "RCPT",
    "payment": "PAY",
    "jv": "JV",
}

_last_stamp = ""
_sequence = 0


def generate_number(kind: str) -> str:
    """Returns PREFIX-YYYYMMDD-HHMMSS, guaranteed unique even when called
    multiple times within the same second."""
    global _last_stamp, _sequence
    prefix = PREFIXES[kind]
    now = dt.datetime.now()
    stamp = now.strftime("%Y%m%d-%H%M%S")
    if stamp == _last_stamp:
        _sequence += 1
        stamp = f"{stamp}{_sequence}"
    else:
        _last_stamp = stamp
        _sequence = 0
    return f"{prefix}-{stamp}"
