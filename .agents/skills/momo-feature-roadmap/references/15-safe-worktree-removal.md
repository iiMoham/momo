# 15 safe-worktree-removal

Source: upstream discussion #4513 "Multi-repo workspaces" (Option B: the smaller core surface that
lets plugins do the rest, plus the data-loss problem it describes).

## Problem

`worktree remove` asks Git to remove the checkout and only escalates to `--force` after Git refuses a
dirty checkout. If a worktree contains nested repositories (sub-repo checkouts gitignored by the
hub), Git sees a clean hub checkout and deletes them, including their uncommitted work, with no
warning. `worktree.removed` fires after the directory is gone, so a plugin cannot intervene.

## Behavior

1. **Nested-repo safety check (core).** Before removing, scan the checkout (bounded depth, skip
   `.git`, `node_modules`, `target`) for nested Git repositories / worktrees. If any has uncommitted
   changes, untracked files, stashes, or unpushed commits, refuse with `worktree_nested_dirty`,
   listing each path and what is at risk. `--force` alone does not bypass it; a new explicit
   `--discard-nested` flag (and TUI second confirmation) does.
2. **Pre-removal hook (core primitive for plugins).** New plugin hook event `worktree.removing`
   (synchronous, before deletion). A hook exiting non-zero vetoes the removal with its stderr as the
   reason. Timeout-bounded; document the veto contract.
3. TUI `Delete worktree checkout...` shows the nested findings in its confirmation dialog.

## Classification

Server/runtime (worktree operations run on the server) + TUI dialog text.

## Stop-and-ask checkpoint

- A synchronous, vetoing plugin hook is new plugin semantics (today hooks are fire-and-forget).
  Confirm the design, the timeout, and whether it belongs in this feature or a follow-up.
- The scan does filesystem I/O; it must run off the server's main loop (worker thread / async task)
  and never in render or detection paths.

## Code map (at d11c0c34)

- `src/worktree.rs`: `build_worktree_remove_command`, `is_dirty_worktree_remove_error`,
  `run_worktree_remove_command_with_recovery` and their tests (~line 800+).
- API/handlers: `src/app/api/worktrees.rs`, `src/app/api/worktrees/`, `src/app/worktrees.rs`.
- CLI: `src/cli/worktree.rs`. TUI dialogs: `src/client/shell/worktree_overlays.rs`.
- Plugin hooks: `src/api/schema/events.rs::PLUGIN_HOOK_EVENT_KINDS`, `src/app/api/plugins/`.

## Tests

- Scan: temp dirs with nested repos in each risky state ⇒ findings; clean nested repo ⇒ none;
  depth and skip-list respected; symlink loops do not hang.
- Removal refused with findings; `--discard-nested` proceeds; plain `--force` does not.
- Hook veto (if approved): non-zero exit blocks removal, timeout behavior, stderr surfaced.
- All Git fixtures in temp dirs; never the user's repositories.
