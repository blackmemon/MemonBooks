"""Top bar: search, date, theme toggle, admin menu — spec's reference layout."""
from __future__ import annotations

import datetime as dt

import flet as ft

from components.styles import colors
from security.auth import current_session, logout


def build_header(page: ft.Page) -> ft.Container:
    session = current_session()
    full_name = session.full_name if session else "Admin"

    def toggle_theme(e):
        order = [ft.ThemeMode.SYSTEM, ft.ThemeMode.LIGHT, ft.ThemeMode.DARK]
        current = page.theme_mode if page.theme_mode in order else ft.ThemeMode.SYSTEM
        page.theme_mode = order[(order.index(current) + 1) % len(order)]
        # Every surface/border/text color in this app is computed at build
        # time from colors.is_dark(page) and baked into each Container as a
        # plain hex value (not a live theme token), so a plain page.update()
        # would leave them stuck on the old theme — the whole route has to
        # be rebuilt so those colors get recomputed against the new mode.
        page.go(page.route)

    def do_logout(e):
        logout()
        page.go("/login")

    theme_icon = {
        ft.ThemeMode.LIGHT: ft.Icons.LIGHT_MODE_OUTLINED,
        ft.ThemeMode.DARK: ft.Icons.DARK_MODE_OUTLINED,
    }.get(page.theme_mode, ft.Icons.BRIGHTNESS_AUTO_OUTLINED)

    return ft.Container(
        bgcolor=colors.surface(page),
        border=ft.Border(bottom=ft.BorderSide(1, colors.border(page))),
        padding=ft.Padding(20, 12, 20, 12),
        content=ft.Row(
            [
                ft.TextField(
                    hint_text="Search menu...",
                    prefix_icon=ft.Icons.SEARCH,
                    border_radius=8,
                    width=280,
                    height=42,
                    content_padding=ft.Padding(12, 8, 12, 8),
                ),
                ft.Row(
                    [
                        ft.Text(dt.date.today().strftime("%d %b %Y"), color=colors.text_muted(page)),
                        ft.IconButton(icon=theme_icon, tooltip="Toggle theme", on_click=toggle_theme),
                        ft.IconButton(icon=ft.Icons.NOTIFICATIONS_NONE, tooltip="Notifications"),
                        ft.IconButton(icon=ft.Icons.HELP_OUTLINE, tooltip="Help"),
                        ft.PopupMenuButton(
                            content=ft.Row(
                                [
                                    ft.CircleAvatar(content=ft.Text(full_name[:1].upper()), bgcolor=colors.PRIMARY, color=ft.Colors.WHITE, radius=16),
                                    ft.Text(full_name, weight=ft.FontWeight.W_500),
                                    ft.Icon(ft.Icons.EXPAND_MORE, size=18),
                                ],
                                spacing=8,
                            ),
                            items=[ft.PopupMenuItem(content=ft.Text("Logout"), on_click=do_logout)],
                        ),
                    ],
                    spacing=8,
                ),
            ],
            alignment=ft.MainAxisAlignment.SPACE_BETWEEN,
        ),
    )
