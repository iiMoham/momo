# 10 workstream-shortcuts (plugin)

Source: upstream discussion #4526 "quickly adding worktrees" (start / finish a workstream with one
shortcut each).

## Problem

Starting a task means: pick a branch name, create a worktree, open it as a workspace, lay out
panes, maybe launch an agent. Finishing means: close the workspace and remove the checkout. Users
script this in dotfiles today.

## Behavior

Plugin `momo.workstream` in `plugins/workstream/`:

- Action `start` (context: workspace). Opens a popup pane (`placement = "popup"`) that asks for a
  branch name (and optional base ref), then runs
  `momo worktree create --workspace <id> --branch <name> [--base <ref>] --focus`, then applies an
  optional layout recipe from config (for example split right and run a command in each pane).
- Action `finish` (context: workspace). Only on linked-worktree workspaces. Shows the checkout path,
  branch, and `git status --short` summary in a popup, asks for confirmation, then runs
  `momo worktree remove --workspace <id>` (never `--force` without a second explicit confirmation)
  and closes the workspace. Never deletes the branch.
- Config `$HERDR_PLUGIN_CONFIG_DIR/config.toml`: `layout = [...]` recipe, `start_command` (for example
  an agent launch), default base ref.
- Suggested bindings documented in the README (`[[keys.command]] type = "plugin_action"`), not set by
  the plugin.

## Classification

Plugin. Everything maps to existing CLI (`worktree create|remove`, `pane split`, `pane run`,
`workspace close`) plus a popup pane for input.

## Safety

- Finish must refuse on the primary (non-linked) workspace.
- Dirty checkout: show the dirty files and require an explicit second confirmation before `--force`.
- Read ids from context JSON and command output only.

## Code/doc map

- CLI shapes: `docs/next/website/src/content/docs/cli-reference.mdx` "Worktrees", "Panes".
- Popup panes: `plugins.mdx` "Panes" (popup gets all input, closes when the command exits).
- Context JSON fields (worktree info): `plugins.mdx` "Commands and environment".

## Tests

- Script-level tests for branch-name validation and recipe parsing.
- Smoke in a throwaway session against a scratch git repo in `/var/tmp` (never the user's repos):
  start → workspace exists with the layout; finish on clean checkout removes it; finish on dirty
  checkout requires a second confirmation; finish on the primary workspace refuses.

## Acceptance

README with install, config, bindings example, and the safety rules above.
