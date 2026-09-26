# 11 pinned-panes (plugin)

Source: upstream discussion #4522 "Add a new panel on the side to store favorite terminal sessions".

## Problem

With many agents and panes, users want a few "favorite" panes to stand out and be quick to jump to.

## Behavior

Plugin `momo.pinned-panes` in `plugins/pinned-panes/`:

- Action `toggle` (context: pane): pins/unpins the focused pane by reporting pane metadata
  `momo pane report-metadata <pane> --source momo.pinned-panes --token pin=★` or
  `--clear-token pin`.
- Pins are persisted in `$HERDR_PLUGIN_STATE_DIR/pins.json` (by pane id). A startup hook re-reports
  metadata for pins whose panes still exist after restore; a `pane.closed` hook drops the pin.
- Action `jump` opens a popup listing pinned panes (workspace / tab / agent) and focuses the chosen
  one with `momo pane focus`.
- README shows how to display the pin in the sidebar with a custom token:

  ```toml
  [ui.sidebar.agents]
  rows = [["state_icon", "$pin", "agent"], ["workspace", "tab"]]
  ```

## Classification

Plugin, using the existing metadata token mechanism. No new sidebar panel in core (a new panel is a
core UI change that upstream is discussing separately; do not build it here).

## Open questions for the plan

- Do pane ids survive a server restart / snapshot restore? Check `session-state.mdx` and
  `src/persist/`. If not, key pins by a stable attribute (workspace + tab + pane name, or cwd) and
  document the limitation.
- Metadata source slot limit: a pane accepts sequenced reports from at most 32 distinct sources; this
  plugin uses one fixed source. Do not pass `--seq` unless needed.

## Tests

- Pure tests for pins.json handling (toggle, prune on close, restore mapping).
- Smoke: toggle pin on a pane in a throwaway session, verify `pane get` shows the token, restart the
  throwaway server, verify pin restored or documented limitation holds.

## Acceptance

README with install, sidebar config snippet, limits.
