"""Mobile bottom navigation — reserved for the future mobile target (spec:
'later it will also be available as a web app and mobile app'). Not wired
into main.py's desktop shell yet."""
from __future__ import annotations

import flet as ft


def build_bottom_nav(page: ft.Page, selected_index: int = 0) -> ft.NavigationBar:
    return ft.NavigationBar(
        selected_index=selected_index,
        destinations=[
            ft.NavigationBarDestination(icon=ft.Icons.SPACE_DASHBOARD_OUTLINED, label="Dashboard"),
            ft.NavigationBarDestination(icon=ft.Icons.APPS_OUTLINED, label="Masters"),
            ft.NavigationBarDestination(icon=ft.Icons.SWAP_HORIZ_OUTLINED, label="Transactions"),
            ft.NavigationBarDestination(icon=ft.Icons.MORE_HORIZ, label="More"),
        ],
    )
