"""tools/setup/oracle.sh's cache and lock logic, against a scratch SUPERFROG_CACHE (no network, no build).

The real build (fetch, patch, cmake) is exercised by the test block's setup; these cover what a
parallel run leans on: a finished build is a no-op, a live builder is waited for, a dead one's lock
is taken over, and the build lands where sf-oracle's build.rs looks (same patch-set hash)."""
import hashlib
import os
import subprocess
import tempfile
import threading
import time
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SCRIPT = ROOT / "tools" / "setup" / "oracle.sh"
ORACLE = ROOT / "engine" / "crates" / "sf-oracle"


def build_dir(cache):
    """Where build.rs expects the build: <cache>/vamiga/<commit>/<sha1 of patches in name order>[:12]."""
    commit = (ORACLE / "VAMIGA_COMMIT").read_text().strip()
    patches = sorted((ORACLE / "patches").glob("*.patch"))
    phash = hashlib.sha1(b"".join(p.read_bytes() for p in patches)).hexdigest()[:12]
    return Path(cache, "vamiga", commit, phash)


class OracleSh(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.cache = self.tmp.name
        self.dest = build_dir(self.cache)
        self.dest.parent.mkdir(parents=True)

    def tearDown(self):
        self.tmp.cleanup()

    def run_script(self, **extra):
        env = dict(os.environ, SUPERFROG_CACHE=self.cache, **extra)
        return subprocess.run(["/bin/sh", str(SCRIPT)], env=env, capture_output=True, text=True, timeout=120)

    def test_a_finished_build_is_a_no_op(self):
        self.dest.mkdir()
        (self.dest / ".done").write_text("x\n")
        proc = self.run_script(VAMIGA_URL="/nonexistent")
        self.assertEqual(proc.returncode, 0, proc.stderr)
        self.assertIn("already built", proc.stdout)
        self.assertIn(str(self.dest), proc.stdout)
        self.assertEqual(sorted(p.name for p in self.dest.parent.iterdir()), [self.dest.name])

    def test_a_live_builder_is_waited_for(self):
        lock = Path(f"{self.dest}.lock")
        lock.mkdir()
        (lock / "pid").write_text(f"{os.getpid()}\n")  # this test process: alive

        def finish():
            time.sleep(1)
            self.dest.mkdir()
            (self.dest / ".done").write_text("x\n")

        t = threading.Thread(target=finish)
        t.start()
        proc = self.run_script(VAMIGA_URL="/nonexistent")
        t.join()
        self.assertEqual(proc.returncode, 0, proc.stderr)
        self.assertIn("while waiting", proc.stdout)
        self.assertTrue(lock.exists(), "the waiter must not remove the builder's lock")

    def test_a_dead_builders_lock_is_taken_over_and_released(self):
        lock = Path(f"{self.dest}.lock")
        lock.mkdir()
        dead = subprocess.Popen(["/usr/bin/true"])
        dead.wait()
        (lock / "pid").write_text(f"{dead.pid}\n")
        # The fetch then fails (no such repository): the script must say so and leave no lock or
        # half-built tree behind.
        proc = self.run_script(VAMIGA_URL=str(Path(self.cache, "no-such-repo")))
        self.assertIn("stale lock", proc.stdout)
        self.assertNotEqual(proc.returncode, 0)
        self.assertFalse(lock.exists())
        self.assertFalse(Path(f"{self.dest}.tmp").exists())
        self.assertFalse((self.dest / ".done").exists())


if __name__ == "__main__":
    unittest.main()
