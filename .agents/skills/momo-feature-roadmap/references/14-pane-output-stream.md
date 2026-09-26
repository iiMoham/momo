# 14 pane-output-stream

Source: upstream discussion #4435 "Streaming pane output (content-change events)".

## Problem

`pane.output_matched` is edge-triggered: it fires once when a regex first matches and rearms only
after a read stops matching, so a broad regex fires once per subscription. `pane.agent_status_changed`
depends on detection flipping. Orchestrators (an agent watching other agents, dashboards, loggers)
have no reliable "this pane has new output" signal and fall back to polling `pane read`.

## Behavior

- New subscription event `pane.output_changed` scoped to one `pane_id`: emitted when the pane's
  terminal content changes, carrying `pane_id`, a monotonically increasing content `revision`, and
  optionally the changed text (see decisions).
- Coalesced: at most one event per pane per interval (`min_interval_ms`, default e.g. 100 ms,
  subscriber-chosen within bounds), so a spewing pane cannot flood subscribers.
- Opt-in per subscription; no cost when nobody subscribes.
- CLI: `momo pane watch <pane_id> [--interval-ms N] [--format text|json]` streams events to stdout.
- Not a plugin hook event (per-output hooks would spawn a process per change); keep
  `plugin_hook_event_names()` unchanged.

## Classification

Server/runtime: terminal content is server state. New event kind + subscription params.

## Stop-and-ask checkpoint

1. Payload: revision only (subscriber then calls `pane read`) vs appended lines / full screen text.
   Revision-only is cheapest and avoids defining diff semantics for alternate-screen apps.
2. Hook point: where terminal content revisions are already tracked (`content_revision` exists on
   `PaneSurfacePane`; find the server-side counter) and how hidden panes are handled. Hidden panes
   still parse output; emitting must not trigger presentation work (AGENTS.md multiplicative paths).
3. Adding an `EventKind` variant: check whether `EventKind` is reachable from any frozen codec
   (events are JSON on the socket API; confirm) and give older clients an `Unknown` fallback.

## Code map (at d11c0c34)

- Events: `src/api/schema/events.rs` (`EventKind`, `KNOWN_EVENT_KINDS`, `PLUGIN_HOOK_EVENT_KINDS`).
- Subscriptions: `src/api/subscriptions.rs`, `src/api/event_hub.rs`, `src/api/wait.rs`
  (existing `output_matched` machinery to learn from).
- Socket subscription tests: `src/api/server/subscription_socket_tests.rs`.
- CLI: `src/cli/pane.rs`.

## Tests

- Subscription emits on content change, coalesces within the interval, stops after unsubscribe.
- No emission for panes without subscribers (assert no work queued).
- Revision strictly increases; unknown pane ⇒ `pane_not_found`.
- Socket-level test streaming several events.

## Perf

Output parsing is per byte × panes. The hook must be O(1) per parse batch (compare revision, set a
dirty flag) with emission on a timer/coalescer. Measure with a flooding pane and 15+ panes, with and
without a subscriber.
