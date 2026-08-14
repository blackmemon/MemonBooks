"""Vendor Maintenance — spec section 3.1.

The list table shows the core identifying/business/accounting columns
(S.No, Vendor ID, Name, Company, Type, Phone, Address, Payment Method,
Debit/Credit/Balance, Status); the remaining spec-listed columns (Email,
Website, Shipping Address, Payment Terms, Description, timestamps,
Custom Fields, Attachments) live in the add/edit form so the table stays
readable instead of scrolling 20 columns wide.
"""
from __future__ import annotations

import flet as ft

from backend.controllers import maintain_controller as controller
from backend.models.account_models import PAYMENT_METHODS, VENDOR_TYPES
from components.styles.combobox import combo
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
        rows = controller.list_vendors(state["status"])
        table_container.content = _build_table(rows)
        page.update()

    def open_form(existing: dict | None):
        name_f = text_field("Vendor Name *", existing["vendor_name"] if existing else "")
        company_f = text_field("Company Name *", existing["company_name"] if existing else "")
        type_f = combo("Vendor Type *", VENDOR_TYPES, value=existing["vendor_type"] if existing else "Supplier")
        phone_f = text_field("Phone *", existing["phone"] if existing else "")
        email_f = text_field("Email", existing["email"] if existing else "")
        website_f = text_field("Website", existing["website"] if existing else "")
        mailing_f = text_field("Mailing Address *", existing["mailing_address"] if existing else "", multiline=True)
        shipping_f = text_field("Shipping Address", existing["shipping_address"] if existing else "", multiline=True)
        terms_f = text_field("Payment Terms", existing["payment_terms"] if existing else "")
        method_f = combo("Payment Method *", PAYMENT_METHODS, value=existing["payment_method"] if existing else "Cash")
        desc_f = text_field("Description", existing["description"] if existing else "", multiline=True)

        def save(e):
            data = {
                "vendor_name": name_f.value,
                "company_name": company_f.value,
                "vendor_type": type_f.value,
                "phone": phone_f.value,
                "email": email_f.value,
                "website": website_f.value,
                "mailing_address": mailing_f.value,
                "shipping_address": shipping_f.value,
                "payment_terms": terms_f.value,
                "payment_method": method_f.value,
                "description": desc_f.value,
            }
            try:
                if existing:
                    controller.update_vendor(existing["id"], data)
                else:
                    controller.create_vendor(data)
                page.pop_dialog()
                refresh()
            except ValueError as ex:
                error_text.value = str(ex)
                page.update()

        error_text.value = ""
        dialog = form_dialog(
            page,
            "Edit Vendor" if existing else "Add Vendor",
            [name_f, company_f, type_f, phone_f, email_f, website_f, mailing_f, shipping_f, terms_f, method_f, desc_f],
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
                controller.set_vendor_status(row["id"], "Inactive" if row["status"] == "Active" else "Active")
                refresh()

            table_rows.append(
                [
                    i,
                    r["vendor_code"],
                    r["vendor_name"],
                    r["company_name"],
                    r["vendor_type"],
                    r["phone"],
                    r["mailing_address"],
                    r["payment_method"],
                    format_currency(r["debit"]),
                    format_currency(r["credit"]),
                    format_currency(r["balance"]),
                    status_badge(r["status"]),
                    row_actions(edit_handler, toggle_handler, r["status"] == "Active"),
                ]
            )
        return build_table(
            ["S.No", "Vendor ID", "Vendor Name", "Company", "Type", "Phone", "Mailing Address", "Payment Method", "Debit", "Credit", "Balance", "Status", "Actions"],
            table_rows,
        )

    header = list_header("Vendors", on_add, "Add Vendor", status_filter_dropdown(state["status"], on_filter_change))
    refresh()
    return ft.Column([header, table_container], spacing=16, expand=True)
