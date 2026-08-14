"""Flet app entry point — login -> dashboard -> Maintain Management pages.

Run `python scripts/generate_keys.py` then `python scripts/setup_db.py`
once before the first launch (spec section: Setup Sequence).
"""
from __future__ import annotations
import flet as ft

from config import APP_NAME
from components.app_layout import build_shell
from components.styles.colors import dark_theme, light_theme
from frontend.auth.login_page import build_view as build_login_view
from frontend.dashboard.dashboard_page import build as build_dashboard
from frontend.maintain_management import (
    coa_page,
    customer_page,
    inventory_location_page,
    item_page,
    vendor_page,
)
from frontend.placeholder_page import build as build_placeholder
from frontend.settings_management.settings_page import build as build_settings
from security.auth import current_session

ROUTES = {
    "/dashboard": build_dashboard,
    "/maintain/vendors": vendor_page.build,
    "/maintain/customers": customer_page.build,
    "/maintain/items": item_page.build,
    "/maintain/coa": coa_page.build,
    "/maintain/locations": inventory_location_page.build,
    "/settings": build_settings,
}

PLACEHOLDER_ROUTES = {
    "/hr/employees": "Employees",
    "/hr/payroll": "Payroll",
    "/hr/attendance": "Attendance",
    "/transactions/purchase": "Purchase",
    "/transactions/sale": "Sale",
    "/transactions/cash-bank-journal": "Cash / Bank / Journal",
    "/reports": "Reports",
}


def main(page: ft.Page) -> None:
    page.title = APP_NAME
    page.theme_mode = ft.ThemeMode.SYSTEM
    page.theme = light_theme()
    page.dark_theme = dark_theme()

    def route_change(e: ft.RouteChangeEvent) -> None:
        page.views.clear()
        route = page.route

        if route != "/login" and current_session() is None:
            page.views.append(build_login_view(page))
        elif route == "/login":
            page.views.append(build_login_view(page))
        elif route in ROUTES:
            page.views.append(build_shell(page, route, ROUTES[route](page)))
        elif route in PLACEHOLDER_ROUTES:
            page.views.append(build_shell(page, route, build_placeholder(page, PLACEHOLDER_ROUTES[route])))
        else:
            page.go("/dashboard" if current_session() else "/login")
            return
        page.update()

    page.on_route_change = route_change
    page.go("/login" if page.route == "/" else page.route)


if __name__ == "__main__":
    ft.run(main)
