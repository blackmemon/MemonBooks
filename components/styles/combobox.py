"""Standardized dropdown/select."""
from __future__ import annotations

from typing import Callable, Optional, Sequence

import flet as ft


def combo(
    label: str,
    choices: Sequence[str],
    value: Optional[str] = None,
    on_change: Optional[Callable] = None,
    width: Optional[int] = None,
) -> ft.Dropdown:
    return ft.Dropdown(
        label=label,
        options=[ft.dropdown.Option(c) for c in choices],
        value=value,
        on_select=on_change,
        width=width,
        border_radius=8,
    )


def id_combo(
    label: str,
    id_label_pairs: Sequence[tuple[str, str]],
    value: Optional[str] = None,
    on_change: Optional[Callable] = None,
    width: Optional[int] = None,
) -> ft.Dropdown:
    """Dropdown keyed by an id string (e.g. primary key) with a human label."""
    return ft.Dropdown(
        label=label,
        options=[ft.dropdown.Option(key=i, text=l) for i, l in id_label_pairs],
        value=value,
        on_select=on_change,
        width=width,
        border_radius=8,
    )
