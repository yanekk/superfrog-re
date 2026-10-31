"""The label check on this repo's real manifests (smoke-tests DESIGN §4), so that the full run catches a label
gap too. test_smoke_shell.py never touches the real repo; this is the one test that does."""
import os
import sys
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import smoke  # noqa: E402


class RealRepo(unittest.TestCase):
    def test_the_check_finds_no_problems(self):
        # gather_projects reads paths relative to the repo root and unittest may start anywhere.
        before = os.getcwd()
        os.chdir(os.path.dirname(HERE))
        self.addCleanup(os.chdir, before)
        self.assertEqual(smoke._problems(smoke.gather_projects()), [])


if __name__ == '__main__':
    unittest.main()
