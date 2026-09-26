# 02 tab-bar-padding

Source: upstream discussion #4398 "Blank padding rows under the tab bar (tmux `status 2` equivalent)".

## Problem

With the tmux-style look (`ui.pane_outer_borders = false`, `ui.pane_gaps = false`) pane content
starts directly against the tab bar, which reads as cramped on small screens.

## Behavior

```toml
[ui]
tab_bar_padding = 1   # integer, default 0, clamp 0..=2
```

- Blank rows between the tab bar and the pane area. With `tab_bar_position = "bottom"` the padding
  sits between the panes and the bar.
- Padding exists only while the tab bar is shown (it disappears with
  `hide_tab_bar_when_single_tab` when there is one tab).
- Padding rows use the tab bar's background treatment decision: plain host background (no fill)
  unless the spec review decides otherwise. Clicking them does nothing.
- Mobile / narrow layouts ignore the setting. If the terminal is too short to keep at least one pane
  row, padding collapses to 0 first.
- Prefix / Navigate / Copy / Resize mode bars that replace the bottom tab row are unaffected.

## Classification

TUI presentation (client layout geometry). Pane PTY sizes shrink by the padding rows; that is the
normal effect of client geometry, not a server change.

## Code map (at d11c0c34)

- `src/client/shell/config.rs` around lines 385-420: computes `tab_bar` and `pane_surface` rects from
  `tab_height` and `tab_bar_position`. Add padding here only.
- Check every consumer of `ClientShellLayout.tab_bar` / `pane_surface` (mouse hit testing in
  `mouse.rs`, mode bars in `render.rs`, `mobile.rs`) with `rg "tab_bar\b|pane_surface"`.
- Config: `UiConfig` in `src/config/model.rs`, client state `src/client/shell/state.rs`,
  reload in `src/client/shell/config.rs`.
- Default config text in `src/main.rs`; reference JSON.

## Tests

- Layout unit tests: top/bottom × padding 0/1/2 produce the expected rects; hidden tab bar ⇒ no
  padding; very short terminal ⇒ padding collapses before panes lose their last row.
- Mouse hit test: click in a padding row does not select a tab or a pane.
- Existing layout snapshot tests unchanged with the default.

## Perf

Geometry is computed per view, not per pane; negligible. State "no pane-scaled loop touched" after
confirming padding does not add per-pane work.

## Acceptance

Default layout identical. Smoke test both positions and capture with `pane read`.
