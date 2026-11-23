#!/usr/bin/env python3
"""Builds the KernTeX logo as pure-vector SVGs (glyph outlines, no fonts needed).

    python3 assets/logo/build_logo.py

Writes assets/logo/logo.svg (full: K, lowered T and a small "eX" under the T's
arm) and assets/logo/logo-small.svg (K + T only, for 16-32 px icons).
Glyphs come from Latin Modern Roman Bold (the classic TeX typeface); requires
fontTools and the lmodern OTF fonts (TeX Live: fonts-lmodern / lm package).
"""

from pathlib import Path
from fontTools.ttLib import TTFont
from fontTools.pens.svgPathPen import SVGPathPen

HERE = Path(__file__).resolve().parent
FONT_CANDIDATES = [
    "/usr/share/texmf/fonts/opentype/public/lm/lmroman10-bold.otf",
    "/usr/share/texlive/texmf-dist/fonts/opentype/public/lm/lmroman10-bold.otf",
]

NAVY = "#1f2a5c"
CREAM = "#fbf7ef"
GOLD = "#ffc94d"

# Layout in a 512×512 box (designed by eye). Each variant gets its own offset
# so its ink is optically centred: the full logo is wider because of "eX".
OFFSET_FULL = (-35, 6)
OFFSET_SMALL = (0, 6)
K = dict(char="K", x=200, y=312, size=300, fill=CREAM)
T = dict(char="T", x=335, y=394, size=300, fill=GOLD)
EX = dict(text="eX", x=431, y=338, size=88, fill=GOLD)


def load_font():
    for p in FONT_CANDIDATES:
        if Path(p).is_file():
            return TTFont(p)
    raise SystemExit("Latin Modern Roman Bold (lmroman10-bold.otf) not found; install the lmodern fonts.")


def glyph_path(font, char, x0, baseline, size):
    """SVG <path> for one glyph whose advance box starts at x0."""
    gs = font.getGlyphSet()
    name = font.getBestCmap()[ord(char)]
    pen = SVGPathPen(gs)
    gs[name].draw(pen)
    s = size / font["head"].unitsPerEm
    return f'<path transform="translate({x0:.2f} {baseline}) scale({s:.5f} {-s:.5f})" d="{pen.getCommands()}"/>'


def advance(font, char, size):
    name = font.getBestCmap()[ord(char)]
    return font["hmtx"][name][0] * size / font["head"].unitsPerEm


def centered_text(font, text, cx, baseline, size, fill):
    """Like SVG text-anchor="middle": centre the run of advances on cx."""
    total = sum(advance(font, c, size) for c in text)
    x = cx - total / 2
    parts = []
    for c in text:
        parts.append(glyph_path(font, c, x, baseline, size))
        x += advance(font, c, size)
    return f'<g fill="{fill}">{"".join(parts)}</g>'


def svg(font, with_ex):
    items = [
        centered_text(font, K["char"], K["x"], K["y"], K["size"], K["fill"]),
        centered_text(font, T["char"], T["x"], T["y"], T["size"], T["fill"]),
    ]
    off = OFFSET_FULL if with_ex else OFFSET_SMALL
    if with_ex:
        items.append(centered_text(font, EX["text"], EX["x"], EX["y"], EX["size"], EX["fill"]))
    return (
        '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" width="512" height="512">\n'
        f'  <rect width="512" height="512" rx="112" fill="{NAVY}"/>\n'
        f'  <g transform="translate({off[0]} {off[1]})">{"".join(items)}</g>\n'
        "</svg>\n"
    )


def main():
    font = load_font()
    (HERE / "logo.svg").write_text(svg(font, True))
    (HERE / "logo-small.svg").write_text(svg(font, False))
    print("wrote", HERE / "logo.svg", "and", HERE / "logo-small.svg")


if __name__ == "__main__":
    main()
