# 03 sidebar-spacing

Sources: upstream discussions #4530 "Spaces sidebar header has no top padding, unlike the Agents
header" and #4534 "Optional row gap between worktree child Spaces in the sidebar".

## Problem

1. The ` spaces` title is drawn at sidebar row 0 while the ` agents` title has a divider row above
   it, so the two panels look inconsistent.
2. `[ui.sidebar.spaces] row_gap` separates top-level Spaces, but indented worktree children are always
   packed, which makes long worktree groups hard to scan.

## Behavior

```toml
[ui.sidebar.spaces]
row_gap = 1          # unchanged meaning: between top-level Spaces
child_row_gap = 1    # new, default 0: between consecutive worktree children of one group
header_padding = 1   # new, default 0: blank rows above the " spaces" title
```

- Defaults keep today's rendering exactly (`header_padding = 0`, `child_row_gap = 0`).
  Do not silently change the default look; the discussion asks for parity but the fork keeps
  upstream visuals unless opted in.
- `child_row_gap` applies only between two indented entries of the same group. The gap between a
  parent and its first child, and after the last child, stays governed by current rules.
- Both keys clamp to 0..=3 like `row_gap` (verify the existing clamp).
- Expanded desktop sidebar only; collapsed and mobile views keep compact layouts.

## Classification

TUI presentation. Client-local config.

## Code map (at d11c0c34)

- `src/client/shell/sidebar.rs::render_sidebar`: the `" spaces"` title (around line 223) and the gap
  computations `u16::from(!next.indented) * config.spaces.row_gap` (around lines 265 and 354).
- `src/client/shell/endpoint_sidebar.rs` around line 347: the same gap rule for multi-machine lists.
  Both code paths must agree; hit-testing and scrolling rely on identical row heights.
- `src/client/shell/agent_sidebar.rs::render_agent_panel_header` for the Agents header reference.
- Config: `src/config/sidebar.rs::SpacesSidebarConfig` (has `row_gap`, default constant
  `DEFAULT_SIDEBAR_ROW_GAP`), existing tests near line 500.

## Tests

- Config parse/default/clamp tests next to the existing `row_gap` tests.
- Row-height tests: group of 3 children with `child_row_gap = 1` yields 2 inter-child gaps; parent to
  first child gap unchanged; top-level gaps unchanged.
- Click and scroll hit-testing still land on the right Space with both new gaps set
  (single-machine and multi-machine paths).
- Header padding shifts Space rows down by N and never pushes the Agents panel off-screen at small
  heights (verify how the split between panels is computed).

## Perf

Sidebar rows scale with workspaces, not panes; the change is arithmetic in an existing loop.

## Acceptance

Defaults identical; both keys documented in config-reference JSON and in the "Sidebar row layouts"
section of `configuration.mdx`.
