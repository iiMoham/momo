import hashlib
import json
import sys
import tempfile
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
sys.path.insert(0, str(HERE.parent))

import release_manifest  # noqa: E402


class ReleaseManifestTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.dir = Path(self.tmp.name)
        self.version = release_manifest.cargo_version(ROOT)
        for target in release_manifest.TARGETS:
            (self.dir / f"momo-{target}").write_bytes(f"binary {target}".encode())

    def tearDown(self):
        self.tmp.cleanup()

    def build(self, release=2, previous=None, notes="### Added\n- things\n"):
        return release_manifest.build(
            tag=f"momo-v{self.version}-momo.{release}",
            repo="iiMoham/momo",
            assets_dir=self.dir,
            notes=notes,
            source_root=ROOT,
            previous=previous,
        )

    def test_manifest_matches_the_updater_and_remote_install_shape(self):
        manifest, sums = self.build()
        label = f"{self.version}-momo.2"
        self.assertEqual(manifest["version"], self.version)
        self.assertEqual(manifest["momo_release"], 2)
        self.assertEqual(manifest["endpoint_generation"], 1)
        self.assertIsInstance(manifest["protocol"], int)
        self.assertEqual(manifest["notes"], "### Added\n- things")
        asset = manifest["assets"]["macos-aarch64"]
        self.assertEqual(
            asset["url"],
            f"https://github.com/iiMoham/momo/releases/download/momo-v{label}/momo-macos-aarch64",
        )
        self.assertEqual(
            asset["sha256"], hashlib.sha256(b"binary macos-aarch64").hexdigest()
        )
        self.assertEqual(manifest["releases"][label]["assets"], manifest["assets"])
        self.assertIn(f"{asset['sha256']}  momo-macos-aarch64\n", sums)
        self.assertEqual(len(sums.splitlines()), 4)
        json.dumps(manifest)

    def test_previous_releases_are_kept_for_older_clients_newest_first(self):
        first, _ = self.build(release=1)
        second, _ = self.build(release=2, previous=first)
        self.assertEqual(
            list(second["releases"]),
            [f"{self.version}-momo.2", f"{self.version}-momo.1"],
        )
        many = {"releases": {f"old-{i}": {} for i in range(50)}}
        capped, _ = self.build(release=3, previous=many)
        self.assertEqual(len(capped["releases"]), release_manifest.KEEP_RELEASES)

    def test_bad_tags_versions_and_missing_assets_are_rejected(self):
        for tag in ["v0.9.1", "momo-v0.9.1", "momo-v0.9.1-momo.0", "momo-0.9.1-momo.1"]:
            with self.assertRaises(release_manifest.ReleaseError, msg=tag):
                release_manifest.parse_tag(tag)
        with self.assertRaises(release_manifest.ReleaseError):
            release_manifest.build(
                tag="momo-v99.0.0-momo.1",
                repo="iiMoham/momo",
                assets_dir=self.dir,
                notes="",
                source_root=ROOT,
                previous=None,
            )
        (self.dir / "momo-linux-aarch64").unlink()
        with self.assertRaises(release_manifest.ReleaseError):
            self.build()

    def test_empty_notes_get_a_default_so_the_updater_accepts_them(self):
        manifest, _ = self.build(notes="  \n")
        self.assertEqual(manifest["notes"], f"MoMo {self.version}-momo.2")


if __name__ == "__main__":
    unittest.main()
