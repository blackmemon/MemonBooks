"""Combines header + sidebar + page content into the authenticated app shell."""
from __future__ import annotations

import flet as ft

from components.header import build_header
from components.sidebar import build_sidebar
from components.styles import colors


def build_shell(page: ft.Page, active_route: str, content: ft.Control) -> ft.View:
    return ft.View(
        route=active_route,
        padding=0,
        controls=[
            ft.Column(
                [
                    build_header(page),
                    ft.Row(
                        [
                            build_sidebar(page, active_route),
                            ft.Container(
                                content=ft.Column([content], scroll=ft.ScrollMode.AUTO, expand=True),
                                expand=True,
                                padding=24,
                                bgcolor=colors.DARK_BG if colors.is_dark(page) else colors.LIGHT_BG,
                            ),
                        ],
                        expand=True,
                        spacing=0,
                    ),
                ],
                expand=True,
                spacing=0,
            )
        ],
    )
