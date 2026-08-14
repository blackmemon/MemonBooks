"""Standardized text input so every form field looks the same."""
from __future__ import annotations

from typing import Callable, Optional

import flet as ft


def text_field(
    label: str,
    value: str = "",
    hint_text: Optional[str] = None,
    multiline: bool = False,
    password: bool = False,
    on_change: Optional[Callable] = None,
    width: Optional[int] = None,
    read_only: bool = False,
    expand: bool = False,
) -> ft.TextField:
    return ft.TextField(
        label=label,
        value=value,
        hint_text=hint_text,
        multiline=multiline,
        password=password,
        can_reveal_password=password,
        on_change=on_change,
        width=width,
        read_only=read_only,
        expand=expand,
        border_radius=8,
    )


def number_field(label: str, value: str = "0", on_change: Optional[Callable] = None, width: Optional[int] = None) -> ft.TextField:
    return ft.TextField(
        label=label,
        value=value,
        on_change=on_change,
        width=width,
        keyboard_type=ft.KeyboardType.NUMBER,
        border_radius=8,
    )
