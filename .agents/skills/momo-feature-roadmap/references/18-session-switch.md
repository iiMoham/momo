# 18 session-switch

Source: upstream discussion #4292 "Let plugins switch the current client between named sessions"
(a community proof of concept exists; read it for ideas, do not copy it wholesale).

## Problem

tmux users port `tmux-sessionizer`: pick a project with fzf, switch the current terminal to that
project's own session, keep the old one running. MoMo has `session attach`, but nothing moves an
already attached client to another named session, so pickers must nest clients or ask the user to
detach manually.

## Behavior

- `momo client switch-session <name> [--create] [--cwd PATH]`, usable from custom commands and
  plugin actions/popups. It switches the client that invoked it (identified from the invocation
  context), leaves other attached clients where they are, and keeps the previous session running.
- `--create` starts the named session if missing, with `--cwd` for its first workspace.
- A keybinding-free design: the picker lives in a plugin or custom command; core only provides the
  switch primitive.

## Classification

Client lifecycle (which server a client is attached to) + a way for a command launched by that client
to address it.

## Stop-and-ask checkpoint

1. Identifying the invoking client: custom commands and plugin panes receive server ids today, not a
   client id. Options: inject a `HERDR_CLIENT_ID`/client socket path into commands launched from a
   client; or route through the server to the client that owns the invocation. Present options.
2. Switching mechanics: in-process reattach (tear down endpoint, attach to the other session's socket)
   vs exec-replace of the client process. Reattach keeps terminal state; exec is simpler. Decide.
3. Interaction with multi-machine clients (switching the Local endpoint only vs any endpoint).

## Code map (at d11c0c34)

- Session plumbing: `src/session.rs` (named sessions, attach command construction), `src/cli.rs`.
- Client attach/startup: `src/client/attach.rs`, `src/client/startup.rs`, `src/client/endpoint/`.
- Custom command env: `src/app/custom_commands.rs`; plugin env: `src/app/api/plugins/`.

## Tests

- Switching resolves the target session socket, starts it with `--create`, errors cleanly otherwise.
- The invoking client switches; a second attached client does not.
- Previous session keeps running (process/state assertion without PTYs where possible).
