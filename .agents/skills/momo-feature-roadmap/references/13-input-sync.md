# 13 input-sync

Sources: upstream discussions #4423 "Hotkey / Toggle to Synchronize Input to All Panes" (closed as
duplicate of #949 — read #949 during orientation for maintainer commentary).

## Problem

tmux users rely on `synchronize-panes` to type the same command into several shells (fleet SSH
sessions, parallel test runners, several agents).

## Behavior

- Per-tab toggle "sync input". While on, keyboard input and paste sent to the focused pane are also
  delivered to every other terminal pane in the same tab.
- Keybinding action `toggle_input_sync`, unbound by default; also a tab context-menu item.
- Visible indicator while on (for example a `SYNC` pill in the tab row, like the `ZOOM` pill, and a
  marker on the tab label). An always-visible indicator is mandatory: silent broadcast is dangerous.
- Excluded targets: popups, plugin popups, panes whose process has exited, and (decide in plan)
  panes running a detected agent unless the user opts in with `ui.input_sync_include_agents = true`.
- Mouse input, scrolling, and copy mode are never broadcast.
- Socket/CLI: `tab.set_input_sync { tab_id, enabled }` and `momo tab sync on|off|toggle [--tab ID]`
  so automation can see and change it; `tab.get`/list exposes an optional `input_sync` field.

## Classification

Mixed, and the hardest item on the roadmap:

- Whether a tab is synced is shared session state (every attached client must show the same
  indicator and the fanout must happen where the PTYs live) ⇒ server/runtime.
- The indicator placement is TUI presentation.

## Stop-and-ask checkpoint (mandatory before coding)

1. Input path: find how client keystrokes reach PTYs (`src/server/pane_input.rs`,
   `src/client/input/`, the input codec in `src/protocol/endpoint.rs` `INPUT_CODEC_V1`). Fanout
   must happen on the server after the target pane's input encoding is chosen; each target pane may
   have different keyboard modes (kitty keyboard protocol, bracketed paste), so bytes cannot simply be
   copied. Describe the design.
2. Snapshot: the client needs the flag per tab. `ClientShellTab` gains an optional field; confirm
   JSON-optional compatibility and whether the private protocol needs a `PROTOCOL_VERSION` bump.
3. Persistence: decide whether sync survives restart (recommended: no, reset to off, so no persisted
   format change).
4. Multiplicative path: input is per keystroke × panes in tab. Measure latency with 1 vs 15 panes.

Present the design and the answers, then wait for approval.

## Tests

- State tests: toggle on/off, per-tab isolation, flag cleared when the tab closes, not persisted.
- Fanout tests at the server input layer with fake pane runtimes: focused + N targets receive
  input; excluded panes do not; paste respects each target's bracketed-paste mode.
- API/CLI tests for the new methods; client hides the action when the server does not advertise it.
- Indicator render test.

## Perf

Per-keystroke fanout across panes; report 1 vs 15 pane latency/throughput with a paste of a large
buffer. Keep lock scope per pane minimal.

## Acceptance

Smoke: three shell panes in one tab, sync on, type `echo hi`, all three print `hi`; sync off, only
focused pane receives input; indicator visible in both states as appropriate.
