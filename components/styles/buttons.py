"""Standardized buttons so pages don't hand-roll Button styling."""
from __future__ import annotations

from typing import Callable, Optional

import flet as ft

from components.styles.colors import ACCENT


def primary_button(label: str, on_click: Optional[Callable] = None, icon=None, disabled: bool = False) -> ft.Button:
    return ft.Button(label, icon=icon, on_click=on_click, disabled=disabled, bgcolor=ACCENT, color=ft.Colors.WHITE)


def secondary_button(label: str, on_click: Optional[Callable] = None, icon=None, disabled: bool = False) -> ft.OutlinedButton:
    return ft.OutlinedButton(label, icon=icon, on_click=on_click, disabled=disabled)


def text_action(label: str, on_click: Optional[Callable] = None) -> ft.TextButton:
    return ft.TextButton(label, on_click=on_click)


def icon_action(icon, on_click: Optional[Callable] = None, tooltip: Optional[str] = None) -> ft.IconButton:
    return ft.IconButton(icon=icon, on_click=on_click, tooltip=tooltip)
