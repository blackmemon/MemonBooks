"""Generates the MemonBooks placeholder app icon (an "MB" monogram on
the White Theme primary color, #51A2FF) and every raster asset the
packaging manifests reference: the Linux hicolor PNG set, a Windows
.ico plus the three MSIX PNG assets, and a macOS .icns.

Requires Pillow (`pip install pillow`) and a bold sans-serif TrueType
font -- defaults to Windows' arialbd.ttf; point FONT_PATH at another
bold sans-serif font on other platforms.

Drawn directly with Pillow rather than rasterized from icon.svg, since
that needs no SVG renderer (rsvg-convert/inkscape/cairosvg) -- keep
icon.svg's rect/text in sync by hand if this script's design changes.

Run from anywhere; paths below are relative to this file's location
(packaging/), so it always writes back into the packaging tree:

    python packaging/generate_icon.py
"""

import os
from PIL import Image, ImageDraw, ImageFont

PRIMARY = (81, 162, 255, 255)  # #51A2FF
WHITE = (255, 255, 255, 255)
FONT_PATH = r"C:\Windows\Fonts\arialbd.ttf"

PACKAGING_DIR = os.path.dirname(os.path.abspath(__file__))


def draw_icon(size: int) -> Image.Image:
    """Rounded square fill + centered "MB" monogram, matching
    icon.svg's layout (rounded rect radius ~18.75% of size, bold
    sans-serif text)."""
    scale = 8  # supersample then downscale for clean edges at small sizes
    big = size * scale
    img = Image.new("RGBA", (big, big), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)

    margin = big * (4 / 128)
    radius = big * (24 / 128)
    draw.rounded_rectangle([margin, margin, big - margin, big - margin], radius=radius, fill=PRIMARY)

    font_size = int(big * (46 / 128))
    font = ImageFont.truetype(FONT_PATH, font_size)
    text = "MB"
    bbox = draw.textbbox((0, 0), text, font=font)
    text_w = bbox[2] - bbox[0]
    text_h = bbox[3] - bbox[1]
    x = (big - text_w) / 2 - bbox[0]
    y = (big - text_h) / 2 - bbox[1]
    draw.text((x, y), text, font=font, fill=WHITE)

    return img.resize((size, size), Image.LANCZOS)


def main():
    master = draw_icon(1024)
    master.save(os.path.join(PACKAGING_DIR, "icon-master-1024.png"))

    # Linux hicolor icon theme sizes (freedesktop icon theme spec),
    # in the standard hicolor/<size>x<size>/apps/<name>.png layout.
    for s in [16, 32, 48, 64, 128, 256, 512]:
        d = os.path.join(PACKAGING_DIR, "linux", "icons", "hicolor", f"{s}x{s}", "apps")
        os.makedirs(d, exist_ok=True)
        draw_icon(s).save(os.path.join(d, "com.memonbooks.MemonBooks.png"))

    # Windows MSIX assets (exact sizes AppxManifest.xml references).
    windows_assets = os.path.join(PACKAGING_DIR, "windows", "Assets")
    os.makedirs(windows_assets, exist_ok=True)
    draw_icon(150).save(os.path.join(windows_assets, "Square150x150Logo.png"))
    draw_icon(44).save(os.path.join(windows_assets, "Square44x44Logo.png"))
    draw_icon(50).save(os.path.join(windows_assets, "StoreLogo.png"))

    # Windows .ico (multi-resolution, for embedding in the .exe itself).
    draw_icon(256).save(
        os.path.join(PACKAGING_DIR, "windows", "MemonBooks.ico"),
        sizes=[(s, s) for s in [16, 32, 48, 256]],
    )

    # macOS .icns (multi-resolution; Pillow's ICNS writer is pure
    # Python, no iconutil/macOS host required).
    draw_icon(1024).save(
        os.path.join(PACKAGING_DIR, "macos", "AppIcon.icns"),
        sizes=[(s, s) for s in [16, 32, 128, 256, 512, 1024]],
    )

    print("wrote icon assets under", PACKAGING_DIR)


if __name__ == "__main__":
    main()
