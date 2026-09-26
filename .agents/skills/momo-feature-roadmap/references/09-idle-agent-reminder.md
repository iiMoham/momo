# 09 idle-agent-reminder (plugin)

Source: upstream discussion #4431 "notification of inactive time for claude code" (users want a
nudge before a provider's prompt cache expires, about one hour for Claude Code).

## Problem

An agent finishes (`done`/`idle`) in a background tab and is forgotten. After ~1h the prompt cache
is cold and resuming costs more.

## Behavior

Plugin `momo.idle-reminder` in `plugins/idle-reminder/`:

- Tracks when each agent pane last entered `idle` or `done`, and clears when it leaves those
  states or the pane closes.
- When an agent has been idle for `remind_after_minutes` (default 50), shows
  `momo notification show "<agent> idle 50m" --body "<workspace> / <tab>" --sound request` once per
  idle period.
- Config file `$HERDR_PLUGIN_CONFIG_DIR/config.toml` (or `.env`, pick one): `remind_after_minutes`,
  `agents = ["claude", "codex"]` filter (empty = all), `enabled`.
- Action `snooze` (context: pane) suppresses reminders for the focused agent's current idle period.
- Action `status` prints tracked agents and idle durations.

## Classification

Plugin. It composes existing events + CLI and needs no core state. See `momo-plugin-authoring`.

## Design problem to solve in the plan

Plugins have event hooks and one-shot startup hooks, but no timers. Options:

1. A startup hook that launches a supervised background process (`nohup`-style) which subscribes to
   events (`momo` event subscription over the socket) and sleeps until the next deadline. Must exit
   when the server goes away and must not duplicate itself after live handoff (use a pidfile/lock in
   `$HERDR_PLUGIN_STATE_DIR`).
2. Event hooks on `pane.agent_status_changed` record timestamps, and a pane-hosted process
   (plugin `[[panes]]` entry) does the waiting. Visible, but costs a pane.
3. A small core primitive (for example a delayed/scheduled event) — this is a core change: ask first.

Recommend one in the plan with its failure modes. `pane.agent_status_changed` is in
`src/api/schema/events.rs::PLUGIN_HOOK_EVENT_KINDS`; confirm the event JSON fields.

## Tests

- Pure logic tests for the idle tracker (state transitions, one reminder per idle period, snooze,
  agent filter) runnable without MoMo.
- Smoke test in a throwaway session with `remind_after_minutes` lowered (allow a seconds override for
  tests, for example `remind_after_seconds`), using a shell pane with a reported fake agent state
  (`momo pane report-agent` / integration docs) rather than a paid agent.

## Perf

Event hooks spawn a process per status change, which is low frequency. The background waiter must
sleep, not poll tightly.

## Acceptance

README with install (`momo plugin link`), config, limits (per-server, lost on server stop unless
state is persisted), and uninstall.
