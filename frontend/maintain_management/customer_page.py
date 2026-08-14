"""Customer Maintenance — spec section 3.2."""
from __future__ import annotations

import flet as ft

from backend.controllers import maintain_controller as controller
from components.styles.table import build_table
from components.styles.textbox import text_field
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
        rows = controller.list_customers(state["status"])
        table_container.content = _build_table(rows)
        page.update()

    def open_form(existing: dict | None):
        name_f = text_field("Customer Name *", existing["customer_name"] if existing else "")
        company_f = text_field("Company Name", existing["company_name"] if existing else "")
        phone_f = text_field("Phone", existing["phone"] if existing else "")
        email_f = text_field("Email", existing["email"] if existing else "")
        ntn_f = text_field("NTN", existing["ntn"] if existing else "")
        strn_f = text_field("STRN", existing["strn"] if existing else "")
        mailing_f = text_field("Mailing Address", existing["mailing_address"] if existing else "", multiline=True)
        shipping_f = text_field("Shipping Address", existing["shipping_address"] if existing else "", multiline=True)
        desc_f = text_field("Description", existing["description"] if existing else "", multiline=True)

        def save(e):
            data = {
                "customer_name": name_f.value,
                "company_name": company_f.value,
                "phone": phone_f.value,
                "email": email_f.value,
                "ntn": ntn_f.value,
                "strn": strn_f.value,
                "mailing_address": mailing_f.value,
                "shipping_address": shipping_f.value,
                "description": desc_f.value,
            }
            try:
                if existing:
                    controller.update_customer(existing["id"], data)
                else:
                    controller.create_customer(data)
                page.pop_dialog()
                refresh()
            except ValueError as ex:
                error_text.value = str(ex)
                page.update()

        error_text.value = ""
        dialog = form_dialog(
            page,
            "Edit Customer" if existing else "Add Customer",
            [name_f, company_f, phone_f, email_f, ntn_f, strn_f, mailing_f, shipping_f, desc_f],
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
                controller.set_customer_status(row["id"], "Inactive" if row["status"] == "Active" else "Active")
                refresh()

            table_rows.append(
                [
                    i,
                    r["customer_code"],
                    r["customer_name"],
                    r["company_name"] or "",
                    r["phone"] or "",
                    r["email"] or "",
                    r["ntn"] or "",
                    format_currency(r["debit"]),
                    format_currency(r["credit"]),
                    format_currency(r["balance"]),
                    status_badge(r["status"]),
                    row_actions(edit_handler, toggle_handler, r["status"] == "Active"),
                ]
            )
        return build_table(
            ["S.No", "Customer ID", "Customer Name", "Company", "Phone", "Email", "NTN", "Debit", "Credit", "Balance", "Status", "Actions"],
            table_rows,
        )

    header = list_header("Customers", on_add, "Add Customer", status_filter_dropdown(state["status"], on_filter_change))
    refresh()
    return ft.Column([header, table_container], spacing=16, expand=True)
