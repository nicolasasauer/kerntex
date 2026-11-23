#!/usr/bin/env python3
"""Writes src-tauri/icons/icon.ico with the simplified logo for 16-32 px and the
full logo for 48 px and up, and the small art for the 32x32.png Linux icon."""
from pathlib import Path
from PIL import Image

root = Path(__file__).resolve().parents[2]
png = root / "assets/logo/png"
small = {s: Image.open(png / f"small-{s}.png").convert("RGBA") for s in (16, 24, 32)}
big = Image.open(png / "full-256.png").convert("RGBA")
sizes = [(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)]
extra = [small[16], small[24], small[32]] + [big.resize(s, Image.LANCZOS) for s in sizes[3:6]]
big.save(root / "src-tauri/icons/icon.ico", format="ICO", sizes=sizes, append_images=extra)
small[32].save(root / "src-tauri/icons/32x32.png")
print("icon.ico:", Image.open(root / "src-tauri/icons/icon.ico").info.get("sizes"))
