# 08 pane-balance

Source: upstream discussion #4527 "Pane balancing (pane.balance / momo pane balance)".

## Problem

Repeated splits halve the focused pane each time (50% / 25% / 25% …). There is no one-shot way to
equalize a tab's panes, like tmux `select-layout even-horizontal`.

## Behavior

- Socket method `pane.balance` with params `{ "tab_id": "<id>" }` (or a pane target resolving to its
  tab; pick one shape in the plan and keep it). Response: the updated layout snapshot the other
  layout methods return.
- CLI: `momo pane balance [--tab ID]` (defaults to the caller's tab via `HERDR_TAB_ID`, then focused tab).
- Keybinding action `balance_panes`, unbound by default.
- Algorithm: for each split node, set the ratio so every leaf in a same-direction chain gets equal
  space. A chain of same-direction splits `A | (B | C)` becomes 1/3 each, not 1/2 + 1/4 + 1/4.
  Mixed-direction subtrees are balanced recursively. Zoom is left as is.
- Emits the same layout-changed event as `layout.set_split_ratio` / `pane.resize`.

## Classification

Server/runtime: layout is shared session state owned by the server. The TUI only sends the method.

## Compatibility

- New method name; nothing in `tests/fixtures/endpoint-method-shapes-v1.json` changes.
- Advertise the method in the endpoint welcome `methods` list (`src/protocol/endpoint.rs::compatible`
  callers). The client must hide/disable the keybinding action when the selected server does not
  advertise `pane.balance` (older remote servers) instead of erroring the connection.

## Code map (at d11c0c34)

- `src/layout.rs`: `Node::Split { direction, ratio, first, second }`, `TileLayout::set_ratio_at`.
  Add a pure `balance()` on `TileLayout`/`Node`.
- Schema: `src/api/schema.rs` (`Method` enum, `#[serde(rename = "pane.…")]`),
  params in `src/api/schema/panes.rs`, response in `src/api/schema/response.rs`.
- Handler: `src/app/api/layouts.rs` or `panes.rs` next to split-ratio handling.
- CLI: `src/cli/pane.rs` and `src/cli/spec*` (command spec); completions.
- Keybinding: checklist in `momo-dev`.
- Docs: `cli-reference.mdx` (Panes), `socket-api.mdx` (Raw methods), keyboard/config reference.

## Tests

- Pure layout tests: 2, 3, 4 same-direction panes ⇒ equal widths (within one cell after rounding);
  mixed tree; single pane no-op; ratios stay within the existing min/max ratio clamp.
- API test via `AppState::test_new()`-style fixtures: balance returns ok and changes ratios; unknown
  tab ⇒ `tab_not_found`.
- Client test: action disabled when method not advertised.
- Schema tests for request/response JSON shape.

## Perf

One tree walk per invocation; not in a render loop. The resulting resize touches every pane in the
tab once, like any resize.

## Acceptance

Smoke: create 3 vertical splits in a throwaway session, `pane balance`, then `pane layout` shows equal
widths.
