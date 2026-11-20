"""generate.py with the real judge (plans/forest-dumb-level-1 T61): canned generators, built into
fd-judge by cargo (`FD_GEN`) and judged by `--cover` on the main program from the owner's disks. The
model is test_model's fake `claude` (T17): a test file never calls the network."""
import os
import sys
import tempfile
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import generate as g  # noqa: E402
import judge_run as jr  # noqa: E402
from test_model import FakeClaude  # noqa: E402

# T36's committed camera generators; `h0_513c` (the camera clamp) is the one judged.
with open(os.path.join(g.GEN_DIR, 'camera.rs'), encoding='utf-8') as f:
    CAMERA = f.read()
CLAMP_Y = '    s.w(h(0, CAMERA_Y), camera(g, MAX_Y));\n    s\n}'
assert CAMERA.endswith(CLAMP_Y + '\n')
# The camera's y always 5: the clamp's two y branches go one way only.
Y_FIXED = CAMERA[:-len(CLAMP_Y) - 1] + '    s.w(h(0, CAMERA_Y), 5);\n    s\n}\n'
# And a byte the clamp never reads.
WILD = CAMERA[:-len(CLAMP_Y) - 1] + '    s.w(h(0, CAMERA_Y), camera(g, MAX_Y));\n    s.b(h(0, 0x10), 1);\n    s\n}\n'


def answer(code):
    return f'```rust\n{code}```\n'


@unittest.skipUnless(os.path.exists(jr.LISTING), "needs the listing from the owner's disks (re_listing.py)")
class RealJudge(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.fake = FakeClaude(self)

    @property
    def sent(self):
        return [p.split('# Now answer this\n\n')[-1] for p in self.fake.prompts()]

    def test_writes_outside_the_reads_then_a_missed_branch_go_back_then_the_full_generator_is_covered(self):
        self.fake.answer(answer(WILD), answer(Y_FIXED), answer(CAMERA))
        res = g.generate('h0+$513c', 'h0_513c', out=self.tmp.name)
        self.assertEqual((res['result'], res['tries'], res['branches'], res['returns'], res['base']),
                         ('covered', 3, 4, 2000, False), res)
        self.assertIn('pokes $c077e8 (h0+$10)', self.sent[1])
        # The y clamp's two branches (it tests y first), each with its listing lines, not the x ones.
        self.assertIn('h0+$5144: every state took it', self.sent[2])
        self.assertIn('h0+$5156: every state took it', self.sent[2])
        self.assertIn('00005144  ', self.sent[2])
        self.assertNotIn('h0+$5168', self.sent[2])


if __name__ == '__main__':
    unittest.main()
