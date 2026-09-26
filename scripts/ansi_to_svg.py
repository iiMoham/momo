#!/usr/bin/env python3
"""Render a captured terminal screen (ANSI SGR text) as an SVG screenshot.

Capture a screen with `momo pane read <pane> --format ansi --lines N > shot.ansi`, then:

    python3 scripts/ansi_to_svg.py shot.ansi assets/readme/shot.svg --title "MoMo"

Block characters (half blocks and full blocks) are drawn as rectangles so pixel art stays crisp.
"""

from __future__ import annotations

import argparse
import html
import re
from pathlib import Path

BG = "#0b1020"
FG = "#d7def5"
CELL_W = 8.4
CELL_H = 18
FONT = 14
PAD = 16
BAR = 34

BASIC = [
    "#1c2340", "#ff5c8a", "#7cf29a", "#ffd166", "#37a8ff", "#c38bff", "#37e2ff", "#d7def5",
    "#5b6690", "#ff7aa2", "#9cf7b4", "#ffe08a", "#6cc0ff", "#d6aaff", "#7eeeff", "#ffffff",
]
SGR = re.compile(r"\x1b\[([0-9;]*)m")
OTHER_ESC = re.compile(r"\x1b\[[0-9;?]*[A-Za-z]|\x1b\][^\x07]*\x07")


def xterm256(n: int) -> str:
    if n < 16:
        return BASIC[n]
    if n < 232:
        n -= 16
        levels = [0, 95, 135, 175, 215, 255]
        return "#{:02x}{:02x}{:02x}".format(levels[n // 36], levels[(n // 6) % 6], levels[n % 6])
    v = 8 + (n - 232) * 10
    return f"#{v:02x}{v:02x}{v:02x}"


class Style:
    def __init__(self) -> None:
        self.fg: str | None = None
        self.bg: str | None = None
        self.bold = self.dim = self.italic = self.underline = self.reverse = False

    def copy(self) -> "Style":
        s = Style()
        s.__dict__.update(self.__dict__)
        return s

    def apply(self, params: list[int]) -> None:
        i = 0
        while i < len(params):
            p = params[i]
            if p == 0:
                self.__init__()
            elif p == 1:
                self.bold = True
            elif p == 2:
                self.dim = True
            elif p == 22:
                self.bold = self.dim = False
            elif p == 3:
                self.italic = True
            elif p == 23:
                self.italic = False
            elif p == 4:
                self.underline = True
            elif p == 24:
                self.underline = False
            elif p == 7:
                self.reverse = True
            elif p == 27:
                self.reverse = False
            elif 30 <= p <= 37:
                self.fg = BASIC[p - 30]
            elif 90 <= p <= 97:
                self.fg = BASIC[p - 90 + 8]
            elif 40 <= p <= 47:
                self.bg = BASIC[p - 40]
            elif 100 <= p <= 107:
                self.bg = BASIC[p - 100 + 8]
            elif p == 39:
                self.fg = None
            elif p == 49:
                self.bg = None
            elif p in (38, 48) and i + 1 < len(params):
                if params[i + 1] == 2 and i + 4 < len(params):
                    color = "#{:02x}{:02x}{:02x}".format(*params[i + 2 : i + 5])
                    i += 4
                elif params[i + 1] == 5 and i + 2 < len(params):
                    color = xterm256(params[i + 2])
                    i += 2
                else:
                    color = None
                if p == 38:
                    self.fg = color
                else:
                    self.bg = color
            i += 1

    def colors(self) -> tuple[str, str | None]:
        fg, bg = self.fg or FG, self.bg
        if self.reverse:
            fg, bg = (bg or BG), fg
        return fg, bg


def parse(text: str) -> list[list[tuple[str, Style]]]:
    rows = []
    style = Style()
    for line in text.rstrip("\n").split("\n"):
        cells: list[tuple[str, Style]] = []
        pos = 0
        line = OTHER_ESC.sub(lambda m: m.group(0) if m.group(0).endswith("m") else "", line)
        for match in SGR.finditer(line):
            for ch in line[pos : match.start()]:
                cells.append((ch, style.copy()))
            params = [int(p) if p else 0 for p in match.group(1).split(";")]
            style.apply(params or [0])
            pos = match.end()
        for ch in line[pos:]:
            cells.append((ch, style.copy()))
        rows.append(cells)
    return rows


def render(rows: list[list[tuple[str, Style]]], title: str) -> str:
    cols = max((len(r) for r in rows), default=0)
    width = cols * CELL_W + PAD * 2
    height = len(rows) * CELL_H + PAD * 2 + BAR
    parts = [
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{width:.0f}" height="{height:.0f}" '
        f'viewBox="0 0 {width:.0f} {height:.0f}" role="img" aria-label="{html.escape(title)}">',
        f'<rect width="100%" height="100%" rx="12" fill="{BG}"/>',
        f'<rect width="100%" height="{BAR}" rx="12" fill="#161f3d"/>',
        f'<rect y="{BAR - 12}" width="100%" height="12" fill="#161f3d"/>',
        '<circle cx="20" cy="17" r="6" fill="#ff5f57"/><circle cx="40" cy="17" r="6" fill="#febc2e"/>'
        '<circle cx="60" cy="17" r="6" fill="#28c840"/>',
        f'<text x="{width / 2:.0f}" y="22" fill="#8a95c0" font-family="ui-sans-serif, -apple-system, '
        f'Segoe UI, Helvetica, Arial, sans-serif" font-size="13" text-anchor="middle">{html.escape(title)}</text>',
        f'<g font-family="ui-monospace, SFMono-Regular, Menlo, Consolas, monospace" font-size="{FONT}" '
        f'xml:space="preserve">',
    ]
    for y, row in enumerate(rows):
        top = PAD + BAR + y * CELL_H
        # backgrounds, merged into runs
        x = 0
        while x < len(row):
            _, bg = row[x][1].colors()
            end = x
            while end + 1 < len(row) and row[end + 1][1].colors()[1] == bg:
                end += 1
            if bg:
                parts.append(
                    f'<rect x="{PAD + x * CELL_W:.1f}" y="{top}" width="{(end - x + 1) * CELL_W + 0.4:.1f}" '
                    f'height="{CELL_H}" fill="{bg}"/>'
                )
            x = end + 1
        # glyphs
        x = 0
        while x < len(row):
            ch, style = row[x]
            fg, _ = style.colors()
            left = PAD + x * CELL_W
            if ch in "▀▄█":
                h = CELL_H / 2
                if ch in "▀█":
                    parts.append(f'<rect x="{left:.1f}" y="{top}" width="{CELL_W + 0.4:.1f}" height="{h + 0.4:.1f}" fill="{fg}"/>')
                if ch in "▄█":
                    parts.append(f'<rect x="{left:.1f}" y="{top + h:.1f}" width="{CELL_W + 0.4:.1f}" height="{h:.1f}" fill="{fg}"/>')
                x += 1
                continue
            end = x
            while (
                end + 1 < len(row)
                and row[end + 1][0] not in "▀▄█"
                and row[end + 1][1].colors()[0] == fg
                and (row[end + 1][1].bold, row[end + 1][1].dim) == (style.bold, style.dim)
            ):
                end += 1
            text = "".join(c for c, _ in row[x : end + 1])
            lead = len(text) - len(text.lstrip(" "))
            text = text.strip(" ")
            left += lead * CELL_W
            if text:
                attrs = ""
                if style.bold:
                    attrs += ' font-weight="700"'
                if style.dim:
                    attrs += ' opacity="0.6"'
                if style.italic:
                    attrs += ' font-style="italic"'
                # Non-breaking spaces keep inner runs of spaces; textLength only adjusts
                # spacing so the text lines up with the cell grid in any monospace font.
                parts.append(
                    f'<text x="{left:.1f}" y="{top + CELL_H - 5}" fill="{fg}" '
                    f'textLength="{len(text) * CELL_W:.1f}" lengthAdjust="spacing"{attrs}>'
                    f"{html.escape(text).replace(' ', '&#160;')}</text>"
                )
            x = end + 1
    parts.append("</g></svg>\n")
    return "".join(parts)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("input", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--title", default="MoMo")
    args = parser.parse_args()
    rows = parse(args.input.read_text())
    while rows and not "".join(c for c, _ in rows[-1]).strip():
        rows.pop()
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(render(rows, args.title))
    print(f"wrote {args.output} ({len(rows)} rows)")


if __name__ == "__main__":
    main()
