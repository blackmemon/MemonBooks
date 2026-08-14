"""Read-only stock view for a location — spec section 3.5.

Reserved Quantity is omitted (spec: "if reservation is implemented" — no
reservation system exists yet), so Available Quantity always equals
Quantity On Hand for now.
"""
from __future__ import annotations

import flet as ft

from backend.controllers import maintain_controller as controller
from components.styles.table import build_table
from utils.helpers import format_date


def open_location_stock_dialog(page: ft.Page, location_id: int, location_name: str) -> None:
    rows = controller.location_stock(location_id)

    if rows:
        table = build_table(
            ["Item Code", "Item Name", "Unit", "Qty on Hand", "Available Qty", "Last Movement"],
            [
                [
                    r["item_code"],
                    r["item_name"],
                    r["unit"] or "",
                    str(r["quantity_on_hand"]),
                    str(r["quantity_on_hand"]),
                    format_date(r["last_movement_date"]),
                ]
                for r in rows
            ],
        )
        content: ft.Control = ft.Container(content=table, width=560, height=360)
    else:
        content = ft.Container(
            content=ft.Text("No stock movements recorded for this location yet.", italic=True),
            width=560,
            padding=24,
        )

    def close(e):
        page.pop_dialog()

    dialog = ft.AlertDialog(
        modal=True,
        title=ft.Text(f"Stock at {location_name}"),
        content=content,
        actions=[ft.TextButton("Close", on_click=close)],
        actions_alignment=ft.MainAxisAlignment.END,
    )
    page.show_dialog(dialog)
