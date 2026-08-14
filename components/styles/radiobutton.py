"""Standardized radio group — used for Active/Inactive style choices, Yes/No fields."""
from __future__ import annotations

from typing import Callable, Optional, Sequence

import flet as ft


def radio_group(choices: Sequence[str], value: Optional[str] = None, on_change: Optional[Callable] = None) -> ft.RadioGroup:
    return ft.RadioGroup(
        content=ft.Row([ft.Radio(value=c, label=c) for c in choices]),
        value=value,
        on_change=on_change,
    )
