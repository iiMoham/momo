# 12 tab-bar-right-styles

Source: upstream discussion #4305 "per-entry fg / bg / bold for ui.tab_bar_right entries" (most
upvoted idea in the recent list; several comments ask for value-dependent `rules`).

## Problem

`ui.tab_bar_right` entries render with one fixed style. Status segments (CPU, usage limits,
recording state) cannot become conspicuous when a threshold is crossed.

## Behavior

Optional style fields on any entry, reusing the sidebar token vocabulary:

```toml
[ui]
tab_bar_right = [
  { type = "command", command = "~/bin/cpu.sh", interval_seconds = 5,
    fg = "#1e1e2e", bg = "#a6e3a1", bold = true,
    rules = [{ gt = 80, bg = "#f38ba8" }, { gt = 60, bg = "#f9e2af" }] },
]
```

- `fg`, `bg`, `bold`, `dim` optional; omitted fields keep today's contextual style.
- `rules` use the exact sidebar rule semantics (first match wins; `equals|contains|starts_with|gt|lt`;
  `ignore_case`; `hide`), evaluated against the segment's resolved text. Reuse the sidebar rule
  types and matcher; do not fork them.
- Styling is declared in config only. Command output stays sanitized plain text (no SGR passthrough).
- Separators are not styled.

## Classification and the wire question (stop and ask)

Entries resolve on the server (`hostname`, `datetime`, `command` run where the panes run), and the
client receives `tab_bar_right: Vec<ClientShellTabStatusSegment { text, accent }>` in the snapshot
(`src/protocol/wire.rs` ~line 1021, populated in `src/server/client_shell.rs` ~line 207). Hidden
entries are omitted, so the client cannot map segments back to config indexes.

Options to present to the user before coding:

1. **Server resolves style.** Add optional `style` to the segment (`#[serde(default,
   skip_serializing_if = "Option::is_none")]`). Old clients ignore it. Needs confirmation of how the
   private same-install protocol frames `ServerMessage` (bincode-over-serde would make this a
   `PROTOCOL_VERSION` bump). Style comes from the *server's* config, which contradicts "presentation
   comes from the client's local config" for remote attach.
2. **Client resolves style.** Add an optional stable `entry_index` (or `entry_id`) per segment; the
   client looks up style/rules in its own local `ui.tab_bar_right` config. Presentation stays
   client-local, but it only works when client and server configs describe the same entries.
3. **Split config.** Keep entries on the server, add a client-local `[ui.tab_bar_right_styles]` keyed
   by an explicit `id` field on each entry. Most explicit, more config surface.

Recommend one (option 2 or 3 fits AGENTS.md's boundary best) and wait for approval.

## Code map (at d11c0c34)

- Config: `src/config/tab_bar.rs::TabBarRightEntryConfig` (`deny_unknown_fields`, tagged enum).
  Adding fields to every variant: consider a wrapper struct with `#[serde(flatten)]` for style,
  and check `deny_unknown_fields` + `flatten` interaction (serde does not support both together).
- Sidebar styles and rules: `src/config/sidebar.rs` (`SidebarTokenStyle`, rules types).
- Server status resolution: `src/app/tab_bar_status.rs`.
- Client render: `src/client/shell/tabs.rs` (~lines 220-300).
- Frozen fixture: `tests/fixtures/endpoint-snapshot-v1.json` includes `tab_bar_right`; it must keep
  deserializing unchanged.

## Tests

- Config parse for every entry type with and without style; invalid rule rejected with diagnostic.
- Rule matching reuses sidebar tests' semantics (numeric parse, first match, hide).
- Render test: styled segment cells carry fg/bg/bold; separators unchanged; width math unchanged.
- Compatibility: old snapshot JSON (fixture) deserializes; new snapshot without styles serializes
  byte-identical to today.

## Perf

Tab bar renders once per frame per client; rules run per segment (≤16). Cache compiled rules at
config load.
