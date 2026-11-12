"""The whole-tree gate on the game (translator T16, its DESIGN §2.2, §2.3, §2.12): `translator_trial.py
--whole` exports the whole program, translates it, builds fd-judge optimised and judges every routine
with a registered generator on 2 000 states. Fails, never skips, without Ghidra, the disks or the
extracted program (DESIGN §2.14): the tool names the path it looked in. About 1 to 2 minutes warm, a
cold Ghidra import more; full set."""
import json
import os
import subprocess
import sys
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
TRIAL = os.path.join(HERE, 'translator_trial.py')
OUT = os.path.join(os.path.dirname(HERE), 'build', 'translator')
# Bounds the run: Ghidra's own import timeout is 1 800 s (DESIGN §5.2), the judging a few minutes.
SECONDS = 2700


class WholeTree(unittest.TestCase):
    def test_the_whole_tree_passes_with_no_hand_edit(self):
        p = subprocess.run([sys.executable, '-I', TRIAL, '--whole'], capture_output=True, text=True,
                           timeout=SECONDS)
        self.assertEqual(p.returncode, 0, p.stderr[-3000:])
        s = json.loads(p.stdout.strip().splitlines()[-1])
        with open(os.path.join(OUT, 'trial.json'), encoding='utf-8') as f:
            whole = json.load(f)['whole']
        # Stage 1 and 2: every instruction translated, every computed jump's targets found without a
        # hint; library calls are the machine's (DESIGN §2.8).
        self.assertEqual(s['unsupported_insns'], 0)
        self.assertEqual(s['unresolved_computed'], 0, whole['reading']['unresolved'])
        self.assertEqual(s['hints'], 0, whole['reading']['hints'])
        # Every judgeable routine passes on 2 000 states with no hint. 100 judgeable at T16: a drop
        # means a generator or the survey broke, not a pass.
        self.assertGreaterEqual(s['judgeable'], 100)
        with open(os.path.join(OUT, 'table.md'), encoding='utf-8') as f:
            table = f.read()
        self.assertEqual(s['pass'], s['judgeable'], table)
        self.assertEqual((s['untranslated'], s['write_into_code'], s['unsupported']), (0, 0, 0))
        self.assertTrue(s['gate'], whole['why'])


if __name__ == '__main__':
    unittest.main()
