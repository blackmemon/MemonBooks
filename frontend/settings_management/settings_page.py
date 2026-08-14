"""Settings — minimal stub (theme + app info). Full settings scope is not
part of this pass's Maintain Management vertical slice."""
from __future__ import annotations

import flet as ft

from config import APP_NAME, APP_VERSION
from components.styles.frames import card
from components.styles.labels import muted_text, page_title, section_title
from security.auth import current_session


def build(page: ft.Page) -> ft.Control:
    session = current_session()
    theme_choices = ft.RadioGroup(
        content=ft.Row(
            [
                ft.Radio(value="system", label="System Default"),
                ft.Radio(value="light", label="Light"),
                ft.Radio(value="dark", label="Dark"),
            ]
        ),
        value=page.theme_mode.value if page.theme_mode else "system",
        on_change=lambda e: _set_theme(page, e.control.value),
    )

    info = card(
        page,
        ft.Column(
            [
                section_title("Appearance"),
                theme_choices,
                ft.Divider(),
                section_title("About"),
                muted_text(f"{APP_NAME} v{APP_VERSION}"),
                muted_text(f"Signed in as {session.full_name} ({session.role})") if session else ft.Container(),
            ],
            spacing=12,
        ),
    )

    return ft.Column([page_title("Settings"), ft.Container(height=8), info], spacing=8, expand=True)


def _set_theme(page: ft.Page, value: str) -> None:
    page.theme_mode = {"light": ft.ThemeMode.LIGHT, "dark": ft.ThemeMode.DARK}.get(value, ft.ThemeMode.SYSTEM)
    page.go(page.route)
