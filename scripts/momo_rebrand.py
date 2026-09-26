#!/usr/bin/env python3
"""Rebrand user-facing herdr text to MoMo inside Rust string literals.

Idempotent: rerun it after merging upstream herdr (take upstream's side of a
conflicting string, then run this script) to reapply the MoMo names.

Only prose and command examples change. Identifiers stay: `HERDR_*`
variables, hook sources (`herdr:claude`), file and socket names
(`herdr-agent-state.sh`, `herdr.sock`), config values (`delivery = "herdr"`),
and `Herdr-managed` markers, so agent hooks, plugins, and the wire protocol
remain compatible with upstream herdr.

Usage: scripts/momo_rebrand.py [--check] [paths...]   (default: src tests)
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DEFAULT_PATHS = ["src", "tests"]

# Files whose herdr strings are compatibility identifiers or are handled by
# hand (update and remote install move to MoMo's own releases separately).
SKIP_FILES = {
    "src/build_info.rs",
    "src/brand.rs",
}
# Files inside SKIP_DIRS whose strings are only user-facing advice (which
# command to run), with no compatibility markers.
INCLUDE_FILES = {
    "src/integration/actions.rs",
    "src/integration/registry.rs",
    "src/integration/version.rs",
}
SKIP_DIRS = (
    "src/detect/manifests/",
    "src/integration/",
    "src/api/schema",
    "src/protocol/",
    "tests/fixtures/",
)

STRING = re.compile(r'r(#*)"(?:.|\n)*?"\1|"(?:[^"\\\n]|\\.|\\\n)*"')

# `herdr` used as a command or product word: followed by a space and a word,
# an option, or a placeholder; or quoted in backticks.
# `\n`/`\t` escapes count as boundaries: "\nherdr pane" is a command too.
BOUNDARY = r"(?:(?<![\w:/.$-])|(?<=\\n)|(?<=\\t))"
COMMAND = re.compile(BOUNDARY + r"herdr(?= (?:[a-z<\[{(`'\"-]|\\\"))")
BACKTICK = re.compile(r"`herdr`")
TRAILING_PROSE = re.compile(
    r"\b(open|in|inside|restart|start|quit|exit|from|to|run|of|with|by|use|using|launch) herdr(?=[ .,;:!?)]|$|\\n)"
)
# Message prefixes (`herdr: failed …`) and the help header (`herdr — …`).
# Hook sources like `herdr:claude` have no space after the colon and stay.
MESSAGE_PREFIX = re.compile(r'^"herdr(?=: | — )')
PRODUCT = re.compile(r"(?:(?<![\w-])|(?<=\\n)|(?<=\\t))Herdr(?![\w-])")
PRODUCT_POSSESSIVE = re.compile(r"(?<![\w-])Herdr's")


def rebrand_literal(literal: str) -> str:
    if "HERDR_" in literal and "herdr " not in literal and "Herdr" not in literal:
        return literal
    literal = MESSAGE_PREFIX.sub('"momo', literal)
    literal = literal.replace("Herdr-managed", "MoMo-managed")
    literal = PRODUCT_POSSESSIVE.sub("MoMo's", literal)
    literal = PRODUCT.sub("MoMo", literal)
    literal = BACKTICK.sub("`momo`", literal)
    literal = COMMAND.sub("momo", literal)
    literal = TRAILING_PROSE.sub(r"\1 momo", literal)
    return literal


def rebrand_source(source: str) -> str:
    out = []
    last = 0
    for match in STRING.finditer(source):
        start, end = match.span()
        # Skip char literals and lifetimes that look like strings are not a
        # concern for `"`; skip strings inside line comments.
        line_start = source.rfind("\n", 0, start) + 1
        prefix = source[line_start:start]
        if "//" in prefix and prefix.count('"') % 2 == 0:
            continue
        out.append(source[last:start])
        out.append(rebrand_literal(match.group(0)))
        last = end
    out.append(source[last:])
    return "".join(out)


def rust_files(paths: list[str]):
    for raw in paths:
        path = ROOT / raw
        files = [path] if path.is_file() else sorted(path.rglob("*.rs"))
        for file in files:
            rel = file.relative_to(ROOT).as_posix()
            if rel in SKIP_FILES or (rel.startswith(SKIP_DIRS) and rel not in INCLUDE_FILES):
                continue
            yield file, rel


def main(argv: list[str]) -> int:
    check = "--check" in argv
    paths = [arg for arg in argv if arg != "--check"] or DEFAULT_PATHS
    changed = []
    for file, rel in rust_files(paths):
        before = file.read_text()
        after = rebrand_source(before)
        if after != before:
            changed.append(rel)
            if not check:
                file.write_text(after)
    for rel in changed:
        print(("needs rebrand: " if check else "rebranded: ") + rel)
    return 1 if check and changed else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
