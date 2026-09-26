# 06 update-check-intervals

Source: upstream discussion #4485 "Configurable interval for version_check and manifest_check".

## Problem

`update.version_check` and `update.manifest_check` are booleans. When enabled they run every 30
minutes (`AUTO_UPDATE_CHECK_INTERVAL`), which adds up across many machines and sessions.

## Behavior

```toml
[update]
version_check_interval_minutes = 30    # default 30, range 5..=10080 (1 week)
manifest_check_interval_minutes = 30   # default 30, same range
```

- The booleans still enable/disable; intervals only apply when enabled.
- Out-of-range values clamp with a config diagnostic (match how other ranges report).
- Reload: new intervals take effect at the next scheduled check after `server reload-config`
  (reschedule from "now" when the value changes; document it).
- Named sessions each run their own checks today; the interval is per server process. Document that.

## Classification

Server/runtime configuration. No API or wire change.

## Code map (at d11c0c34)

- `src/app/mod.rs`: `const AUTO_UPDATE_CHECK_INTERVAL` (line ~41), `update_manifest_check_enabled`,
  scheduling near lines 548-600.
- `src/app/runtime.rs` lines ~110-130: `next_*_check` scheduling uses the constant twice (version
  and agent manifest). Replace with per-kind durations stored on `App`.
- `src/config/model.rs::UpdateConfig` (line ~33).
- Reload path: `rg "reload_config|apply_config" src/app`.

## Tests

- Config parse/default/clamp tests.
- Scheduling test with an injected clock or by asserting the computed `Instant` offset from the
  configured duration (avoid wall-clock waits).
- Reload changes the next deadline.

## Perf

Timer bookkeeping only.

## Acceptance

Reference JSON rows for both keys; mention in `configuration.mdx` or the update docs page if one
describes checks (`rg "version_check" docs/next`).
