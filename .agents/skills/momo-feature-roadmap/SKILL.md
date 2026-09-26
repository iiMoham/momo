---
name: momo-feature-roadmap
description: The fork's high-impact feature roadmap for momo - feature order, per-feature specs, classification (TUI / server-runtime / plugin), and status tracking in .local/prd/STATUS.md. Use when picking the next feature, planning one, or updating roadmap status.
---

# MoMo feature roadmap

High-impact features for the personal fork, each sourced from a real request in
upstream momo GitHub Discussions (read-only research; never post there). Specs live in
`references/`. They were written against upstream commit `d11c0c34` (2026-09-24). Upstream moves
fast: verify every path and symbol before planning, and trust the code over the spec.

## Order

Feature ids (`NN`) are stable identifiers, not positions: dropped ids are never reused and new
features get the next free id. Work top to bottom through the "Next" table. Do not reorder without
asking; the user chose this order for impact (2026-09-24: "focus on those who really have big impact,
not small adjustments").

### Next (in order)

| Order | NN | Slug | Class | Size | Source |
| --- | --- | --- | --- | --- | --- |
| 1 | 13 | `input-sync` | server runtime + TUI | L | #949 (17 upvotes), #4423 |
| 2 | 14 | `pane-output-stream` | server API/events | M | #4435 |
| 3 | 15 | `safe-worktree-removal` | server + TUI + plugin hook | M | #4513 |
| 4 | 10 | `workstream-shortcuts` | plugin | M | #4526 |
| 5 | 16 | `notification-to-pane` | client + platform | L | #4531, #4396 |
| 6 | 17 | `shared-done-acks` | server + client | M | #4420 |
| 7 | 09 | `idle-agent-reminder` | plugin | M | #4431 |
| 8 | 18 | `session-switch` | client lifecycle | L | #4292 |
| 9 | 07 | `config-includes` | config loading | M | #4265 |
| 10 | 08 | `pane-balance` | server API + CLI + key | M | #4527 |

Every L/M core feature above has a stop-and-ask checkpoint in its spec: present the design decision
before writing code.

### Done

| NN | Slug | Notes |
| --- | --- | --- |
| 01 | `confirm-close-tab` | merged |
| 02 | `tab-bar-padding` | merged |

### Dropped (specs kept in `references/dropped/`)

03 `sidebar-spacing`, 04 `indexed-colors`, 05 `space-path-token`, 06 `update-check-intervals`,
11 `pinned-panes`, 12 `tab-bar-right-styles`: cosmetic or low-impact. Do not propose features of
this kind (padding, spacing, color syntax, labels) unless the user asks.

## Workflow per feature

1. **Orient.** Read `references/NN-<slug>.md`. Verify each path and symbol with `rg`. Re-check the
   upstream discussion only if the spec is ambiguous (read-only `gh api graphql`).
2. **Plan.** Write `.local/prd/NN-<slug>.md` using the template below. Show it to the user and wait.
3. **Branch.** `git switch main && git switch -c feat/NN-<slug>`.
4. **Implement, test, smoke-test, document, commit, merge** per `momo-dev`
   (and `momo-plugin-authoring` for plugin features).
5. **Track.** Update `.local/prd/STATUS.md` at each state change.

### Plan template (`.local/prd/NN-<slug>.md`)

```markdown
# NN slug

- Base: <main commit>
- Classification: TUI presentation | server/runtime fact | plugin (why)
- Spec drift: <paths/symbols that moved or changed since the spec>
- Files to change:
- Config keys (name, type, default, reload behavior):
- API methods / fields (name, optional?, advertised how):
- CLI:
- Tests to add (name -> behavior proven):
- Perf: which pane-scaled loop is touched, if any, and how it will be measured
- Docs pages:
- Stop-and-ask items:
```

## Status tracking

`.local/prd/STATUS.md` (ignored by git) holds one row per feature:

```markdown
| NN | slug | state | branch | merged commit | notes |
```

States: `todo` → `planned` (plan written) → `approved` → `in-progress` → `review` (report delivered)
→ `merged` | `deferred` | `dropped`. Record the reason for `deferred`/`dropped`.

## Rules every spec inherits

- Off-by-default or behavior-preserving defaults. Existing configs must render and behave the same.
- New config keys follow the checklist in `momo-dev` (model, default config, reference JSON,
  reload).
- Client presentation settings belong in the client's local config; server/runtime facts belong in
  server state and the JSON API (AGENTS.md runtime/client boundary).
- No edits to frozen endpoint fixtures; new endpoint capabilities get new method names or
  optional fields.
- Stop and ask before bumping `PROTOCOL_VERSION`, adding a dependency, changing persisted state
  formats, or touching agent detection manifests.
