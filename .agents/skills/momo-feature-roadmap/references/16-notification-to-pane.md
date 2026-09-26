# 16 notification-to-pane

Sources: upstream discussions #4531 "System notification click should focus the originating MoMo
window/pane" and #4396 "API to select a saved machine in an attached client".

## Problem

A system notification says an agent finished or needs input, but clicking it at best brings the
terminal app forward. The user still hunts for the workspace/tab/pane, and for agents on a saved SSH
machine the local client stays on the machine it was showing.

## Behavior

- Clicking a MoMo system notification (macOS `terminal-notifier`; Linux `notify-send` actions where
  supported) focuses the terminal app **and** navigates the client that raised it to the originating
  machine → workspace → tab → pane.
- Works for panes on saved machines: the client switches its viewed machine, then focuses the pane.
- New client-control surface so automation can do the same: `momo client focus-pane --machine
  <label> <pane_id>` (name to be decided) that targets the attached client, not only the server.
- Unsupported platforms / osascript fallback: behavior unchanged, documented.

## Classification

Client-side (machine selection is client-owned federation state) + platform notification code. The
server already supports focusing a pane; the missing piece is telling a specific client to navigate.

## Stop-and-ask checkpoint

1. Addressing a client: the client socket (`herdr-client.sock`) speaks the private binary protocol.
   Options: a small JSON control endpoint on the client, a new private-protocol message, or a
   server-relayed "navigate client X" request. Each has compatibility implications; present them.
2. Notification click action: `terminal-notifier -execute` runs a shell command on click; decide
   the exact command and how it identifies the originating client (client id / socket path in env).
3. Security: the click command must not be injectable from pane-controlled text (titles/bodies come
   from agents). Pass ids as argv, never interpolate into a shell string.

## Code map (at d11c0c34)

- macOS notifications: `src/platform/macos.rs` (~line 700-800: terminal-notifier, osascript).
- Linux/Windows: `src/platform/linux.rs`, `src/platform/windows.rs`.
- Client notifications: `src/client/notifications.rs`, `src/client/shell/notifications.rs`,
  `src/client/shell/notification_policy.rs`.
- Machine selection: `src/client/shell/endpoints.rs`, `src/client/shell/endpoint_navigation.rs`,
  `~/.local/state/momo/client/endpoint-selection.json` handling.

## Tests

- Click-command construction: argv only, hostile titles/bodies cannot inject.
- Client navigation request: local pane, remote-machine pane, unknown machine/pane ⇒ notice, no crash.
- Platform code compile-gated per AGENTS.md; unit-test the pure parts.
