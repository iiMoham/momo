# Changelog

## [0.9.1-momo.3] - 2026-09-26

### Changed

- MoMo now stands on its own. Help text, messages, docs, and plugin examples all say MoMo, and the
  docs moved into `docs/` in this repository.
- Log files are now `momo-client.log` and `momo-server.log`.
- New worktrees go to `~/.momo/worktrees` by default. Existing checkouts stay where they are.
- Plugins can name their manifest `momo-plugin.toml`. `herdr-plugin.toml` still works.
- Config accepts `delivery = "momo"` and `[ui.toast.momo]`, and `pane input --right-click` accepts
  `momo`. The older spellings still work, and MoMo writes the new ones.

## [0.9.1-momo.2] - 2026-09-26

The first MoMo release.

### Added

- An animated pixel herd in the sidebar: one sheep per agent, colored by state. Sheep walk while
  agents work and a blinking `!` marks the ones that need you. Turn it off with
  `[ui] animation = false`.
- The midnight-neon theme (the default) and its light sibling, neon-day.
- Input sync: type into every pane of a tab at once with `momo tab sync on`, a key binding, or the
  tab menu.
- `momo pane watch` and the `pane.output_changed` event stream pane changes to scripts.
- Worktree removal checks nested repositories for uncommitted or unpushed work first
  (`momo worktree removal-check`, `--discard-nested`).
- The workstream plugin starts a task in a new worktree with your layout and finishes it safely.
- On macOS with terminal-notifier, clicking an agent's notification focuses that agent's pane.
- `ui.confirm_close_tab` and `ui.tab_bar_padding`.
- `momo update` and SSH remote installs download MoMo's own releases.
