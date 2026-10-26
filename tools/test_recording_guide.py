"""Tests for tools/recording_guide.py's pure part (no engine, no disks)."""
import os, sys, unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import recording_guide as G


def row(k, c, r, dying=0, lives=3):
    """A bug_trail line with the frog at cell (c, r)."""
    return f'{k} {c * 16 - 0x17} {r * 16 + 0x10} 0 0 {dying} {lives}'


class Guide(unittest.TestCase):
    def test_thinned_to_two_cells(self):
        lines, stop = G.guide([row(0, 10, 5), row(1, 11, 5), row(2, 12, 5), row(3, 12, 6)])
        self.assertEqual(lines, ['10,5  # step 0', '12,5  # step 2'])
        self.assertIsNone(stop)

    def test_dying_frames_are_skipped(self):
        lines, _ = G.guide([row(0, 10, 5), row(1, 20, 5, dying=1), row(2, 12, 5)])
        self.assertEqual([l.split()[0] for l in lines], ['10,5', '12,5'])

    def test_cut_at_the_first_life_lost(self):
        lines, stop = G.guide([row(0, 10, 5), row(1, 14, 5), row(2, 3, 3, lives=2), row(3, 30, 3, lives=2)])
        self.assertEqual(stop, 2)
        self.assertEqual([l.split()[0] for l in lines], ['10,5', '14,5'])


if __name__ == '__main__':
    unittest.main()
