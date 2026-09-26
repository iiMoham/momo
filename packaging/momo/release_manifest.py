#!/usr/bin/env python3
"""Build MoMo's release files: `latest.json` and `SHA256SUMS`.

`latest.json` uses herdr's update-manifest shape so `momo update` and remote
SSH installs read it unchanged:

- top level: `version` (upstream base), `momo_release` (N), `protocol`,
  `endpoint_generation`, `notes`, and `assets` as {url, sha256} objects;
- `releases`: every MoMo release by label (`0.9.1-momo.N`), so a remote host
  can always get the exact build its client runs. Entries from the previous
  manifest are kept, newest first, up to KEEP_RELEASES.

Usage (from the release workflow):
  release_manifest.py --tag momo-v0.9.1-momo.2 --repo iiMoham/momo \
      --assets-dir dist --notes-file notes.md --out-dir dist \
      [--previous previous-latest.json] [--source-root .]
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from pathlib import Path

TARGETS = ("linux-x86_64", "linux-aarch64", "macos-x86_64", "macos-aarch64")
KEEP_RELEASES = 20
TAG = re.compile(r"^momo-v(?P<base>\d+\.\d+\.\d+)-momo\.(?P<release>[1-9]\d*)$")


class ReleaseError(Exception):
    pass


def parse_tag(tag: str) -> tuple[str, int]:
    match = TAG.match(tag)
    if not match:
        raise ReleaseError(f"tag {tag!r} is not momo-v<base>-momo.<N>, e.g. momo-v0.9.1-momo.1")
    return match["base"], int(match["release"])


def read_const(path: Path, name: str) -> int:
    match = re.search(rf"pub const {name}: u32 = (\d+);", path.read_text())
    if not match:
        raise ReleaseError(f"{name} not found in {path}")
    return int(match.group(1))


def cargo_version(root: Path) -> str:
    match = re.search(r'^version = "([^"]+)"', (root / "Cargo.toml").read_text(), re.M)
    if not match:
        raise ReleaseError("version not found in Cargo.toml")
    return match.group(1)


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def build(
    *,
    tag: str,
    repo: str,
    assets_dir: Path,
    notes: str,
    source_root: Path,
    previous: dict | None,
) -> tuple[dict, str]:
    base, release = parse_tag(tag)
    if base != cargo_version(source_root):
        raise ReleaseError(
            f"tag base {base} does not match Cargo.toml version {cargo_version(source_root)}"
        )
    notes = notes.strip() or f"MoMo {base}-momo.{release}"
    protocol = read_const(source_root / "src/protocol/wire.rs", "PROTOCOL_VERSION")
    generation = read_const(
        source_root / "src/protocol/endpoint.rs", "ENDPOINT_PROTOCOL_GENERATION"
    )

    assets: dict[str, dict[str, str]] = {}
    sums = []
    for target in TARGETS:
        name = f"momo-{target}"
        path = assets_dir / name
        if not path.is_file():
            raise ReleaseError(f"missing release asset {path}")
        digest = sha256(path)
        assets[target] = {
            "url": f"https://github.com/{repo}/releases/download/{tag}/{name}",
            "sha256": digest,
        }
        sums.append(f"{digest}  {name}")

    label = f"{base}-momo.{release}"
    entry = {
        "notes": notes,
        "protocol": protocol,
        "endpoint_generation": generation,
        "assets": assets,
    }
    releases = {label: entry}
    for key, value in (previous or {}).get("releases", {}).items():
        if key != label and len(releases) < KEEP_RELEASES:
            releases[key] = value

    manifest = {
        "version": base,
        "momo_release": release,
        "protocol": protocol,
        "endpoint_generation": generation,
        "notes": notes,
        "assets": assets,
        "releases": releases,
    }
    return manifest, "\n".join(sums) + "\n"


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--tag", required=True)
    parser.add_argument("--repo", required=True)
    parser.add_argument("--assets-dir", type=Path, required=True)
    parser.add_argument("--notes-file", type=Path)
    parser.add_argument("--previous", type=Path)
    parser.add_argument("--source-root", type=Path, default=Path("."))
    parser.add_argument("--out-dir", type=Path, required=True)
    args = parser.parse_args(argv)

    previous = None
    if args.previous and args.previous.is_file():
        try:
            previous = json.loads(args.previous.read_text())
        except json.JSONDecodeError:
            previous = None
    notes = args.notes_file.read_text() if args.notes_file else ""
    try:
        manifest, sums = build(
            tag=args.tag,
            repo=args.repo,
            assets_dir=args.assets_dir,
            notes=notes,
            source_root=args.source_root,
            previous=previous,
        )
    except ReleaseError as err:
        print(f"error: {err}", file=sys.stderr)
        return 1
    args.out_dir.mkdir(parents=True, exist_ok=True)
    (args.out_dir / "latest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    (args.out_dir / "SHA256SUMS").write_text(sums)
    print(f"wrote latest.json and SHA256SUMS for {manifest['version']}-momo.{manifest['momo_release']}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
