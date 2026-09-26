import hashlib
import os
import platform
import stat
import subprocess
import tempfile
import unittest
from pathlib import Path

INSTALLER = Path(__file__).resolve().parents[1] / "install.sh"


def asset_name():
    system = {"Darwin": "macos", "Linux": "linux"}[platform.system()]
    machine = platform.machine()
    arch = "aarch64" if machine in ("arm64", "aarch64") else "x86_64"
    return f"momo-{system}-{arch}"


class InstallerTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        root = Path(self.tmp.name)
        self.release = root / "release"
        self.release.mkdir()
        self.bin_dir = root / "bin"
        binary = self.release / asset_name()
        binary.write_text("#!/bin/sh\necho 'momo 0.9.1-momo.7'\n")
        self.digest = hashlib.sha256(binary.read_bytes()).hexdigest()

    def tearDown(self):
        self.tmp.cleanup()

    def write_sums(self, digest):
        (self.release / "SHA256SUMS").write_text(
            f"{'0' * 64}  momo-other\n{digest}  {asset_name()}\n"
        )

    def run_installer(self):
        env = dict(os.environ)
        env.update(
            MOMO_DOWNLOAD_BASE=self.release.as_uri(),
            MOMO_INSTALL_DIR=str(self.bin_dir),
            PATH=os.environ.get("PATH", "/usr/bin:/bin"),
        )
        return subprocess.run(
            ["sh", str(INSTALLER)], env=env, capture_output=True, text=True, timeout=60
        )

    def test_installs_the_verified_binary_as_momo(self):
        self.write_sums(self.digest)
        result = self.run_installer()
        self.assertEqual(result.returncode, 0, result.stderr)
        installed = self.bin_dir / "momo"
        self.assertTrue(installed.stat().st_mode & stat.S_IXUSR)
        self.assertIn("installed momo 0.9.1-momo.7", result.stdout)
        self.assertIn("is not on your PATH", result.stdout)
        self.assertEqual(sorted(p.name for p in self.bin_dir.iterdir()), ["momo"])

    def test_checksum_mismatch_installs_nothing(self):
        self.write_sums("f" * 64)
        result = self.run_installer()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("checksum mismatch", result.stderr)
        self.assertFalse((self.bin_dir / "momo").exists())

    def test_missing_checksum_entry_installs_nothing(self):
        (self.release / "SHA256SUMS").write_text(f"{self.digest}  momo-other\n")
        result = self.run_installer()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("no valid checksum", result.stderr)
        self.assertFalse((self.bin_dir / "momo").exists())


if __name__ == "__main__":
    unittest.main()
