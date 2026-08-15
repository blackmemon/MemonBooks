"""Light/dark palette + Theme objects — corporate navy SaaS palette per the
Dashboard Redesign brief. Default theme mode is SYSTEM (spec: 'the app
will automatically follow the user's device theme')."""
from __future__ import annotations
import flet as ft

# Primary brand color: deep navy, used for the sidebar/logo/headings.
PRIMARY = "#16305C"
PRIMARY_DARK = "#4A73B8"

# Bright blue accent: primary action buttons (Save/Post), links, active nav item.
ACCENT = "#2E6FF2"
ACCENT_DARK = "#5C8BFF"

CYAN = "#3BB4E5"
TEAL = "#1FA98C"
PINK = "#E64C6B"
ORANGE = "#F5A623"
GRAY = "#8A94A6"

SUCCESS = TEAL
WARNING = ORANGE
DANGER = PINK
INFO = CYAN

# Swatches offered for the (decorative, not yet wired to a live switcher)
# theme-color picker in Settings — spec section 7/8: "Theme color swatches
# shown at the bottom for palette customization."
THEME_SWATCHES = [PRIMARY, ACCENT, CYAN, TEAL, PINK, ORANGE, GRAY]

LIGHT_BG = "#F4F6FA"
LIGHT_SURFACE = "#FFFFFF"
LIGHT_BORDER = "#E5E9F2"
LIGHT_TEXT = "#1B2536"
LIGHT_TEXT_MUTED = "#6B7A99"

DARK_BG = "#0F1830"
DARK_SURFACE = "#16305C"
DARK_BORDER = "#2A407A"
DARK_TEXT = "#E7EBF5"
DARK_TEXT_MUTED = "#9DAAC7"

def light_theme() -> ft.Theme:
    return ft.Theme(
        color_scheme_seed=PRIMARY,
        font_family="Roboto",
        color_scheme=ft.ColorScheme(
            primary=PRIMARY,
            secondary=ACCENT,
            surface=LIGHT_SURFACE,
            on_surface=LIGHT_TEXT,
            outline=LIGHT_BORDER,
        ),
        scaffold_bgcolor=LIGHT_BG,
    )

def dark_theme() -> ft.Theme:
    return ft.Theme(
        color_scheme_seed=PRIMARY_DARK,
        font_family="Roboto",
        color_scheme=ft.ColorScheme(
            primary=PRIMARY_DARK,
            secondary=ACCENT_DARK,
            surface=DARK_SURFACE,
            on_surface=DARK_TEXT,
            outline=DARK_BORDER,
        ),
        scaffold_bgcolor=DARK_BG,
    )

def is_dark(page: ft.Page) -> bool:
    if page.theme_mode == ft.ThemeMode.DARK:
        return True
    if page.theme_mode == ft.ThemeMode.LIGHT:
        return False
    return page.platform_brightness == ft.Brightness.DARK

def surface(page: ft.Page) -> str:
    return DARK_SURFACE if is_dark(page) else LIGHT_SURFACE

def text_muted(page: ft.Page) -> str:
    return DARK_TEXT_MUTED if is_dark(page) else LIGHT_TEXT_MUTED

def border(page: ft.Page) -> str:
    return DARK_BORDER if is_dark(page) else LIGHT_BORDER

def accent(page: ft.Page) -> str:
    return ACCENT_DARK if is_dark(page) else ACCENT