# 05 space-path-token

Source: upstream discussion #4584 "Make Git and non-Git Spaces equally scannable in the sidebar".

## Problem

Git Spaces get a second line (`branch`, `git_status`); non-Git Spaces collapse to one line and are
easy to miss. There is no token for "where is this Space".

## Behavior

A new Space row token `path`, rendered as the Space's directory with `$HOME` abbreviated to `~` and
middle-truncated to the sidebar width:

```toml
[ui.sidebar.spaces]
rows = [
  ["state_icon", "workspace"],
  ["branch", "git_status"],
  ["path"],
]
```

- The default layout does not change. Users opt in by adding the token.
- Optional companion (decide in the plan): `path_basename` for just the last component.
- Supports inline style tables and text `rules` like other text tokens.
- For remote machines the path is the remote path; abbreviate with the remote `$HOME` only if the
  server provides it, otherwise show it verbatim.

## Classification

TUI presentation for the token. The path value itself is a server/runtime fact.

## Stop-and-ask checkpoint

`ClientShellWorkspace` (in `src/protocol/wire.rs`) has `new_workspace_cwd`, but that is the cwd for
*new* workspaces and follows `terminal.new_cwd` policy, so it can be `$HOME` and is not a stable
Space path. Orientation must answer:

1. Does the server already know a stable Space directory (workspace root / first pane cwd / worktree
   path)? `rg "cwd|root" src/workspace.rs src/app/state.rs`.
2. Is it already in the snapshot (for example `worktree` for linked worktrees)?
3. If a new snapshot field is needed: `ClientShellSnapshot` structs are serde JSON (endpoint snapshot
   codec v1). An optional field with `#[serde(default, skip_serializing_if = "Option::is_none")]` is
   allowed by AGENTS.md, but check how the private same-install protocol frames `ServerMessage`
   (bincode over serde?) before concluding no `PROTOCOL_VERSION` bump is needed.
   Report findings and wait for approval before adding the field.

## Code map (at d11c0c34)

- Token enum and parsing: `src/config/sidebar.rs::SpaceSidebarToken` (around line 127) and its
  deserializer/tests.
- Token value resolution and rendering: `rg "SpaceSidebarToken::Branch"` in `src/client/shell/`.
- Snapshot population: `src/server/client_shell.rs` (workspace entries, around line 80).
- Frozen fixture `tests/fixtures/endpoint-snapshot-v1.json` must still deserialize unchanged.

## Tests

- Token parse + round-trip; rules/styles accepted on `path`.
- Home abbreviation and truncation helper tests (pure function).
- Snapshot without the field (old server) ⇒ token renders nothing and its row disappears.
- Fixture compatibility test still passes without editing the fixture.

## Perf

Sidebar work scales with Spaces. Compute the abbreviated string once per snapshot update, not per
frame, if the render path is per-frame.

## Acceptance

Default sidebar unchanged; docs updated in "Sidebar row layouts" token list.
