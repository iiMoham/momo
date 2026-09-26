# 01 confirm-close-tab

Source: upstream discussion #4533 "Optional confirmation before closing a tab" (4 upvotes).

## Problem

A misfired `prefix+shift+x` or a wrong pick in a tab's right-click menu closes a tab and kills
every process in it immediately. MoMo only confirms when the close would remove a workspace
(closing the last tab, or a workspace close with `ui.confirm_close = true`).

## Behavior

New client setting, off by default:

```toml
[ui]
confirm_close_tab = true
```

- When on, closing a non-last tab from the keybinding or the tab context menu opens the existing
  confirm dialog (Enter / click confirm closes, Esc / click cancel keeps the tab).
- Title `Close tab?`, detail `<tab label> — N pane(s)` (same format as the workspace dialog).
- Closing the last tab keeps today's behavior: the workspace dialog (governed by `confirm_close`)
  is the only prompt. Never show two prompts for one action.
- CLI and socket `tab.close` stay non-interactive and unchanged.
- The key is a client presentation preference: it comes from the client's local config, including
  when the client views a remote machine.

## Classification

TUI presentation state only. No server, API, or wire change.

## Code map (at d11c0c34)

- `src/client/shell/overlay_input.rs::request_tab_close` is the single entry for client tab closes
  (callers: `actions.rs` `KeybindAction::CloseTab`, `context_menu.rs` tab menu).
  It already opens `open_close_confirmation(workspace_id, Some(tab_id))` for the last-tab case.
- `open_close_confirmation` builds `ClientConfirmCloseOverlay { workspace_id, tab_target, title, detail }`.
  `tab_target: Some(ClientTabCloseConfirmation)` already makes `accept_close_confirmation` send
  `Method::TabClose` with a stale-target check. Reuse this path; add a tab-scoped constructor rather
  than overloading the workspace wording.
- Rendering: `src/client/shell/overlays.rs::render_confirm_close_overlay` (no change expected).
- Config: `src/config/model.rs::UiConfig` (`confirm_close` lives there, default in `Default` impl),
  client copy in `src/client/shell/state.rs` (`confirm_close: bool`), wiring in
  `src/client/shell/config.rs` (initial load around line 149, live reload around line 339).
- Default config text: `src/main.rs` (`# confirm_close = true` is near line 280).
- Docs: `docs/next/website/src/data/config-reference.json` (ui section),
  `docs/next/website/src/content/docs/configuration.mdx` if a sentence helps.

## Tests

Client-shell unit tests (see `src/client/shell/tests/` for fixtures that build a snapshot with
several tabs):

- `confirm_close_tab_off_closes_non_last_tab_immediately` (current behavior preserved).
- `confirm_close_tab_on_opens_tab_dialog_for_non_last_tab` and Enter sends `TabClose` for that tab.
- Esc leaves the tab and sends nothing.
- Last tab with both settings on still opens exactly one (workspace) dialog.
- Stale target: tab disappears while the dialog is open → "Close target changed" notice, no close.
- Config parse/default test in `src/config` and a live-reload test that toggles the key.

## Perf

Not on a pane-scaled path (one snapshot scan on a user action).

## Acceptance

- Default config: behavior byte-for-byte unchanged.
- `config_reference_check.py` passes; `momo --default-config` lists the key commented out.
- Smoke: throwaway session with two tabs and `confirm_close_tab = true`; `prefix+shift+x` shows the
  dialog, Esc keeps the tab, Enter closes it. Capture both screens with `pane read`.
