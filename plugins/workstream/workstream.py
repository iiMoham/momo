#!/usr/bin/env python3
"""Workstream plugin for MoMo: start a task in a new worktree, finish it safely.

Actions have no terminal, so `launch <start|finish>` opens this plugin's popup
entrypoint, and `start` / `finish` run inside that popup where they can ask
questions. Everything else goes through the momo CLI at $HERDR_BIN_PATH.

Standard library only, and compatible with Python 3.9 (macOS system python).
"""

import json
import os
import subprocess
import sys

PLUGIN_ID = "momo.workstream"
VALID_DIRECTIONS = ("right", "down")
STATUS_PREVIEW_LINES = 10
NESTED_REFUSALS = (
    "worktree_nested_repositories_at_risk",
    "worktree_removal_check_incomplete",
)


class ConfigError(Exception):
    pass


# ---------------------------------------------------------------------------
# Pure logic (unit tested)
# ---------------------------------------------------------------------------


def parse_config(text):
    """Validate config.json text; None or empty means all defaults."""
    config = {"branch_prefix": "", "base": "", "root_command": "", "splits": []}
    if text is None or not text.strip():
        return config
    try:
        raw = json.loads(text)
    except ValueError as error:
        raise ConfigError(f"config.json is not valid JSON: {error}") from error
    if not isinstance(raw, dict):
        raise ConfigError("config.json must be a JSON object")
    for key in ("branch_prefix", "base", "root_command"):
        if key in raw:
            if not isinstance(raw[key], str):
                raise ConfigError(f"{key} must be a string")
            config[key] = raw[key]
    if "splits" in raw:
        splits = raw["splits"]
        if not isinstance(splits, list):
            raise ConfigError("splits must be a list")
        for index, split in enumerate(splits):
            if not isinstance(split, dict):
                raise ConfigError(f"splits[{index}] must be an object")
            direction = split.get("direction", "right")
            if direction not in VALID_DIRECTIONS:
                raise ConfigError(f"splits[{index}].direction must be right or down")
            command = split.get("command", "")
            if not isinstance(command, str):
                raise ConfigError(f"splits[{index}].command must be a string")
            config["splits"].append({"direction": direction, "command": command})
    unknown = sorted(set(raw) - {"branch_prefix", "base", "root_command", "splits"})
    if unknown:
        raise ConfigError("unknown config keys: {}".format(", ".join(unknown)))
    return config


def full_branch_name(prefix, name):
    name = name.strip()
    if not name or not prefix or name.startswith(prefix):
        return name
    return prefix + name


def worktree_create_args(workspace_id, branch, base):
    args = ["worktree", "create", "--workspace", workspace_id, "--branch", branch, "--focus"]
    if base:
        args += ["--base", base]
    return args


def layout_steps(config):
    """Ordered (kind, value) steps applied to the new workspace's root pane."""
    steps = []
    if config["root_command"]:
        steps.append(("run_root", config["root_command"]))
    for split in config["splits"]:
        steps.append(("split", split))
    return steps


def finish_blocker(context):
    """Why finish must not run for this invocation context, or None."""
    worktree = context.get("worktree")
    if not context.get("workspace_id"):
        return "no workspace in the invocation context"
    if not worktree:
        return "this workspace is not a Herdr worktree"
    if not worktree.get("is_linked_worktree"):
        return "this is the repository's main checkout; finish only removes linked worktrees"
    return None


def escalation_for(error_code):
    """Extra confirmation (word, flag) a refused removal may be retried with."""
    if error_code == "dirty_worktree_requires_force":
        return ("force", "--force")
    if error_code in NESTED_REFUSALS:
        return ("discard", "--discard-nested")
    return None


def popup_message(message):
    """Server refusal text without its CLI-only hint (the popup asks for a word instead)."""
    lines = [
        line
        for line in message.splitlines()
        if "--discard-nested" not in line and "--force" not in line
    ]
    return "\n".join(lines).strip()


def parse_json(text):
    try:
        return json.loads(text)
    except ValueError:
        return None


# ---------------------------------------------------------------------------
# momo / git helpers
# ---------------------------------------------------------------------------


def momo_cli(args):
    """Run the momo CLI; returns (ok, parsed JSON or None, raw text)."""
    binary = os.environ.get("HERDR_BIN_PATH") or "momo"
    completed = subprocess.run([binary, *args], capture_output=True, text=True)
    raw = completed.stdout if completed.returncode == 0 else completed.stderr
    return (
        completed.returncode == 0,
        parse_json(raw.strip().splitlines()[-1] if raw.strip() else ""),
        raw,
    )


def error_of(payload, raw):
    if isinstance(payload, dict) and isinstance(payload.get("error"), dict):
        return payload["error"].get("code"), payload["error"].get("message", raw.strip())
    return None, raw.strip() or "momo command failed"


def valid_branch(name, cwd):
    completed = subprocess.run(
        ["git", "check-ref-format", "--branch", name],
        cwd=cwd or None,
        capture_output=True,
    )
    return completed.returncode == 0


def load_context():
    for key in ("WORKSTREAM_CONTEXT_JSON", "HERDR_PLUGIN_CONTEXT_JSON"):
        context = parse_json(os.environ.get(key, ""))
        if isinstance(context, dict):
            return context
    return {}


def load_config():
    config_dir = os.environ.get("HERDR_PLUGIN_CONFIG_DIR", "")
    path = os.path.join(config_dir, "config.json") if config_dir else ""
    text = None
    if path and os.path.exists(path):
        with open(path, encoding="utf-8") as handle:
            text = handle.read()
    return parse_config(text)


# ---------------------------------------------------------------------------
# Interactive popups
# ---------------------------------------------------------------------------


def say(line=""):
    print(line, flush=True)


def ask(prompt):
    try:
        return input(prompt)
    except EOFError:
        return ""


def fail(message):
    say("")
    say("error: " + message)
    ask("press Enter to close ")
    return 1


def launch(which):
    """Action entry point: open the interactive popup with the invocation context."""
    context = os.environ.get("HERDR_PLUGIN_CONTEXT_JSON", "{}")
    ok, payload, raw = momo_cli(
        [
            "plugin",
            "pane",
            "open",
            "--plugin",
            PLUGIN_ID,
            "--entrypoint",
            which + "-ui",
            "--placement",
            "popup",
            "--env",
            "WORKSTREAM_CONTEXT_JSON=" + context,
        ]
    )
    if not ok:
        sys.stderr.write(error_of(payload, raw)[1] + "\n")
        return 1
    return 0


def start():
    context = load_context()
    try:
        config = load_config()
    except ConfigError as error:
        return fail(str(error))
    workspace_id = context.get("workspace_id")
    if not workspace_id:
        return fail("no workspace in the invocation context")
    worktree = context.get("worktree") or {}
    repo_cwd = worktree.get("repo_root") or context.get("workspace_cwd")

    say("Start workstream in {}".format(context.get("workspace_label") or workspace_id))
    if config["branch_prefix"]:
        say("Branch prefix: {}".format(config["branch_prefix"]))
    name = ask("Branch name (empty cancels): ").strip()
    if not name:
        return 0
    branch = full_branch_name(config["branch_prefix"], name)
    if not valid_branch(branch, repo_cwd):
        return fail(f"'{branch}' is not a valid branch name")

    say(f"Creating worktree for {branch} ...")
    ok, payload, raw = momo_cli(worktree_create_args(workspace_id, branch, config["base"]))
    if not ok:
        return fail(error_of(payload, raw)[1])
    root_pane = ((payload or {}).get("result") or {}).get("root_pane") or {}
    root_id = root_pane.get("pane_id")
    if not root_id:
        return fail("MoMo did not report the new workspace's pane")

    for kind, value in layout_steps(config):
        if kind == "run_root":
            ok, payload, raw = momo_cli(["pane", "run", root_id, value])
        else:
            ok, payload, raw = momo_cli(
                ["pane", "split", root_id, "--direction", value["direction"], "--no-focus"]
            )
            new_pane = (((payload or {}).get("result") or {}).get("pane") or {}).get("pane_id")
            if ok and value["command"] and new_pane:
                ok, payload, raw = momo_cli(["pane", "run", new_pane, value["command"]])
        if not ok:
            return fail("layout step failed: " + error_of(payload, raw)[1])
    return 0


def git_status_preview(checkout):
    completed = subprocess.run(
        ["git", "-C", checkout, "status", "--short"],
        capture_output=True,
        text=True,
    )
    lines = [line for line in completed.stdout.splitlines() if line.strip()]
    return lines


def finish():
    context = load_context()
    blocker = finish_blocker(context)
    if blocker:
        return fail(blocker)
    workspace_id = context["workspace_id"]
    worktree = context["worktree"]
    checkout = worktree.get("checkout_path", "")

    say("Finish workstream: {}".format(context.get("workspace_label") or workspace_id))
    say(f"Checkout: {checkout}")
    changes = git_status_preview(checkout)
    if changes:
        say(f"Uncommitted changes ({len(changes)}):")
        for line in changes[:STATUS_PREVIEW_LINES]:
            say("  " + line)
        if len(changes) > STATUS_PREVIEW_LINES:
            say(f"  ...and {len(changes) - STATUS_PREVIEW_LINES} more")
    else:
        say("No uncommitted changes in the checkout.")
    ok, payload, raw = momo_cli(["worktree", "removal-check", "--workspace", workspace_id])
    nested = (
        ((((payload or {}).get("result") or {}).get("check") or {}).get("nested")) if ok else None
    )
    if nested:
        say("Nested repositories with work that would be lost:")
        for repository in nested:
            say("  {}: {}".format(repository.get("relative_path"), repository.get("summary")))
    say("The branch is kept; only the checkout folder and its workspace are removed.")
    if ask("Remove this checkout? [y/N] ").strip().lower() not in ("y", "yes"):
        return 0

    flags = []
    while True:
        ok, payload, raw = momo_cli(["worktree", "remove", "--workspace", workspace_id, *flags])
        if ok:
            return 0
        code, message = error_of(payload, raw)
        escalation = escalation_for(code)
        if escalation is None or escalation[1] in flags:
            return fail(message)
        word, flag = escalation
        say("")
        say(popup_message(message))
        if ask(f"Type '{word}' to delete this work anyway: ").strip() != word:
            return 0
        flags.append(flag)


def main(argv):
    if len(argv) >= 3 and argv[1] == "launch" and argv[2] in ("start", "finish"):
        return launch(argv[2])
    if len(argv) == 2 and argv[1] == "start":
        return start()
    if len(argv) == 2 and argv[1] == "finish":
        return finish()
    sys.stderr.write("usage: workstream.py launch <start|finish> | start | finish\n")
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv))
