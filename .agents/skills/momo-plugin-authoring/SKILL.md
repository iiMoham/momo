---
name: momo-plugin-authoring
description: Decide whether a momo feature belongs in a plugin instead of core, then build, link, test, and document a momo plugin (momo-plugin.toml manifest, actions, event hooks, startup hooks, panes, link handlers). Use for roadmap features classified as plugins.
---

# Authoring momo plugins for the fork

The authoritative reference is `docs/next/website/src/content/docs/plugins.mdx`, plus the plugin
sections of `socket-api.mdx` and `cli-reference.mdx`. Re-read them before starting; this skill only
adds decision rules and fork conventions.

## Plugin or core?

Prefer a plugin when all of these hold:

- The feature composes existing CLI/API calls (`momo workspace|tab|pane|agent|worktree|notification …`).
- It needs no new render surface beyond a terminal pane or popup.
- It needs no new state inside `AppState` or the wire snapshot.
- Latency of a process spawn per invocation is acceptable. Event hooks spawn a process per event, so
  they must not fire on per-byte or per-frame paths.

Prefer core when the feature changes rendering, layout geometry, input routing, detection, or the
endpoint contract, or when it must work identically for every client without installation.

If a spec's classification looks wrong once you read the code, stop and ask the user.

## Where fork plugins live

```text
plugins/<slug>/
  momo-plugin.toml
  bin/ or src/          # implementation
  README.md             # install, config, behaviour, limits
  tests/                # script-level tests (see below)
```

Plugin ids use the `momo.<slug>` namespace, for example `momo.idle-reminder`.
Set `min_herdr_version` to the oldest release whose CLI/API the plugin uses (check
`docs/next/.../cli-reference.mdx` and `git log -S` for when a command appeared). Declare `platforms`.

## Implementation rules

- Call MoMo through `$HERDR_BIN_PATH`, never a bare `momo`, so the plugin talks to the server that
  launched it (this matters when a debug `momo-dev` server runs next to the user's installed one).
- Prefer POSIX `sh` plus the momo CLI with `--json`/JSON output parsed by a tool you already depend
  on. If you need structured JSON processing, use a single small script runtime and document it.
  Do not add npm dependencies without asking.
- User config lives in `$HERDR_PLUGIN_CONFIG_DIR`, durable state in `$HERDR_PLUGIN_STATE_DIR`.
  Never write into `$HERDR_PLUGIN_ROOT`.
- Read ids from `HERDR_PLUGIN_CONTEXT_JSON` / `HERDR_*_ID` and command output; never construct ids.
- Hooks must be idempotent and exit promptly. Startup hooks are one-shot, not daemons. If a feature
  needs periodic work, drive it from events or from an explicitly started, documented pane process.
- Validate event names against `src/api/schema/events.rs::plugin_hook_event_names()`; unknown names
  only produce a link-time warning, so check `plugin list` warnings yourself.

## Test loop

1. Unit-test pure logic (parsing, thresholds, state files) with a script test runnable from
   `plugins/<slug>/tests/`; keep it dependency-free.
2. In a throwaway session (see `momo-throwaway-repro` and `momo-dev`), using the debug binary:

   ```bash
   H="$PWD/target/debug/momo"   # plus the env -u … HERDR_SESSION=<name> prefix from momo-dev
   $H plugin link "$PWD/plugins/<slug>"
   $H plugin list                 # check warnings
   $H plugin action list --plugin momo.<slug>
   $H plugin action invoke momo.<slug>.<action>
   $H plugin log list --plugin momo.<slug>
   ```

   Plugin registration is global to the user for that binary's app dir. The debug binary uses the
   `momo-dev` app dir, so linking with it does not touch the user's installed plugins. Unlink at the
   end of the smoke test.
3. Record commands, plugin log excerpts, and observed UI in the feature report.

## Documentation

Plugins are documented in `plugins/<slug>/README.md`. Only touch `docs/next/...` if the plugin needed
a core change (for example a new hookable event) that is user-facing.

## When a plugin needs a core change

Sometimes a plugin is right but core lacks one primitive (an event, a CLI flag, an optional field).
Split the work: land the minimal core primitive first as its own reviewed step (API checklist in
`momo-dev`), then the plugin. Ask the user before adding the core primitive.
