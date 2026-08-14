"""Generic 'coming in a future phase' placeholder for routes whose module
(HR/Payroll, Purchase, Sales, Cash/Bank/Journal, Reports) has DB models
defined but no UI yet in this pass."""
from __future__ import annotations

import flet as ft

from components.styles import colors
from components.styles.frames import card
from components.styles.labels import muted_text, page_title


def build(page: ft.Page, title: str) -> ft.Control:
    return ft.Column(
        [
            page_title(title),
            ft.Container(height=8),
            card(
                page,
                ft.Column(
                    [
                        ft.Icon(ft.Icons.CONSTRUCTION_OUTLINED, size=32, color=colors.text_muted(page)),
                        muted_text(f"{title} is coming in a future phase. Database models already exist; the UI is not built yet."),
                    ],
                    spacing=12,
                    horizontal_alignment=ft.CrossAxisAlignment.CENTER,
                ),
                padding=40,
            ),
        ],
        spacing=8,
        expand=True,
    )
