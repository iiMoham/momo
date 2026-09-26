# 04 indexed-colors

Source: upstream discussion #4514 "support ansi color codes in color config" (references
upstream PR #4512 as a prototype; do not copy code from it without reading it critically).

## Problem

Theme colors accept hex, named colors, `rgb(r,g,b)` and reset aliases. Users who want MoMo's UI to
follow their terminal's palette can only choose the whole `terminal` theme; they cannot point one
override at, say, palette slot 4 or 208.

## Behavior

Accept palette indexes anywhere `parse_color` is used:

```toml
[theme.custom]
accent = "ansi:4"     # palette index 0-255
panel_bg = "ansi:236"
```

- Syntax: `ansi:<0-255>`. Decide in the plan whether a bare integer string (`"4"`) is also accepted;
  recommended: no, to avoid ambiguity with future numeric forms.
- Out-of-range or malformed values follow the existing invalid-color fallback and produce the
  normal startup/reload diagnostic.
- Sidebar token `fg` fields are strict `#RGB`/`#RRGGBB` today (`SidebarTokenColor`). Extending them is
  out of scope unless the plan argues for it; if extended, keep serialization round-trips exact.

## Classification

Config parsing + TUI rendering. Client-local.

## Code map (at d11c0c34)

- `src/config/theme.rs::parse_color` (around line 155) returns `ratatui::style::Color`; add a branch
  producing `Color::Indexed(n)`.
- Check every place that converts theme colors into RGB (for example contrast calculations such as
  `contrast(p)` in overlays, theme sync, or host-terminal palette queries in
  `src/app/theme_sync.rs` / `src/terminal_theme.rs`). Indexed colors have no intrinsic RGB; decide
  and document the fallback (use the host palette if known, else a fixed xterm-256 table).
- Validation diagnostics: find where invalid colors are reported (`rg "invalid color"`).

## Tests

- `parse_color("ansi:0")`, `("ansi:255")`, `("ANSI:4")` (case), whitespace trimming.
- Rejects `ansi:256`, `ansi:-1`, `ansi:`, `ansi:x`.
- Contrast/luminance helpers do not panic on `Color::Indexed` and give a sensible result.
- A theme with indexed overrides renders (buffer test) using `Color::Indexed` cells.

## Perf

Parsing happens at config load; no render-loop change beyond color values.

## Acceptance

Documented in `configuration.mdx` Theme section ("Color values accept …") and the reference JSON
descriptions for color keys if they list formats.
