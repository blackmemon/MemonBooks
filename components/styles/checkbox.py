"""Standardized checkbox."""
from __future__ import annotations

from typing import Callable, Optional

import flet as ft


def check(label: str, value: bool = False, on_change: Optional[Callable] = None) -> ft.Checkbox:
    return ft.Checkbox(label=label, value=value, on_change=on_change)
