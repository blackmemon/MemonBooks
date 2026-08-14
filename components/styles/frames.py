"""Standardized card/panel container — soft drop shadow, rounded corners,
per the Dashboard Redesign brief (section 1/8)."""
from __future__ import annotations

from typing import Optional

import flet as ft

from components.styles import colors


def _card_shadow(page: ft.Page) -> ft.BoxShadow:
    opacity = 0.35 if colors.is_dark(page) else 0.06
    return ft.BoxShadow(
        spread_radius=0,
        blur_radius=16,
        color=ft.Colors.with_opacity(opacity, ft.Colors.BLACK),
        offset=ft.Offset(0, 4),
    )


def card(page: ft.Page, content: ft.Control, padding: int = 16) -> ft.Container:
    return ft.Container(
        content=content,
        padding=padding,
        border_radius=12,
        bgcolor=colors.surface(page),
        border=ft.Border.all(1, colors.border(page)),
        shadow=_card_shadow(page),
    )


def stat_tile(
    page: ft.Page,
    title: str,
    value: str,
    icon=None,
    delta: Optional[str] = None,
    delta_up: bool = True,
) -> ft.Container:
    from components.styles.labels import muted_text, stat_value

    header_row: list[ft.Control] = []
    if icon:
        header_row.append(
            ft.Container(
                content=ft.Icon(icon, size=18, color=colors.PRIMARY),
                bgcolor=ft.Colors.with_opacity(0.10, colors.PRIMARY),
                border_radius=8,
                padding=8,
            )
        )
    header_row.append(muted_text(title))

    children: list[ft.Control] = [ft.Row(header_row, spacing=8), stat_value(value)]
    if delta:
        delta_color = colors.SUCCESS if delta_up else colors.DANGER
        children.append(
            ft.Row(
                [
                    ft.Icon(ft.Icons.ARROW_UPWARD if delta_up else ft.Icons.ARROW_DOWNWARD, size=12, color=delta_color),
                    ft.Text(delta, size=12, color=delta_color, weight=ft.FontWeight.W_600),
                ],
                spacing=2,
            )
        )
    return card(page, ft.Column(children, spacing=6, tight=True), padding=18)
