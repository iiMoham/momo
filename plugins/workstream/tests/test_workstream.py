"""Unit tests for the workstream plugin's pure logic.

Run with `just fork-plugins-test` or `python3 -m unittest discover -s plugins/workstream/tests`.
"""

import os
import subprocess
import sys
import tempfile
import unittest

sys.path.insert(0, os.path.join(os.path.dirname(__file__), os.pardir))

import workstream


class ConfigTests(unittest.TestCase):
    def test_missing_or_empty_config_uses_defaults(self):
        for text in (None, "", "  \n"):
            config = workstream.parse_config(text)
            self.assertEqual(
                config,
                {"branch_prefix": "", "base": "", "root_command": "", "splits": []},
            )

    def test_full_config_is_parsed_and_split_direction_defaults_to_right(self):
        config = workstream.parse_config(
            '{"branch_prefix": "feat/", "base": "origin/main", "root_command": "claude",'
            ' "splits": [{"command": "npm run dev"}, {"direction": "down", "command": ""}]}'
        )
        self.assertEqual(config["branch_prefix"], "feat/")
        self.assertEqual(config["base"], "origin/main")
        self.assertEqual(
            config["splits"],
            [
                {"direction": "right", "command": "npm run dev"},
                {"direction": "down", "command": ""},
            ],
        )

    def test_invalid_config_is_rejected_with_a_readable_message(self):
        for text, fragment in (
            ("{", "not valid JSON"),
            ("[]", "JSON object"),
            ('{"base": 3}', "base must be a string"),
            ('{"splits": {}}', "splits must be a list"),
            ('{"splits": ["x"]}', "splits[0] must be an object"),
            ('{"splits": [{"direction": "left"}]}', "right or down"),
            ('{"splits": [{"command": 1}]}', "splits[0].command"),
            ('{"layout": []}', "unknown config keys: layout"),
        ):
            with self.assertRaises(workstream.ConfigError) as raised:
                workstream.parse_config(text)
            self.assertIn(fragment, str(raised.exception), text)


class BranchTests(unittest.TestCase):
    def test_prefix_is_added_once(self):
        self.assertEqual(workstream.full_branch_name("feat/", "login"), "feat/login")
        self.assertEqual(workstream.full_branch_name("feat/", "feat/login"), "feat/login")
        self.assertEqual(workstream.full_branch_name("", " login "), "login")
        self.assertEqual(workstream.full_branch_name("feat/", "  "), "")

    def test_branch_names_are_checked_by_git(self):
        with tempfile.TemporaryDirectory() as repo:
            subprocess.run(["git", "init", "--quiet", repo], check=True)
            self.assertTrue(workstream.valid_branch("feat/login", repo))
            for invalid in ("bad name", "a..b", "trailing.", "-leading", "x~y"):
                self.assertFalse(workstream.valid_branch(invalid, repo), invalid)


class CommandTests(unittest.TestCase):
    def test_worktree_create_focuses_and_passes_the_base_only_when_set(self):
        self.assertEqual(
            workstream.worktree_create_args("w1", "feat/x", ""),
            ["worktree", "create", "--workspace", "w1", "--branch", "feat/x", "--focus"],
        )
        self.assertEqual(
            workstream.worktree_create_args("w1", "feat/x", "origin/main")[-2:],
            ["--base", "origin/main"],
        )

    def test_layout_runs_the_root_command_before_splits(self):
        config = workstream.parse_config(
            '{"root_command": "claude", "splits": [{"command": "npm run dev"}]}'
        )
        self.assertEqual(
            workstream.layout_steps(config),
            [
                ("run_root", "claude"),
                ("split", {"direction": "right", "command": "npm run dev"}),
            ],
        )
        self.assertEqual(workstream.layout_steps(workstream.parse_config(None)), [])


class FinishTests(unittest.TestCase):
    def test_finish_only_runs_on_linked_worktrees(self):
        linked = {"workspace_id": "w2", "worktree": {"is_linked_worktree": True}}
        self.assertIsNone(workstream.finish_blocker(linked))
        self.assertIn(
            "main checkout",
            workstream.finish_blocker(
                {"workspace_id": "w1", "worktree": {"is_linked_worktree": False}}
            ),
        )
        self.assertIn("not a Herdr worktree", workstream.finish_blocker({"workspace_id": "w1"}))
        self.assertIn("no workspace", workstream.finish_blocker({}))

    def test_popup_drops_cli_only_hints_from_refusals(self):
        message = (
            "refusing to remove /w/task: 1 nested repository has work that would be lost:\n"
            "  frontend: 1 unpushed commit\n"
            "remove it with --discard-nested to delete this work anyway"
        )
        self.assertEqual(
            workstream.popup_message(message),
            "refusing to remove /w/task: 1 nested repository has work that would be lost:\n"
            "  frontend: 1 unpushed commit",
        )
        self.assertEqual(workstream.popup_message("plain failure"), "plain failure")

    def test_each_refusal_needs_its_own_confirmation_word(self):
        self.assertEqual(
            workstream.escalation_for("dirty_worktree_requires_force"), ("force", "--force")
        )
        for code in workstream.NESTED_REFUSALS:
            self.assertEqual(workstream.escalation_for(code), ("discard", "--discard-nested"))
        for code in (None, "worktree_remove_failed", "workspace_not_found"):
            self.assertIsNone(workstream.escalation_for(code))


class ContextTests(unittest.TestCase):
    def test_popup_context_wins_over_the_plugin_context(self):
        env = {
            "WORKSTREAM_CONTEXT_JSON": '{"workspace_id": "w2"}',
            "HERDR_PLUGIN_CONTEXT_JSON": '{"workspace_id": "w9"}',
        }
        old = {key: os.environ.get(key) for key in env}
        os.environ.update(env)
        try:
            self.assertEqual(workstream.load_context()["workspace_id"], "w2")
            os.environ["WORKSTREAM_CONTEXT_JSON"] = "not json"
            self.assertEqual(workstream.load_context()["workspace_id"], "w9")
        finally:
            for key, value in old.items():
                if value is None:
                    os.environ.pop(key, None)
                else:
                    os.environ[key] = value


if __name__ == "__main__":
    unittest.main()
