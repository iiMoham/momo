# MoMo

Terminal runtime for coding agents: persistent workspaces, tabs, and panes, with agent detection
and a socket API. The binary is `momo`.

These are the working rules for anyone changing MoMo, human or agent.

## Engineering rules

### Principles

- **State is separated from runtime.** `AppState` is pure data, testable without PTYs or async. `PaneState` is separate from `PaneRuntime`. Workspace logic doesn't need real terminals.
- **Render is pure.** `compute_view()` handles geometry and mutations. `render()` takes `&AppState` and only draws. Never mutate state during render.
- **No god objects.** If a module is doing too many things, split it. `app/` is already split into state, actions, and input. Keep it that way.
- **Platform code is isolated.** OS-specific behavior lives in the matching `src/platform/<os>.rs` file, with only shared traits, types, wrappers, and testable contracts in `src/platform/mod.rs`. Core modules don't have `#[cfg(target_os)]`.
- **Detection is decoupled.** The detector reads a screen snapshot, never touches the parser or viewport state.
- **Screen detection is evidence-based.** When changing `src/detect/manifests/`, first capture the relevant bottom-buffer state with `momo agent read <pane> --source detection --format text` and, when styling or alternate screen behavior matters, `--format ansi`. Decide which visible controls are invariant, which are alternatives, and encode them as explicit AND/OR gates. Do not match whole-pane incidental text, and do not use the user-visible viewport for agent status because users can scroll it.
- **UI patterns should be reused.** MoMo is a mouse-first TUI. New dialogs, onboarding, settings, and post-update flows should follow the existing UI/UX language and interaction patterns instead of inventing one-off screens. Prefer reusing existing modal/screen structure, affordances, and close actions so the app feels consistent.

### Multiplicative performance paths

Treat work reachable from view computation, rendering, background-pane resizing,
PTY parsing, detection, and client frame fanout as multiplicative. Before adding
work, identify its frequency and cardinality: per byte, event, or render × panes,
tabs, or workspaces × attached clients.

Inside pane-scaled render and layout loops:

- Use narrow terminal-state accessors. Do not collect aggregate input state,
  format terminal snapshots, inspect process trees, perform filesystem I/O, or
  allocate when one scalar fact is enough.
- Keep terminal-core lock duration minimal.
- Preserve hidden-source and retained-render early exits. Hidden panes still
  parse output, but their output must not trigger presentation work merely to
  keep terminal or detection state current.
- When a change adds or widens work in one of these loops, profile fixed geometry
  with 1 and at least 15 populated panes and report the scaling delta. Use
  `just bench-render-scale` to exercise both background-workspace and active-pane
  cardinality when applicable.

Prefer deterministic operation or architecture tests to wall-clock CI limits.
Performance benchmarks are supporting evidence, not substitutes for behavioral
coverage. `just bench-render-scale` profiles fixed geometry with 1 and 15 populated panes.

### Runtime/client boundary guardrail

MoMo is migrating toward a server-owned runtime protocol with the TUI as one client. New work should not deepen the current server/TUI coupling.

Before adding state, API fields, events, commands, or socket messages, classify the feature:

- Shared runtime/session fact: belongs in server state and should be exposed through the JSON API/event path when practical.
- TUI presentation state: belongs only in the TUI/client layer.

Do not add new shared behavior that only works through the private TUI client socket. Use neutral server/API names, not UI-surface names like sidebar, row, card, or widget.

Examples:

- Pane/agent metadata, process state, terminal state, events: server/runtime.
- Sidebar layout, token placement, colors, selection, modals, mouse/viewport state: TUI/client.
- Workspace/tab/pane remain shared session organization for now, but avoid making them mandatory identity for unrelated runtime features.

### Stable client endpoint contract

The client-owned TUI endpoint generation is independent from the private same-install protocol. Generation 1 is the compatibility floor for Local, SSH, and Cloud connections and must remain available unless retired for a security reason.

- Named core codecs are immutable. Do not add, remove, reorder, or reinterpret fields or enum variants reachable from a published codec. Introduce a new codec name and keep the old codec as a fallback instead.
- Keep baseline JSON handshake and snapshot fields required. New JSON fields must be optional or have field-specific defaults; new enum values need an `Unknown` fallback where older clients can safely ignore them.
- Add server features through advertised API methods and optional snapshot data when possible. A missing optional feature must disable only that action, not reject the connection.
- Do not change the meaning or load-bearing parameter shape of an advertised endpoint method. If an old server could ignore a new field and incorrectly report success, add a new method name or a separately advertised capability and omit that field without it.
- Missing methods, rejections, timeouts, and unavailable servers are client-local outcomes. They must not disconnect other compatible servers, and typing in a pane must not dismiss their notices.
- Frozen endpoint fixtures, bincode digests, wire-tag tests, and `tests/fixtures/endpoint-method-shapes-v1.json` are compatibility contracts. Never update a generation-1 expectation merely to bless a wire change; create and negotiate a new codec or method.
- MoMo's release manifest (`latest.json`) advertises `endpoint_generation`. Keep `packaging/momo/release_manifest.py` aligned so an older updater knows when a new server generation really requires replacement.
- Existing-value digests cannot detect an appended enum variant. Review every enum reachable from a frozen codec as append-closed even when tests remain green.

### Compatibility names

MoMo keeps a few identifiers from the codebase it grew out of, because agent hooks, plugins, and
machines already depend on them: the `HERDR_*` environment variables, integration hook file names
(`herdr-agent-state.sh` and friends) and hook sources (`herdr:claude`), socket file names, the
`herdr-plugin.toml` manifest name (accepted next to `momo-plugin.toml`), and the `delivery =
"herdr"` / `[ui.toast.herdr]` config spellings (accepted next to `momo`). Do not rename these; add
new names as aliases instead. Everything a person reads (UI text, help, messages, docs) says MoMo.
`scripts/momo_rebrand.py --check` (part of `just fork-plugins-test`) fails when user-facing text
still uses the old name.

## Testing

Use `just` recipes instead of invoking cargo or scripts directly.

```bash
just ci                  # formatting, clippy, nextest, maintenance tests
just fork-plugins-test   # plugin, packaging, and rename checks
just check               # ci + Windows target lint + fork-plugins-test
```

Run `just ci && just fork-plugins-test` before committing. Do not bypass failing checks; fix the
failure or explain exactly why a narrower check is enough. Several test files only run on Linux
(`tests/cli.rs`, `tests/machine_api.rs`, `tests/auto_detect.rs`, `tests/live_handoff.rs`,
`tests/host_shutdown.rs`, and tests in `src/platform/linux.rs`), so a green macOS run is not
enough: wait for the Linux job in MoMo CI before releasing.

Windows cross-lint from macOS or Linux needs the Windows SDK: install `xwin` with
`cargo install xwin --locked`, run `just setup-windows-cross` once, and accept Microsoft's SDK
license. `just windows-lint` then checks `cfg(windows)` code. MoMo does not ship Windows builds,
but the Windows code must keep compiling.

Unit tests live next to the code (`#[cfg(test)] mod tests`). New `AppState` or `Workspace` behavior
should be testable with `AppState::test_new()` and `Workspace::test_new()` without PTYs.

For broad refactors, classify the risk before editing. Treat changes as refactor-risk when they
touch two or more core surfaces, persisted state, protocol/API IDs, workspace/tab/pane identity,
restore/handoff, agent detection authority, or UI/input state projection. Identify the protected
behavior and add or name characterization tests first. Identity/state refactors should use
`AppState::assert_invariants_for_test()` or `Workspace::assert_invariants_for_test()` with
adversarial state from `AppState::test_with_adversarial_identity_state()` or
`Workspace::test_adversarial_identity_state()`.

To try a debug build from inside a running MoMo session, clear the inherited socket overrides so
the debug binary talks to its own `momo-dev` server:

```bash
env -u HERDR_SOCKET_PATH -u HERDR_CLIENT_SOCKET_PATH cargo run -- <command>
```

## Agent detection updates

Use the manifest hot-reload loop. Create a disposable named session (see the `momo-throwaway-repro`
skill) and drive the real agent UI into the target state. Read the pane with
`momo agent read <pane> --source detection --format text` and inspect matching with
`momo agent explain <pane> --json`. Update the bundled manifest in `src/detect/manifests/<agent>.toml`,
copy it to `~/.config/momo/agent-detection/<agent>.toml`, then run
`momo server reload-agent-manifests` against the test session. Never overwrite an existing override
without asking, and remove the temporary override when the rule is right.

Unit-test the detection engine, not individual agents' screens: synthetic manifests and minimal
input strings for parsing, regions, matching, AND/OR/NOT gates, rule priority, source precedence,
and reload. Validate agent-specific behavior with live smoke tests of the changed state and nearby
transitions, and record the agent's CLI version.

## Vendored libghostty-vt

`vendor/libghostty-vt.vendor.json` records the upstream source commit currently vendored.

Local patches on top of the vendored source must be tracked in `vendor/libghostty-vt.patches.md` and stored as patch files under `vendor/patches/libghostty-vt/`. Each entry should say why the patch exists, the MoMo issue, upstream PR/discussion, vendored base commit, touched files, verification, and the exact removal condition.

When updating libghostty-vt, check every active patch in `vendor/libghostty-vt.patches.md`. If the new upstream commit contains the fix, remove the local patch and index entry, then rerun the listed verification. If not, reapply the patch on top of the new vendored source.

`just check` runs maintenance tests that verify local libghostty-vt patch files are listed in the index and reverse-apply cleanly against the vendored tree. Do not leave a patch file untracked or an indexed patch unapplied.

## Docs

User docs live in `README.md` (overview, install, shortcuts, commands) and `docs/` (one Markdown
page per topic). Update them in the same change as any user-facing behavior. `docs/config-reference.json`
lists every config key; `scripts/config_reference_check.py` fails when a key is missing or stale.
`skills/momo/SKILL.md` teaches coding agents to drive MoMo and is printed by `momo --skill`; keep
it in step with the CLI. Put local plans and notes under `.local/prd/` (git-ignored).

## Releases

Releases are cut by pushing an annotated `momo-v<base>-momo.<N>` tag; see
`packaging/momo/README.md`. Wait for green MoMo CI on the commit (Linux and macOS) first.

## Commit style

Use lowercase conventional commits, no emojis, and no AI co-author lines. Keep subjects
descriptive; they feed release notes. Propose the commit message before committing. Reference
issues with a `refs #<number>` body line, not closing keywords.

## Code conventions

- Rust: no `unwrap()` in production code. Use `tracing` for logging. Use `#[allow]` only with a comment explaining why.
- Rust platform-specific code must be compile-gated. Put OS APIs and substantial OS behavior in `src/platform/`; when platform checks are needed elsewhere, use `#[cfg(windows)]`, `#[cfg(unix)]`, or target-specific `#[cfg(...)]` on imports, fields, functions, impls, and match arms so Windows-only code does not compile into Unix builds and Unix-only code does not compile into Windows builds. Use `cfg!(...)` only for pure cross-platform policy constants whose branches both compile on every target.
- Don't add dependencies without a reason. Check whether existing dependencies cover the need first.
- Integration asset versions (`HERDR_INTEGRATION_VERSION` markers and matching `*_INTEGRATION_VERSION` constants) are migration versions relative to the latest released tag, not per-commit counters on `main`. If an integration asset changes multiple times between releases, bump it once from the version in the latest release.
- When changing the server/client wire protocol, compare `src/protocol/wire.rs::PROTOCOL_VERSION` against the protocol of the latest MoMo release. Bump it when the current source protocol has already been released and the wire format changes incompatibly. Do not bump it again for multiple incompatible changes before that protocol is published. Update hardcoded protocol expectations and manual protocol fixtures in tests.
