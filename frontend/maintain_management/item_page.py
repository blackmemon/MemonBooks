"""Item Maintenance — spec section 3.3."""
from __future__ import annotations

import flet as ft

from backend.controllers import maintain_controller as controller
from backend.models.inventory_models import CATEGORY_MAP, ITEM_TYPES, UNITS_OF_MEASURE
from components.styles.combobox import combo
from components.styles.table import build_table
from components.styles.textbox import number_field, text_field
from utils.helpers import format_currency
from frontend.maintain_management._common import (
    form_dialog,
    list_header,
    row_actions,
    status_badge,
    status_filter_dropdown,
)


def build(page: ft.Page) -> ft.Control:
    state = {"status": "All"}
    table_container = ft.Container()
    error_text = ft.Text("", color=ft.Colors.RED)

    def refresh():
        rows = controller.list_items(state["status"])
        table_container.content = _build_table(rows)
        page.update()

    def open_form(existing: dict | None):
        name_f = text_field("Item Name *", existing["item_name"] if existing else "")
        initial_type = existing["item_type"] if existing else "Finished Goods"
        category_f = combo("Category", CATEGORY_MAP.get(initial_type, ()), value=existing["category"] if existing else None)

        def on_type_change(e):
            category_f.options = [ft.dropdown.Option(c) for c in CATEGORY_MAP.get(type_f.value, ())]
            category_f.value = None
            page.update()

        type_f = combo("Item Type *", ITEM_TYPES, value=initial_type, on_change=on_type_change)
        sub_category_f = text_field("Sub-Category", existing["sub_category"] if existing else "")
        unit_f = combo("Unit of Measure", UNITS_OF_MEASURE, value=existing["unit_of_measure"] if existing else "Pcs")
        purchase_price_f = number_field("Purchase Price", str(existing["purchase_price"]) if existing else "0")
        sale_price_f = number_field("Sale Price", str(existing["sale_price"]) if existing else "0")
        desc_f = text_field("Description", existing["description"] if existing else "", multiline=True)

        def save(e):
            data = {
                "item_name": name_f.value,
                "item_type": type_f.value,
                "category": category_f.value,
                "sub_category": sub_category_f.value,
                "unit_of_measure": unit_f.value,
                "purchase_price": _to_float(purchase_price_f.value),
                "sale_price": _to_float(sale_price_f.value),
                "description": desc_f.value,
            }
            try:
                if existing:
                    controller.update_item(existing["id"], data)
                else:
                    controller.create_item(data)
                page.pop_dialog()
                refresh()
            except ValueError as ex:
                error_text.value = str(ex)
                page.update()

        error_text.value = ""
        dialog = form_dialog(
            page,
            "Edit Item" if existing else "Add Item",
            [name_f, type_f, category_f, sub_category_f, unit_f, purchase_price_f, sale_price_f, desc_f],
            save,
            error_text,
        )
        page.show_dialog(dialog)

    def on_add(e):
        open_form(None)

    def on_filter_change(e):
        state["status"] = e.control.value
        refresh()

    def _build_table(rows: list[dict]) -> ft.DataTable:
        table_rows = []
        for i, r in enumerate(rows, start=1):
            def edit_handler(e, row=r):
                open_form(row)

            def toggle_handler(e, row=r):
                controller.set_item_status(row["id"], "Inactive" if row["status"] == "Active" else "Active")
                refresh()

            table_rows.append(
                [
                    i,
                    r["item_code"],
                    r["item_name"],
                    r["item_type"],
                    r["category"] or "",
                    r["unit_of_measure"] or "",
                    format_currency(r["purchase_price"]),
                    format_currency(r["sale_price"]),
                    str(r["quantity_on_hand"]) if r["is_stockable"] else "—",
                    status_badge(r["status"]),
                    row_actions(edit_handler, toggle_handler, r["status"] == "Active"),
                ]
            )
        return build_table(
            ["S.No", "Item Code", "Item Name", "Type", "Category", "Unit", "Purchase Price", "Sale Price", "Qty on Hand", "Status", "Actions"],
            table_rows,
        )

    header = list_header("Items / Stock", on_add, "Add Item", status_filter_dropdown(state["status"], on_filter_change))
    refresh()
    return ft.Column([header, table_container], spacing=16, expand=True)


def _to_float(value) -> float:
    try:
        return float(value)
    except (TypeError, ValueError):
        return 0.0
