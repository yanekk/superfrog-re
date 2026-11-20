"""translate.py on the game (translator T18, its DESIGN §2.12): `char_sprite_position` passes on the
real program, and with the tree cached a second run takes under 2 minutes. The first run may export
and build from cold. Fails, never skips, without Ghidra, the disks or the extracted program (DESIGN
§2.14): the tool names the path it looked in. Full set."""
import json
import os
import subprocess
import sys
import time
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
TOOL = os.path.join(HERE, 'translate.py')
REPO = os.path.dirname(os.path.dirname(os.path.dirname(HERE)))
# A cold Ghidra import is bounded at 1 800 s (DESIGN §5.2); the warm run's bar is the task's.
COLD_SECONDS = 2700
WARM_SECONDS = 120


class OnTheGame(unittest.TestCase):
    def run_tool(self, timeout):
        p = subprocess.run([sys.executable, '-I', TOOL, 'char_sprite_position'], capture_output=True, text=True,
                           timeout=timeout)
        self.assertEqual(p.returncode, 0, p.stdout + p.stderr[-3000:])
        return json.loads(p.stdout.strip().splitlines()[-1]), p.stderr

    def test_char_sprite_position_passes_and_a_warm_run_is_under_two_minutes(self):
        self.run_tool(COLD_SECONDS)
        t0 = time.monotonic()
        line, err = self.run_tool(WARM_SECONDS)
        took = time.monotonic() - t0
        self.assertLess(took, WARM_SECONDS)
        self.assertIn('tree: current', err)
        self.assertEqual((line['routine'], line['name'], line['result'], line['states']),
                         ('h0+$5462', 'char_sprite_position', 'pass', 2000))
        self.assertEqual(line['rust'], 'games/forest-dumb/build/translate/h0-5462/translated.rs')
        with open(os.path.join(REPO, line['rust']), encoding='utf-8') as f:
            self.assertIn('pub fn h0_5462(', f.read())


if __name__ == '__main__':
    unittest.main()
