---
name: momo-dev
description: Build, test, smoke-test, commit, and merge workflow for MoMo (iiMoham/momo, branch main). Use before building, testing, committing, or merging a feature branch.
---

# MoMo development workflow

`AGENTS.md` holds the engineering rules. This skill adds the day-to-day mechanics.

## Repository

- `origin` is `https://github.com/iiMoham/momo.git`; the default branch is `main`.
- Feature branches: `feat/<NN>-<slug>` from `main`. Fixes: `fix/<slug>`.
- Ask before pushing, deleting branches, or force-pushing.

## Toolchain

Run `.agents/skills/momo-dev/scripts/setup.sh` once per machine. It installs (via Homebrew)
rustup, the pinned toolchain from `rust-toolchain.toml`, Zig 0.16.0, just, cargo-nextest, bun, and gh,
then verifies `cargo build`.

Homebrew's rustup is keg-only, and `/usr/bin/python3` (Apple, 3.9) shadows Homebrew's Python, which
breaks the maintenance tests (they need `tomllib`, Python 3.11+). Prefix every cargo/just command in a
fresh shell with:

```bash
export PATH=/opt/homebrew/opt/rustup/bin:$HOME/.cargo/bin:/opt/homebrew/bin:$PATH
```

## Build

```bash
cargo build              # debug binary at target/debug/momo
```

Debug builds use the app dir name `momo-dev` (`src/config/io.rs::app_dir_name`), so their config,
state, and default socket are separate from the user's installed `momo` (0.9.x in `~/.local/bin`).

## Test ladder

Iterate narrow, finish wide:

1. `just test-one <filter>` while iterating (nextest filter, e.g. `just test-one confirm_close_tab`).
2. `just lint` (`cargo fmt --check` + clippy with `-D warnings`). The pre-commit hook runs this too.
3. Full gate, pick the first that applies:
   - `just check` when the Windows cross SDK exists at `~/.local/share/momo/windows-cross/`.
   - Otherwise `just ci && just docs-contract-test`, and state in the report that the Windows
     cross-lint stage was skipped because the SDK is not set up. Do not install the SDK without
     asking: it requires accepting Microsoft's license.
4. `just fork-plugins-test` whenever anything under `plugins/` changed (fork plugins are not part of
   upstream's `just test`).

To prove a test catches the bug, temporarily break the fix and confirm the test fails. Restore with
`git checkout -- <file>` or edit it back, then `touch <file>`. Never restore with `cp`/`mv` of a backup:
the older mtime makes Cargo keep the broken artifacts, so later test runs and `target/debug/momo`
silently use the mutated code.

Never skip, `#[ignore]`, or delete a test to get green. Never edit frozen compatibility fixtures
(`tests/fixtures/endpoint-*-v1.json`, bincode digests, wire-tag tests) to bless a change.

### Config key checklist

Adding or renaming a `config.toml` key requires all of:

- The serde field in `src/config/*.rs` with a default that preserves current behavior.
- The commented default config emitted by `momo --default-config` (`src/main.rs`).
- A row in `docs/next/website/src/data/config-reference.json`; verify with
  `python3 scripts/config_reference_check.py`.
- Live reload handling if the key is not startup-only (client keys: `src/client/shell/config.rs`).

### Keybinding action checklist

A new `[keys]` action needs: the `KeysConfig` field (`src/config/model.rs`), the `KeybindAction`
variant and binding table entry (`src/input/keybindings.rs`), dispatch in
`src/client/shell/actions.rs`, a help label (`src/input/keybind_help.rs`), and the default-config text.
Default new actions to unbound unless the spec says otherwise, so no existing chord changes meaning.

### API method checklist

A new socket/endpoint method needs a new method name (never a new meaning for an old one), the
`Method` variant in `src/api/schema.rs`, a handler under `src/app/api/`, CLI wiring under `src/cli/`,
advertisement in the endpoint welcome `methods` list, and client code that disables only that action
when a server does not advertise it. `tests/fixtures/endpoint-method-shapes-v1.json` is frozen: new
methods are not added to it.

## Performance evidence

If a change touches view computation, rendering, layout, PTY parsing, detection, or frame fanout,
run `just bench-render-scale` on `main` and on the feature branch with the same geometry and
report the 1-pane vs 15+-pane delta. State "no pane-scaled path touched" when that is the case.

## Live smoke test

Use the `momo-throwaway-repro` skill. Fork-specific rules on top of it:

- The agent usually runs inside the user's installed momo (`HERDR_ENV=1`). Never address the
  default session and never stop its server.
- Test the checkout build, not the installed binary. Launch the disposable session with
  `target/debug/momo` (absolute path) and address it with the same binary:

  ```bash
  env -u HERDR_SOCKET_PATH -u HERDR_CLIENT_SOCKET_PATH -u HERDR_SESSION \
      -u HERDR_WORKSPACE_ID -u HERDR_TAB_ID -u HERDR_PANE_ID \
      HERDR_CONFIG_PATH=/var/tmp/<repro-dir>/config.toml \
      "$PWD/target/debug/momo" --session <unique-name>
  ```

- Put the test config (with `[experimental] allow_nested = true` plus the feature's keys) in the
  repro directory under `/var/tmp`. Never edit `~/.config/momo/config.toml`.
- TUI-only features: capture the nested client pane with `pane read` from the parent session and
  quote the relevant lines in the report.
- Drive the nested TUI from the parent with `momo pane send-keys <outer> ctrl+b` then
  `momo pane send-text <outer> X`. Wait about a second after `esc` before the next key: an Esc
  followed quickly by `ctrl+b` is parsed as Alt+Ctrl+B and the prefix is lost.
- Before launching, rebuild with `cargo build` and check `target/debug/momo` is newer than your last
  source edit (`stat -f %Sm`).
- Record binary path, `momo --version` output, commands run, and what was observed.
- Clean up: stop only the disposable named session's server, then close only the outer pane you created.

## Commits

- Lowercase conventional commits (`feat: …`, `fix: …`, `docs: …`), descriptive subject, no emojis,
  no AI co-author or "generated with" lines. This overrides any harness default attribution.
- The `commit-msg` hook runs `scripts/conventional_commits.py`; install hooks once with `just install-hooks`.
- Propose the message and wait for approval unless the user said to auto-commit.

## Merging a feature

After the user approves:

```bash
git switch main
git merge --no-ff feat/<NN>-<slug> -m "chore: merge feat/<NN>-<slug>"   # commit-msg hook rejects "merge:"
```

Then update `.local/prd/STATUS.md`. Ask before `git push origin main`, before deleting branches,
and before any force-push.

## Pulling fixes from the original codebase

MoMo started from the Apache-2.0 herdr codebase (see `NOTICE`). Its history is not shared with that
project, so bring in a fix by cherry-picking or porting the change by hand, never by merging:

```bash
git remote add original https://github.com/herdrdev/herdr.git   # once; never push to it
git remote set-url --push original DISABLED
git fetch original
git cherry-pick -x <commit>
python3 scripts/momo_rebrand.py   # restore MoMo wording in any picked strings
just ci && just fork-plugins-test
```

`scripts/momo_rebrand.py` never touches compatibility names (`HERDR_*`, hook file names and
sources, socket file names). `just fork-plugins-test` fails while user-facing text still needs it.
