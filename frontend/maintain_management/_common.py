"""Shared list-page scaffolding for the five Maintain Management pages —
status filter, status badge, row action buttons — so each page file only
has to define its own fields/columns instead of re-deriving this shell.
"""
from __future__ import annotations

from typing import Callable, Optional

import flet as ft

from components.styles import colors
from components.styles.buttons import icon_action, primary_button
from components.styles.combobox import combo
from components.styles.labels import page_title
from backend.models.mixins import MASTER_STATUSES

STATUS_FILTER_CHOICES = ("All",) + MASTER_STATUSES


def status_filter_dropdown(value: str, on_change: Callable) -> ft.Dropdown:
    return combo("Status", STATUS_FILTER_CHOICES, value=value, on_change=on_change, width=160)


def status_badge(status: str) -> ft.Container:
    active = status == "Active"
    color = colors.SUCCESS if active else ft.Colors.OUTLINE
    return ft.Container(
        content=ft.Text(status, size=11, color=color, weight=ft.FontWeight.W_600),
        bgcolor=ft.Colors.with_opacity(0.12, color),
        padding=ft.Padding(8, 3, 8, 3),
        border_radius=20,
    )


def row_actions(on_edit: Callable, on_toggle: Callable, is_active: bool, extra: Optional[list[ft.Control]] = None) -> ft.Row:
    controls = [
        icon_action(ft.Icons.EDIT_OUTLINED, on_click=on_edit, tooltip="Edit"),
        icon_action(
            ft.Icons.TOGGLE_OFF_OUTLINED if is_active else ft.Icons.TOGGLE_ON_OUTLINED,
            on_click=on_toggle,
            tooltip="Deactivate" if is_active else "Activate",
        ),
    ]
    if extra:
        controls.extend(extra)
    return ft.Row(controls, spacing=0)


def list_header(title: str, on_add: Callable, add_label: str, filter_control: ft.Control) -> ft.Row:
    return ft.Row(
        [
            page_title(title),
            ft.Row([filter_control, primary_button(add_label, on_click=on_add, icon=ft.Icons.ADD)], spacing=12),
        ],
        alignment=ft.MainAxisAlignment.SPACE_BETWEEN,
    )


def form_dialog(page: ft.Page, title: str, fields: list[ft.Control], on_save: Callable, error_text: ft.Text) -> ft.AlertDialog:
    def close(e):
        page.pop_dialog()

    return ft.AlertDialog(
        modal=True,
        title=ft.Text(title),
        content=ft.Container(
            content=ft.Column(fields + [error_text], tight=True, scroll=ft.ScrollMode.AUTO, spacing=12),
            width=480,
            height=440,
        ),
        actions=[ft.TextButton("Cancel", on_click=close), primary_button("Save", on_click=on_save)],
        actions_alignment=ft.MainAxisAlignment.END,
    )
