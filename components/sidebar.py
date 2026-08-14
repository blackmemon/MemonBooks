"""Left navigation, grouped per spec section 17 (Maintain/HR/Transaction
Management), matching the reference screenshot's grouped-sidebar layout.
Redesign brief section 3: logo + hamburger collapse toggle at top."""
from __future__ import annotations

import flet as ft

from components.styles import colors

SIDEBAR_WIDTH = 248
SIDEBAR_COLLAPSED_WIDTH = 76

NAV_GROUPS: list[tuple[str, list[tuple[str, object, str]]]] = [
    ("", [("Dashboard", ft.Icons.SPACE_DASHBOARD_OUTLINED, "/dashboard")]),
    (
        "MAINTAIN MANAGEMENT",
        [
            ("Vendors", ft.Icons.LOCAL_SHIPPING_OUTLINED, "/maintain/vendors"),
            ("Customers", ft.Icons.PEOPLE_ALT_OUTLINED, "/maintain/customers"),
            ("Items / Stock", ft.Icons.INVENTORY_2_OUTLINED, "/maintain/items"),
            ("Chart of Accounts", ft.Icons.ACCOUNT_TREE_OUTLINED, "/maintain/coa"),
            ("Inventory Locations", ft.Icons.WAREHOUSE_OUTLINED, "/maintain/locations"),
        ],
    ),
    (
        "HR MANAGEMENT",
        [
            ("Employees", ft.Icons.BADGE_OUTLINED, "/hr/employees"),
            ("Payroll", ft.Icons.PAYMENTS_OUTLINED, "/hr/payroll"),
            ("Attendance", ft.Icons.EVENT_AVAILABLE_OUTLINED, "/hr/attendance"),
        ],
    ),
    (
        "TRANSACTION MANAGEMENT",
        [
            ("Purchase", ft.Icons.SHOPPING_CART_OUTLINED, "/transactions/purchase"),
            ("Sale", ft.Icons.POINT_OF_SALE_OUTLINED, "/transactions/sale"),
            ("Cash / Bank / Journal", ft.Icons.ACCOUNT_BALANCE_OUTLINED, "/transactions/cash-bank-journal"),
            ("Reports", ft.Icons.BAR_CHART_OUTLINED, "/reports"),
        ],
    ),
    ("", [("Settings", ft.Icons.SETTINGS_OUTLINED, "/settings")]),
]


def build_sidebar(page: ft.Page, active_route: str) -> ft.Container:
    collapsed = getattr(page, "sidebar_collapsed", False)

    def toggle_collapse(e):
        page.sidebar_collapsed = not collapsed
        # Same reasoning as the theme toggle: width/labels are baked into
        # these Containers at build time, so the route has to be rebuilt.
        page.go(page.route)

    top_row_children: list[ft.Control] = []
    if not collapsed:
        top_row_children.append(
            ft.Row(
                [ft.Icon(ft.Icons.RECEIPT_LONG, color=colors.PRIMARY), ft.Text("MemonBook ERP", size=18, weight=ft.FontWeight.BOLD)],
                spacing=8,
            )
        )
    top_row_children.append(ft.IconButton(icon=ft.Icons.MENU, icon_size=18, tooltip="Collapse menu" if not collapsed else "Expand menu", on_click=toggle_collapse))

    items: list[ft.Control] = [
        ft.Row(
            top_row_children,
            alignment=ft.MainAxisAlignment.SPACE_BETWEEN if not collapsed else ft.MainAxisAlignment.CENTER,
        ),
        ft.Container(height=12),
    ]

    for group_title, entries in NAV_GROUPS:
        if group_title and not collapsed:
            items.append(ft.Container(ft.Text(group_title, size=11, weight=ft.FontWeight.W_600, color=colors.text_muted(page)), padding=ft.Padding(8, 16, 8, 6)))
        elif group_title and collapsed:
            items.append(ft.Divider(height=16, color=colors.border(page)))
        for label, icon, route in entries:
            selected = active_route == route
            items.append(_nav_item(page, label, icon, route, selected, collapsed))

    return ft.Container(
        width=SIDEBAR_COLLAPSED_WIDTH if collapsed else SIDEBAR_WIDTH,
        bgcolor=colors.surface(page),
        border=ft.Border(right=ft.BorderSide(1, colors.border(page))),
        padding=ft.Padding(8 if collapsed else 12, 16, 8 if collapsed else 12, 16),
        content=ft.Column(items, spacing=2, scroll=ft.ScrollMode.AUTO),
        animate=ft.Animation(200, ft.AnimationCurve.EASE_OUT),
    )


def _nav_item(page: ft.Page, label: str, icon, route: str, selected: bool, collapsed: bool) -> ft.Container:
    def go(e):
        page.go(route)

    row_children: list[ft.Control] = [ft.Icon(icon, size=18, color=colors.PRIMARY if selected else colors.text_muted(page))]
    if not collapsed:
        row_children.append(ft.Text(label, color=colors.PRIMARY if selected else None, weight=ft.FontWeight.W_600 if selected else ft.FontWeight.NORMAL))

    return ft.Container(
        content=ft.Row(row_children, spacing=10, alignment=ft.MainAxisAlignment.CENTER if collapsed else ft.MainAxisAlignment.START),
        tooltip=label if collapsed else None,
        padding=ft.Padding(10, 9, 10, 9),
        border_radius=8,
        bgcolor=ft.Colors.with_opacity(0.10, colors.PRIMARY) if selected else None,
        on_click=go,
        ink=True,
    )
