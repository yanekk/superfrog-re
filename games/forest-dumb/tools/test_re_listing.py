import os
import subprocess
import sys
import tempfile
import unittest

TOOL = os.path.join(os.path.dirname(os.path.abspath(__file__)), "re_listing.py")


class Command(unittest.TestCase):
    """The real program's listing is checked by fd-tools' exe_map_claims; this needs no disks."""

    def test_a_missing_program_says_how_to_get_it(self):
        with tempfile.TemporaryDirectory() as d:
            out = subprocess.run(
                [sys.executable, TOOL, "--exe", os.path.join(d, "nope"), "--out", d],
                capture_output=True,
                text=True,
            )
        self.assertNotEqual(out.returncode, 0)
        self.assertIn("fd-tools -- extract", out.stderr)


if __name__ == "__main__":
    unittest.main()
