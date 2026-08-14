"""Chart of Accounts — spec section 3.4, including the Table/Tree view toggle."""
from __future__ import annotations

import flet as ft

from backend.controllers import maintain_controller as controller
from backend.models.account_models import ACCOUNT_TYPES
from components.styles import colors
from components.styles.combobox import combo, id_combo
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
    state = {"status": "All", "view": "table"}
    body_container = ft.Container()
    error_text = ft.Text("", color=ft.Colors.RED)

    def refresh():
        rows = controller.list_accounts(state["status"])
        body_container.content = _build_tree(page, rows) if state["view"] == "tree" else _build_table(rows)
        page.update()

    def open_form(existing: dict | None):
        all_accounts = controller.list_accounts("All")
        parent_choices = [("", "(none)")] + [
            (str(a["id"]), f"{a['account_code']} - {a['account_name']}") for a in all_accounts if not existing or a["id"] != existing["id"]
        ]
        name_f = text_field("Account Name *", existing["account_name"] if existing else "")
        type_f = combo("Account Type *", ACCOUNT_TYPES, value=existing["account_type"] if existing else "Asset")
        parent_f = id_combo("Parent Account", parent_choices, value=str(existing["parent_account_id"]) if existing and existing["parent_account_id"] else "")
        desc_f = text_field("Description", existing["description"] if existing else "", multiline=True)

        def save(e):
            data = {
                "account_name": name_f.value,
                "account_type": type_f.value,
                "parent_account_id": int(parent_f.value) if parent_f.value else None,
                "description": desc_f.value,
            }
            try:
                if existing:
                    controller.update_account(existing["id"], data)
                else:
                    controller.create_account(data)
                page.pop_dialog()
                refresh()
            except ValueError as ex:
                error_text.value = str(ex)
                page.update()

        error_text.value = ""
        dialog = form_dialog(
            page,
            "Edit Account" if existing else "Add Account",
            [name_f, type_f, parent_f, desc_f],
            save,
            error_text,
        )
        page.show_dialog(dialog)

    def on_add(e):
        open_form(None)

    def on_filter_change(e):
        state["status"] = e.control.value
        refresh()

    def on_view_change(e):
        state["view"] = e.control.selected[0] if e.control.selected else "table"
        refresh()

    def _build_table(rows: list[dict]) -> ft.DataTable:
        table_rows = []
        for i, r in enumerate(rows, start=1):
            def edit_handler(e, row=r):
                open_form(row)

            def toggle_handler(e, row=r):
                controller.set_account_status(row["id"], "Inactive" if row["status"] == "Active" else "Active")
                refresh()

            table_rows.append(
                [
                    i,
                    r["account_code"],
                    r["account_name"],
                    r["account_type"],
                    r["parent_account_name"] or "",
                    format_currency(r["debit"]),
                    format_currency(r["credit"]),
                    format_currency(r["balance"]),
                    status_badge(r["status"]),
                    row_actions(edit_handler, toggle_handler, r["status"] == "Active"),
                ]
            )
        return build_table(
            ["S.No", "Code", "Account Name", "Type", "Parent Account", "Debit", "Credit", "Balance", "Status", "Actions"],
            table_rows,
        )

    view_toggle = ft.SegmentedButton(
        selected=[state["view"]],
        segments=[
            ft.Segment(value="table", label=ft.Text("Table"), icon=ft.Icon(ft.Icons.TABLE_ROWS_OUTLINED)),
            ft.Segment(value="tree", label=ft.Text("Tree"), icon=ft.Icon(ft.Icons.ACCOUNT_TREE_OUTLINED)),
        ],
        on_change=on_view_change,
    )

    header = ft.Row(
        [
            list_header("Chart of Accounts", on_add, "Add Account", status_filter_dropdown(state["status"], on_filter_change)),
        ],
    )
    refresh()
    return ft.Column([header, view_toggle, body_container], spacing=16, expand=True)


def _build_tree(page: ft.Page, rows: list[dict]) -> ft.Control:
    by_parent: dict[int | None, list[dict]] = {}
    by_id = {r["id"]: r for r in rows}
    for r in rows:
        by_parent.setdefault(r["parent_account_id"], []).append(r)

    def render(parent_id, depth=0) -> list[ft.Control]:
        out: list[ft.Control] = []
        for r in sorted(by_parent.get(parent_id, []), key=lambda x: x["account_code"]):
            out.append(
                ft.Container(
                    padding=ft.Padding(depth * 24, 6, 0, 6),
                    content=ft.Row(
                        [
                            ft.Icon(ft.Icons.SUBDIRECTORY_ARROW_RIGHT, size=14, color=colors.text_muted(page)) if depth else ft.Container(width=14),
                            ft.Text(r["account_code"], color=colors.text_muted(page), width=110),
                            ft.Text(r["account_name"], weight=ft.FontWeight.W_500, expand=True),
                            ft.Text(r["account_type"], color=colors.text_muted(page), width=90),
                            ft.Text(format_currency(r["balance"]), width=140),
                            status_badge(r["status"]),
                        ]
                    ),
                )
            )
            out.extend(render(r["id"], depth + 1))
        return out

    return ft.Column(render(None), spacing=0, scroll=ft.ScrollMode.AUTO)
