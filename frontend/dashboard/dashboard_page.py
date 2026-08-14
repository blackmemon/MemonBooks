"""Dashboard — static placeholder cards + a live Cash & Bank widget sourced
from the Chart of Accounts. Sales/Purchases/Profit figures need the
Transaction Management module (out of scope for this pass; spec section
8.8-8.10), so those stay at zero rather than showing fabricated numbers."""
from __future__ import annotations

import flet as ft

from backend.controllers import maintain_controller as controller
from components.styles import colors
from components.styles.buttons import text_action
from components.styles.frames import card, stat_tile
from components.styles.labels import muted_text, page_title, section_title
from security.auth import current_session
from utils.helpers import format_currency


def build(page: ft.Page) -> ft.Control:
    session = current_session()
    greeting = f"Welcome back, {session.full_name}!" if session else "Welcome back!"

    stats = ft.Row(
        [
            stat_tile(page, "Total Sales (This Month)", format_currency(0), icon=ft.Icons.POINT_OF_SALE_OUTLINED),
            stat_tile(page, "Total Purchases (This Month)", format_currency(0), icon=ft.Icons.SHOPPING_CART_OUTLINED),
            stat_tile(page, "Gross Profit (This Month)", format_currency(0), icon=ft.Icons.TRENDING_UP),
            stat_tile(page, "Total Receivables", format_currency(0), icon=ft.Icons.ACCOUNT_BALANCE_WALLET_OUTLINED),
            stat_tile(page, "Total Payables", format_currency(0), icon=ft.Icons.PAYMENTS_OUTLINED),
        ],
        wrap=True,
        spacing=16,
    )

    chart_placeholder = card(
        page,
        ft.Column(
            [
                section_title("Sales vs Purchases"),
                ft.Container(
                    height=200,
                    alignment=ft.Alignment(0, 0),
                    content=ft.Text(
                        "No transactions posted yet — this chart will populate once "
                        "the Purchase and Sales modules are implemented.",
                        color=colors.text_muted(page),
                        text_align=ft.TextAlign.CENTER,
                    ),
                ),
            ]
        ),
        padding=20,
    )

    cash_bank_widget = card(page, _build_cash_bank_section(page), padding=20)

    return ft.Column(
        [
            ft.Row([page_title(greeting)], alignment=ft.MainAxisAlignment.START),
            ft.Text("Here's what's happening with your business today.", color=colors.text_muted(page)),
            ft.Container(height=8),
            stats,
            ft.Container(height=8),
            ft.Row(
                [
                    ft.Container(content=chart_placeholder, expand=2),
                    ft.Container(content=cash_bank_widget, expand=1),
                ],
                spacing=16,
                vertical_alignment=ft.CrossAxisAlignment.START,
            ),
        ],
        spacing=8,
        expand=True,
    )


def _build_cash_bank_section(page: ft.Page) -> ft.Column:
    accounts = controller.list_cash_bank_accounts()

    def go_to_coa(e):
        page.go("/maintain/coa")

    rows: list[ft.Control] = []
    if accounts:
        for a in accounts:
            rows.append(
                ft.Row(
                    [ft.Text(a["account_name"]), ft.Text(format_currency(a["balance"]), weight=ft.FontWeight.W_600)],
                    alignment=ft.MainAxisAlignment.SPACE_BETWEEN,
                )
            )
    else:
        rows.append(muted_text("No cash/bank accounts yet."))

    return ft.Column(
        [
            ft.Row(
                [section_title("Cash & Bank Accounts"), text_action("View All", on_click=go_to_coa)],
                alignment=ft.MainAxisAlignment.SPACE_BETWEEN,
            ),
            ft.Column(rows, spacing=10),
        ],
        spacing=12,
    )
