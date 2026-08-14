"""Typography helpers for consistent section/field labeling."""
from __future__ import annotations

import flet as ft

from components.styles.colors import PRIMARY


def page_title(text: str) -> ft.Text:
    return ft.Text(text, size=22, weight=ft.FontWeight.BOLD)


def section_title(text: str) -> ft.Text:
    return ft.Text(text, size=16, weight=ft.FontWeight.W_600)


def field_label(text: str) -> ft.Text:
    return ft.Text(text, size=12, weight=ft.FontWeight.W_500)


def muted_text(text: str) -> ft.Text:
    return ft.Text(text, size=12, color=ft.Colors.OUTLINE)


def stat_value(text: str) -> ft.Text:
    return ft.Text(text, size=24, weight=ft.FontWeight.BOLD, color=PRIMARY)
