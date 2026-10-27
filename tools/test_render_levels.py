"""Tests for tools/render_levels.py. Needs the unpacked chunks (tools/unpack.py, build/decomp)."""
import glob, os, sys, unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import render_levels as rl

HAVE = bool(glob.glob(os.path.join(rl.DECOMP, '*_level_*.bin')))

# Colour 1 of each world as the engine draws it (sf-conform examples/level_palette, the copper's
# palette mid-playfield): a colour the old neighbouring-chunk search got wrong for World 6.
ENGINE_COLOUR_1 = {'d1_c21': 0x35b, 'd1_c31': 0x8ac, 'd2_c12': 0x744,
                   'd2_c22': 0x997, 'd2_c32': 0x112, 'd4_c02': 0x134}


@unittest.skipUnless(HAVE, 'needs build/decomp (tools/unpack.py)')
class Palettes(unittest.TestCase):
    def setUp(self):
        self.assoc = {os.path.basename(f).split('_level')[0]: (f, pal)
                      for f, _, _, pal in rl.associations()}

    def test_every_level_uses_its_own_palette(self):
        for chunk, (f, pal) in self.assoc.items():
            with self.subTest(chunk=chunk):
                self.assertEqual(pal, rl.base_palette(open(f, 'rb').read()))

    def test_matches_the_engine_per_world(self):
        for chunk, want in ENGINE_COLOUR_1.items():
            with self.subTest(chunk=chunk):
                r, g, b = self.assoc[chunk][1][1]
                self.assertEqual((r // 17) << 8 | (g // 17) << 4 | b // 17, want)


if __name__ == '__main__':
    unittest.main()
