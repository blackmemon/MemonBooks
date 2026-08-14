"""Standardized DataTable builder shared by every master-list page."""
from __future__ import annotations

from typing import Sequence

import flet as ft


def build_table(columns: Sequence[str], rows: Sequence[Sequence]) -> ft.DataTable:
    return ft.DataTable(
        columns=[ft.DataColumn(ft.Text(c, weight=ft.FontWeight.W_600)) for c in columns],
        rows=[
            ft.DataRow(cells=[ft.DataCell(cell if isinstance(cell, ft.Control) else ft.Text(str(cell))) for cell in row])
            for row in rows
        ],
        heading_row_height=42,
        data_row_min_height=44,
        column_spacing=24,
        horizontal_lines=ft.BorderSide(1, ft.Colors.OUTLINE_VARIANT),
    )
