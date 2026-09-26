#!/usr/bin/env python3
"""Generate the README artwork in assets/readme/: the animated herd banner and feature icons.

The sheep use the same sprites and midnight-neon colors as the sidebar banner
(src/client/shell/herd_banner.rs), so the README shows what the app draws.

Usage: python3 scripts/readme_art.py
"""

from __future__ import annotations

from pathlib import Path

OUT = Path(__file__).resolve().parent.parent / "assets" / "readme"

NAVY = "#0b1020"
PANEL = "#121a33"
TEXT = "#d7def5"
MUTED = "#5b6690"
HEAD = "#7682ad"
LEGS = "#5b6690"
CYAN = "#37e2ff"  # working, focus
MAGENTA = "#ff4fd8"  # needs you
GREEN = "#7cf29a"  # done
IDLE = "#8b9ad6"  # idle

BODY = [".WWW...", "WWWWWHH", "WWWWWH."]
LEGS_APART = [".L..L..", ".L..L.."]
LEGS_TOGETHER = ["..LL...", "..LL..."]
SLEEPING = [".WWW...", "WWWWWH.", "WWWWWHH"]

GLYPHS = {
    "M": ["10001", "11011", "10101", "10101", "10001", "10001", "10001"],
    "o": ["00000", "00000", "01110", "10001", "10001", "10001", "01110"],
}


def rects(rows: list[str], x0: float, y0: float, px: float, colors: dict[str, str]) -> str:
    out = []
    for dy, row in enumerate(rows):
        for dx, ch in enumerate(row):
            if ch in colors:
                out.append(
                    f'<rect x="{x0 + dx * px:g}" y="{y0 + dy * px:g}" width="{px:g}" '
                    f'height="{px:g}" fill="{colors[ch]}"/>'
                )
    return "".join(out)


def sheep(x: float, y: float, px: float, wool: str, sleeping: bool = False) -> str:
    colors = {"W": wool, "H": HEAD, "L": LEGS}
    if sleeping:
        return rects(SLEEPING, x, y + px * 2, px, colors)
    body = rects(BODY, x, y, px, colors)
    apart = rects(LEGS_APART, x, y + px * 3, px, colors)
    together = rects(LEGS_TOGETHER, x, y + px * 3, px, colors)
    return f'{body}<g class="legs-a">{apart}</g><g class="legs-b">{together}</g>'


def title(x: float, y: float, px: float) -> str:
    out, cursor = [], x
    for i, letter in enumerate("MoMo"):
        fill = "url(#title)" if i % 2 == 0 else MAGENTA
        out.append(rects(GLYPHS[letter], cursor, y, px, {"1": fill}))
        cursor += (len(GLYPHS[letter][0]) + 1) * px
    return "".join(out)


def banner() -> str:
    width, height, px = 960, 330, 10
    ground_y = 300
    herd = [
        (0, CYAN, False),
        (110, CYAN, False),
        (220, MAGENTA, True),
        (330, GREEN, False),
    ]
    walkers = []
    for offset, wool, alert in herd:
        piece = sheep(offset, ground_y - 5 * px, px, wool)
        if alert:
            piece += (
                f'<g class="blink"><rect x="{offset + 3 * px}" y="{ground_y - 9 * px}" width="{px}" '
                f'height="{px * 2}" fill="{MAGENTA}"/><rect x="{offset + 3 * px}" '
                f'y="{ground_y - 6.5 * px}" width="{px}" height="{px}" fill="{MAGENTA}"/></g>'
            )
        walkers.append(piece)
    sleeper_x = 820
    stars = "".join(
        f'<rect x="{x}" y="{y}" width="2" height="2" fill="{TEXT}" opacity="{o}"/>'
        for x, y, o in [
            (40, 30, 0.5), (130, 70, 0.3), (240, 24, 0.6), (380, 52, 0.25), (520, 20, 0.45),
            (610, 64, 0.3), (700, 36, 0.55), (790, 18, 0.35), (880, 58, 0.5), (930, 110, 0.3),
            (60, 150, 0.25), (470, 120, 0.2), (660, 140, 0.25),
        ]
    )
    dots = "".join(
        f'<rect x="{x}" y="{ground_y + 6}" width="4" height="4" fill="{MUTED}"/>'
        for x in range(0, width * 2, 56)
    )
    return f"""<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}" role="img" aria-labelledby="t d">
<title id="t">MoMo</title>
<desc id="d">Pixel sheep walk across a night sky under the MoMo title. Cyan sheep are agents at work, a magenta sheep with a blinking exclamation mark needs you, a green sheep is done, and a sleeping sheep is idle.</desc>
<defs>
<linearGradient id="sky" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="{NAVY}"/><stop offset="1" stop-color="{PANEL}"/></linearGradient>
<linearGradient id="title" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="{CYAN}"/><stop offset="1" stop-color="#8f7dff"/></linearGradient>
</defs>
<style>
.herd {{ animation: walk 16s linear -9s infinite; }}
.legs-a {{ animation: step 0.5s steps(1) infinite; }}
.legs-b {{ animation: step 0.5s steps(1) infinite reverse; opacity: 0; }}
.blink {{ animation: blink 1s steps(1) infinite; }}
.ground {{ animation: scroll 4s linear infinite; }}
.zz {{ animation: float 3s ease-in infinite; }}
.zz2 {{ animation: float 3s ease-in 1.5s infinite; opacity: 0; }}
@keyframes walk {{ from {{ transform: translateX(-460px); }} to {{ transform: translateX({width}px); }} }}
@keyframes step {{ 0% {{ opacity: 1; }} 50% {{ opacity: 0; }} }}
@keyframes blink {{ 0% {{ opacity: 1; }} 50% {{ opacity: 0; }} }}
@keyframes scroll {{ from {{ transform: translateX(0); }} to {{ transform: translateX(-56px); }} }}
@keyframes float {{ 0% {{ opacity: 0; transform: translate(0, 0); }} 20% {{ opacity: 1; }} 100% {{ opacity: 0; transform: translate(14px, -34px); }} }}
@media (prefers-reduced-motion: reduce) {{ .herd, .legs-a, .legs-b, .blink, .ground, .zz, .zz2 {{ animation: none; }} .herd {{ transform: translateX(380px); }} }}
</style>
<rect width="{width}" height="{height}" rx="18" fill="url(#sky)"/>
{stars}
{title(56, 48, 12)}
<text x="58" y="172" fill="{TEXT}" font-family="ui-monospace, SFMono-Regular, Menlo, Consolas, monospace" font-size="20">a terminal runtime for your coding agents</text>
<text x="58" y="198" fill="{MUTED}" font-family="ui-monospace, SFMono-Regular, Menlo, Consolas, monospace" font-size="15">workspaces · tabs · panes · a herd that tells you who needs you</text>
<g class="ground">{dots}</g>
<g class="herd">{"".join(walkers)}</g>
{sheep(sleeper_x, ground_y - 5 * px, px, IDLE, sleeping=True)}
<text class="zz" x="{sleeper_x + 58}" y="{ground_y - 34}" fill="{IDLE}" font-family="ui-monospace, Menlo, monospace" font-size="16">z</text>
<text class="zz2" x="{sleeper_x + 58}" y="{ground_y - 34}" fill="{IDLE}" font-family="ui-monospace, Menlo, monospace" font-size="22">Z</text>
</svg>
"""


ICONS = {
    # 12x12 pixel icons; letters map to colors below.
    "herd": (
        ["............", "............", "...CCC......", "..CCCCCHH...", "..CCCCCH....",
         "...L..L.....", "...L..L..M..", ".........M..", "............", ".........M..",
         "............", "............"],
        {"C": CYAN, "H": HEAD, "L": LEGS, "M": MAGENTA},
    ),
    "sync": (
        ["............", ".CCCCC.CCCCC", ".C...C.C...C", ".C.K.C.C.K.C", ".C...C.C...C",
         ".CCCCC.CCCCC", "............", "...MMMMMMM..", "..M.......M.", ".MMM.....MMM",
         "............", "............"],
        {"C": CYAN, "K": TEXT, "M": MAGENTA},
    ),
    "stream": (
        ["............", ".GGGG.......", "......GGGGG.", ".GGGGGG.....", "........GGG.",
         ".GGG........", ".....GGGGGG.", ".GGGGG......", ".......GGGG.", ".GG.........",
         "......GGGGG.", "............"],
        {"G": GREEN},
    ),
    "shield": (
        ["............", "..CCCCCCCC..", "..C......C..", "..C..GG..C..", "..C.GGGG.C..",
         "..C..GG..C..", "..C..GG..C..", "...C....C...", "....C..C....", ".....CC.....",
         "............", "............"],
        {"C": CYAN, "G": GREEN},
    ),
    "branch": (
        ["............", "..M.........", "..M.....C...", "..M.....C...", "..M....C....",
         "..M...C.....", "..M..C......", "..MMM.......", "..M.........", "..M.........",
         "..M.........", "............"],
        {"M": MAGENTA, "C": CYAN},
    ),
    "bell": (
        ["............", ".....MM.....", "....MMMM....", "...MMMMMM...", "...MMMMMM...",
         "...MMMMMM...", "..MMMMMMMM..", "..MMMMMMMM..", "............", ".....MM.....",
         "............", "............"],
        {"M": MAGENTA},
    ),
}


def icon(rows: list[str], colors: dict[str, str]) -> str:
    px = 4
    size = len(rows) * px
    return (
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{size + 16}" height="{size + 16}" '
        f'viewBox="0 0 {size + 16} {size + 16}" aria-hidden="true">'
        f'<rect width="{size + 16}" height="{size + 16}" rx="12" fill="{PANEL}"/>'
        f"{rects(rows, 8, 8, px, colors)}</svg>\n"
    )


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    (OUT / "banner.svg").write_text(banner())
    for name, (rows, colors) in ICONS.items():
        (OUT / f"icon-{name}.svg").write_text(icon(rows, colors))
    print(f"wrote {OUT}")


if __name__ == "__main__":
    main()
