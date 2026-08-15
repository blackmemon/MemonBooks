"""Staff login screen."""
from __future__ import annotations
import flet as ft
from components.styles import colors
from components.styles.buttons import primary_button
from components.styles.textbox import text_field
from database.database import open_user_database
from security import auth

def build_view(page: ft.Page) -> ft.View:
    username_f = text_field("Username")
    password_f = text_field("Password", password=True)
    error_text = ft.Text("", color=ft.Colors.RED)

    def do_login(e):
        try:
            open_user_database()
            session = auth.login(username_f.value, password_f.value)
        except Exception as ex:
            error_text.value = f"{ex}"
            page.update()
            return
        if session is None:
            error_text.value = "Invalid username or password."
            page.update()
            return
        page.go("/dashboard")

    password_f.on_submit = do_login

    return ft.View(
        route="/login",
        controls=[
            ft.Container(
                expand=True,
                alignment=ft.Alignment(0, 0),
                bgcolor=colors.DARK_BG if colors.is_dark(page) else colors.LIGHT_BG,
                content=ft.Container(
                    width=360,
                    padding=32,
                    border_radius=16,
                    bgcolor=colors.surface(page),
                    border=ft.Border.all(1, colors.border(page)),
                    content=ft.Column(
                        [
                            ft.Icon(ft.Icons.RECEIPT_LONG, size=40, color=colors.PRIMARY),
                            ft.Text("MemonBook ERP", size=22, weight=ft.FontWeight.BOLD),
                            ft.Text("Sign in to continue", color=colors.text_muted(page)),
                            username_f,
                            password_f,
                            error_text,
                            primary_button("Sign In", on_click=do_login),
                        ],
                        horizontal_alignment=ft.CrossAxisAlignment.CENTER,
                        spacing=16,
                    ),
                ),
            )
        ],
    )
