"""Inventory Location — spec section 3.5."""
from __future__ import annotations

import flet as ft

from backend.controllers import maintain_controller as controller
from backend.models.inventory_models import LOCATION_TYPES
from components.styles.checkbox import check
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
from frontend.maintain_management.location_stock_dialog import open_location_stock_dialog


def build(page: ft.Page) -> ft.Control:
    state = {"status": "All"}
    table_container = ft.Container()
    error_text = ft.Text("", color=ft.Colors.RED)

    def refresh():
        rows = controller.list_locations(state["status"])
        table_container.content = _build_table(rows)
        page.update()

    def open_form(existing: dict | None):
        initial_type = existing["location_type"] if existing else "Warehouse"
        name_f = text_field("Location Name *", existing["location_name"] if existing else "")
        type_f = combo("Location Type *", LOCATION_TYPES, value=initial_type)
        stockable_f = check("Stockable location", existing["is_stockable"] if existing else True)
        contact_f = text_field("Contact Person", existing["contact_person"] if existing else "")
        phone_f = text_field("Phone", existing["phone"] if existing else "")
        email_f = text_field("Email", existing["email"] if existing else "")
        mailing_f = text_field("Mailing Address", existing["mailing_address"] if existing else "", multiline=True)
        shipping_f = text_field("Shipping Address", existing["shipping_address"] if existing else "", multiline=True)
        capacity_f = number_field("Capacity", str(existing["capacity"]) if existing and existing["capacity"] is not None else "")
        desc_f = text_field("Description", existing["description"] if existing else "", multiline=True)

        def save(e):
            data = {
                "location_name": name_f.value,
                "location_type": type_f.value,
                "is_stockable": stockable_f.value,
                "contact_person": contact_f.value,
                "phone": phone_f.value,
                "email": email_f.value,
                "mailing_address": mailing_f.value,
                "shipping_address": shipping_f.value,
                "capacity": capacity_f.value,
                "description": desc_f.value,
            }
            try:
                if existing:
                    controller.update_location(existing["id"], data)
                else:
                    controller.create_location(data)
                page.pop_dialog()
                refresh()
            except ValueError as ex:
                error_text.value = str(ex)
                page.update()

        error_text.value = ""
        dialog = form_dialog(
            page,
            "Edit Location" if existing else "Add Location",
            [name_f, type_f, stockable_f, contact_f, phone_f, email_f, mailing_f, shipping_f, capacity_f, desc_f],
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
                controller.set_location_status(row["id"], "Inactive" if row["status"] == "Active" else "Active")
                refresh()

            def view_stock_handler(e, row=r):
                open_location_stock_dialog(page, row["id"], row["location_name"])

            table_rows.append(
                [
                    i,
                    r["location_code"],
                    r["location_name"],
                    r["location_type"],
                    "Yes" if r["is_stockable"] else "No",
                    r["contact_person"] or "",
                    r["phone"] or "",
                    r["capacity"] if r["capacity"] is not None else "",
                    status_badge(r["status"]),
                    row_actions(
                        edit_handler,
                        toggle_handler,
                        r["status"] == "Active",
                        extra=[ft.IconButton(icon=ft.Icons.INVENTORY_OUTLINED, tooltip="View Stock", on_click=view_stock_handler)]
                        if r["is_stockable"]
                        else None,
                    ),
                ]
            )
        return build_table(
            ["S.No", "Location ID", "Location Name", "Type", "Stockable", "Contact", "Phone", "Capacity", "Status", "Actions"],
            table_rows,
        )

    header = list_header("Inventory Locations", on_add, "Add Location", status_filter_dropdown(state["status"], on_filter_change))
    refresh()
    return ft.Column([header, table_container], spacing=16, expand=True)
