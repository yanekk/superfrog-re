"""Tests for tools/re_ghidra_import.py. Needs the player's disks; the import test needs Ghidra."""
import os, shutil, subprocess, sys, tempfile, unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import re_ghidra_import as G
import re_listing as R
from test_re_listing import Disks


class CompleteHunks(unittest.TestCase):
    def test_absent_hunks_become_empty_bss(self):
        for cid in ('d1_c03', 'd3_c04'):
            with self.subTest(cid):
                before = R.parse_exe(Disks.chunk(cid))
                after = R.parse_exe(G.complete_hunks(Disks.chunk(cid)))
                self.assertEqual([h.kind for h in after.hunks], ['CODE', 'BSS', 'BSS', 'BSS'])
                self.assertEqual([h.size for h in after.hunks], [h.size for h in before.hunks])
                self.assertEqual(after.relocs, before.relocs)

    def test_complete_executables_are_unchanged(self):
        data = Disks.chunk('d1_c01')
        self.assertEqual(G.complete_hunks(data), data)


@unittest.skipUnless(os.environ.get('GHIDRA_HOME') or shutil.which('ghidraRun'),
                     'Ghidra not installed (tools/setup/install-ghidra.sh)')
class Import(unittest.TestCase):
    def test_stage15_imports_with_symbols_outside_the_repo(self):
        with tempfile.TemporaryDirectory() as out:
            r = subprocess.run([sys.executable, os.path.join(R.ROOT, 'tools', 're_ghidra_import.py'),
                                'd1_stage15', '--out', out], cwd=R.ROOT, capture_output=True, text=True,
                               timeout=600)
            self.assertEqual(r.returncode, 0, r.stdout + r.stderr)
            with open(os.path.join(out, 'd1_stage15.c')) as f:
                text = f.read()
            self.assertIn('// atn_decompress @ h0+$2da', text)
            self.assertIn('// hunk_load @ h0+$43a', text)


if __name__ == '__main__':
    unittest.main()
