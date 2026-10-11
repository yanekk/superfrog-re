"""tools/setup/install-brew-deps.sh against a stub brew: installs only what is missing."""
import os
import subprocess
import tempfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).resolve().parent / "setup" / "install-brew-deps.sh"
STUB = """#!/bin/sh
echo "$*" >> "$BREWLOG"
case "$1" in
  list) [ -n "$HAVE" ] && { echo "cmake 9.9"; exit 0; }; exit 1 ;;
  install) exit 0 ;;
esac
"""


class InstallBrewDeps(unittest.TestCase):
    def run_script(self, have, with_brew=True):
        with tempfile.TemporaryDirectory() as tmp:
            bin_dir = Path(tmp, "bin")
            bin_dir.mkdir()
            if with_brew:
                stub = bin_dir / "brew"
                stub.write_text(STUB)
                stub.chmod(0o755)
            log = Path(tmp, "log")
            log.touch()
            env = {"PATH": f"{bin_dir}:/usr/bin:/bin", "BREWLOG": str(log), "HAVE": "1" if have else ""}
            proc = subprocess.run(["/bin/sh", str(SCRIPT)], env=env, capture_output=True, text=True)
            return proc.returncode, log.read_text().splitlines()

    def test_installed_package_is_left_alone(self):
        rc, calls = self.run_script(have=True)
        self.assertEqual(rc, 0)
        self.assertNotIn("install cmake", calls)
        self.assertIn("list --versions cmake", calls)

    def test_missing_package_is_installed(self):
        rc, calls = self.run_script(have=False)
        self.assertEqual(rc, 0)
        self.assertEqual(calls, ["list --versions cmake", "install cmake"])

    def test_no_homebrew_fails(self):
        rc, calls = self.run_script(have=False, with_brew=False)
        self.assertEqual(rc, 1)
        self.assertEqual(calls, [])


if __name__ == "__main__":
    unittest.main()
